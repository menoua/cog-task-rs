use crate::resource::{AudioChannel, FrameBuffer, MediaStream, StreamMode};
use crate::server::Config;
use eframe::egui::mutex::RwLock;
use eframe::egui::{ColorImage, ImageData, TextureId, TextureOptions, Vec2};
use eframe::epaint::TextureManager;
use eyre::{eyre, Context, Result};
use gst::prelude::*;
use gstreamer as gst;
use gstreamer_app as gst_app;
use num_rational::Rational32;
use num_traits::ToPrimitive;
use once_cell::sync::OnceCell;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{env, thread};
use thiserror::Error;

static GST_INIT: OnceCell<()> = OnceCell::new();

/// A video handle that uses GStreamer to stream video content.
/// This `struct` and its associated `impl` is a simplified version of the
/// `VideoPlayer` struct found at: https://github.com/jazzfool/iced_video_player.
#[derive(Clone)]
pub struct Stream {
    path: PathBuf,
    source: gst::Bin,
    playbin: gst::Bin,
    bus: gst::Bus,
    frame_size: [u32; 2],
    frame_rate: f64,
    audio_chan: u16,
    audio_rate: u32,
    duration: Duration,
    is_eos: bool,
    paused: bool,
    tex_manager: Arc<RwLock<TextureManager>>,
}

impl MediaStream for Stream {
    /// Create a new video object from a given video file.
    fn new(
        tex_manager: Arc<RwLock<TextureManager>>,
        path: &Path,
        _config: &Config,
    ) -> Result<Self> {
        init()?;

        let (source, playbin) = launch(path, &StreamMode::Query, 1.0)?;
        let bus = source.bus().unwrap();

        let video_sink = get_video_sink(&playbin, false);
        let (width, height, frame_rate) = match video_sink.as_ref() {
            Some(sink) => video_meta_from_sink(sink)?,
            None => (0, 0, 0.0),
        };

        let audio_sink = get_audio_sink(&playbin, false);
        let (audio_chan, audio_rate) = match audio_sink.as_ref() {
            Some(sink) => audio_meta_from_sink(sink)?,
            None => (0, 0),
        };
        println!("--> width={width} height={height} framerate={frame_rate} audio_chan={audio_chan} audio_sr={audio_rate}");

        let duration = Duration::from_nanos(
            source
                .query_duration::<gst::ClockTime>()
                .ok_or(Error::Duration)
                .unwrap()
                .nseconds(),
        );

        source
            .set_state(gst::State::Null)
            .wrap_err_with(|| eyre!("Failed to close video graciously ({path:?})"))?;

        Ok(Stream {
            path: path.to_owned(),
            source,
            playbin,
            bus,
            frame_size: [width, height],
            frame_rate,
            audio_chan,
            audio_rate,
            duration,
            is_eos: false,
            paused: true,
            tex_manager,
        })
    }

    fn cloned(
        &self,
        frame: Arc<Mutex<Option<(TextureId, Vec2)>>>,
        media_mode: StreamMode,
        volume: f32,
    ) -> Result<Self> {
        let (source, playbin) = launch(&self.path, &media_mode, volume)?;
        let bus = source.bus().unwrap();

        let video_sink = get_video_sink(&playbin, true);
        if let Some(sink) = video_sink {
            sink.set_max_buffers(5 * self.frame_rate.ceil() as u32);
            let path = self.path.clone();
            let [width, height] = self.size();
            let tex_manager = self.tex_manager.clone();

            thread::spawn(move || {
                sink.set_callbacks(
                    gst_app::AppSinkCallbacks::builder()
                        .new_sample(move |sink| {
                            let sample = sink.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                            let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
                            let map = buffer.map_readable().map_err(|_| gst::FlowError::Error)?;

                            *frame.lock().map_err(|_| gst::FlowError::Error)? = Some((
                                tex_manager.write().alloc(
                                    format!("{path:?}:@:[current]"),
                                    ImageData::Color(ColorImage::from_rgba_unmultiplied(
                                        [width as _, height as _],
                                        map.as_slice(),
                                    )),
                                    TextureOptions::LINEAR,
                                ),
                                Vec2::new(width as _, height as _),
                            ));

                            Ok(gst::FlowSuccess::Ok)
                        })
                        .build(),
                );
            });
        }

        Ok(Stream {
            path: self.path.clone(),
            source,
            playbin,
            bus,
            frame_size: self.frame_size,
            frame_rate: self.frame_rate,
            audio_chan: self.audio_chan,
            audio_rate: self.audio_rate,
            duration: self.duration,
            is_eos: self.is_eos,
            paused: self.paused,
            tex_manager: self.tex_manager.clone(),
        })
    }

    /// Get if the stream ended or not.
    #[inline(always)]
    fn eos(&self) -> bool {
        self.is_eos
    }

    /// Get the size/resolution of the video as `[width, height]`.
    #[inline(always)]
    fn size(&self) -> [u32; 2] {
        self.frame_size
    }

    /// Get the framerate of the video as frames per second.
    #[inline(always)]
    fn framerate(&self) -> f64 {
        self.frame_rate
    }

    /// Get the number of audio channels.
    #[inline(always)]
    fn channels(&self) -> u16 {
        self.audio_chan
    }

    /// Get the media duration.
    #[inline(always)]
    fn duration(&self) -> Duration {
        self.duration
    }

    /// Check if stream has a video channel.
    #[inline(always)]
    fn has_video(&self) -> bool {
        self.frame_size.iter().sum::<u32>() > 0
    }

    /// Check if stream has an audio channel.
    #[inline(always)]
    fn has_audio(&self) -> bool {
        self.audio_chan > 0
    }

    /// Starts a stream; assumes it is at first frame and unpauses.
    fn start(&mut self) -> Result<()> {
        self.set_paused(false)?;
        Ok(())
    }

    /// Restarts a stream; seeks to the first frame and unpauses, sets the `eos` flag to false.
    fn restart(&mut self) -> Result<()> {
        self.is_eos = false;
        let position: gst::GenericFormattedValue = gst::format::Default::from_u64(0).into();
        self.source
            .seek_simple(gst::SeekFlags::FLUSH, position)
            .wrap_err("Failed to seek video position.")?;
        self.set_paused(false)?;
        Ok(())
    }

    fn pause(&mut self) -> Result<()> {
        self.set_paused(true)
    }

    fn pull_samples(&self) -> Result<(FrameBuffer, f64)> {
        let (source, playbin) = launch(&self.path, &StreamMode::Query, 1.0)?;

        let video_sink = get_video_sink(&playbin, false);
        if let Some(sink) = video_sink.as_ref() {
            sink.set_max_lateness(0);
            sink.set_max_buffers(5 * self.frame_rate.ceil() as u32);
        }

        let audio_sink = get_audio_sink(&playbin, false);
        if let Some(sink) = audio_sink.as_ref() {
            sink.set_max_buffers(5 * self.audio_rate * 2);
        }

        playbin.set_property("mute", true);
        source
            .set_state(gst::State::Playing)
            .wrap_err_with(|| format!("Failed to change video state (\"{:?}\")", self.path))?;

        let video_sink =
            video_sink.ok_or_else(|| eyre!("Tried to pull on non-existent video sink."))?;

        let mut frames = vec![];
        let t1 = Instant::now();
        while let Ok(sample) = video_sink.pull_sample() {
            let buffer = sample
                .buffer()
                .ok_or_else(|| eyre!("Failed to obtain buffer on video sample: {:?}", self.path))?;
            let map = buffer.map_readable().wrap_err_with(|| {
                format!("Failed to obtain map on buffered sample: {:?}", self.path)
            })?;

            frames.push((
                self.tex_manager.write().alloc(
                    format!("{:?}:@:{}", self.path, frames.len()),
                    ImageData::Color(ColorImage::from_rgba_unmultiplied(
                        [self.frame_size[0] as _, self.frame_size[1] as _],
                        map.as_slice(),
                    )),
                    TextureOptions::LINEAR,
                ),
                Vec2::new(self.frame_size[0] as _, self.frame_size[1] as _),
            ));
        }
        println!("Took {:?} to pull samples for video.", Instant::now() - t1);

        Ok((Arc::new(frames), self.frame_rate))
    }

    /// Process stream bus to see if stream has ended
    fn process_bus(&mut self, looping: bool) -> Result<bool> {
        let mut eos = false;
        for msg in self.bus.iter() {
            match msg.view() {
                gst::MessageView::Error(e) => {
                    Err(eyre!("Encountered error in gstreamer bus:\n{e:#?}"))?
                }
                gst::MessageView::Eos(_eos) => eos = true,
                _ => {}
            }
        }

        if eos && looping {
            self.restart()?;
            Ok(false)
        } else if eos {
            self.is_eos = true;
            self.set_paused(true)?;
            Ok(true)
        } else {
            Ok(self.is_eos)
        }
    }
}

impl Stream {
    /// Set the volume multiplier of the audio.
    /// `0.0` = 0% volume, `1.0` = 100% volume.
    pub fn set_volume(&mut self, volume: f64) {
        self.playbin.set_property("volume", &volume);
    }

    /// Set if the audio is muted or not, without changing the volume.
    pub fn set_muted(&mut self, muted: bool) {
        self.playbin.set_property("mute", &muted);
    }

    /// Get if the stream is paused.
    #[inline(always)]
    pub fn paused(&self) -> bool {
        self.paused
    }

    /// Set if the media is paused or not.
    pub fn set_paused(&mut self, paused: bool) -> Result<()> {
        self.source
            .set_state(if paused {
                gst::State::Paused
            } else {
                gst::State::Playing
            })
            .wrap_err("Failed to change video state.")?;
        self.paused = paused;
        Ok(())
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        self.source
            .set_state(gst::State::Null)
            .expect("Failed to drop video handle.");
    }
}

pub fn init() -> Result<()> {
    if GST_INIT.get().is_some() {
        return Ok(());
    }

    let plugin_env = "GST_PLUGIN_PATH";
    if env::var(plugin_env).is_err() {
        let mut list = vec![];

        if let Ok(home) = env::var("HOME") {
            let path = format!("{home}/.gstreamer-0.10");
            if Path::new(&path).exists() {
                list.push(path);
            }
        }

        let path = "/usr/local/lib/gstreamer-1.0";
        if Path::new(path).exists() {
            list.push(path.to_owned());
        }

        env::set_var(plugin_env, list.join(":"));
    }

    gst::init()
        .map(|r| {
            GST_INIT.set(()).expect("Tried to init GStreamer twice");
            r
        })
        .wrap_err("Failed to initialize GStreamer: required because there is a video element in this block.")
}

fn pipeline(path: &Path, mode: &StreamMode) -> Result<String> {
    let mut pipeline = format!(
        "\
        playbin uri=\"file://{}\" name=playbin \
        video-sink=\"videoconvert ! videoscale ! appsink name=video_sink caps=video/x-raw,format=RGBA,pixel-aspect-ratio=1/1\"",
        path.canonicalize()
            .wrap_err_with(|| format!(
                "Failed to canonicalize resource path: {path:?}"
            ))?
            .to_str()
            .unwrap()
    );

    match mode {
        StreamMode::Query => pipeline.push_str(
            " \
            audio-sink=\"audioconvert ! appsink name=audio_sink caps=audio/x-raw,format=S16LE,layout=interleaved\""
        ),
        StreamMode::Normal(AudioChannel::Stereo) => {},
        StreamMode::Normal(AudioChannel::Left) => pipeline.push_str(
            " \
            audio-sink=\"audioconvert ! audiopanorama panorama=-1 ! playsink\""
        ),
        StreamMode::Normal(AudioChannel::Right) => pipeline.push_str(
            " \
            audio-sink=\"audioconvert ! audiopanorama panorama=1 ! playsink\""
        ),
        StreamMode::Muted => pipeline.push_str(
            " \
            audio-sink=\"audioconvert ! fakesink\""
        ),
    };

    Ok(pipeline)
}

fn launch(path: &Path, mode: &StreamMode, volume: f32) -> Result<(gst::Bin, gst::Bin)> {
    let source = gst::parse_launch(&pipeline(path, mode)?)
        .wrap_err_with(|| format!("Failed to parse gstreamer command for video: {path:?}"))?
        .downcast::<gst::Bin>()
        .unwrap();

    let playbin = source.clone();

    playbin.set_property("volume", volume as f64);

    source
        .set_state(gst::State::Paused)
        .wrap_err_with(|| format!("Failed to change state for video ({path:?})."))?;
    source
        .state(gst::ClockTime::from_seconds(5))
        .0
        .wrap_err_with(|| format!("Failed to read state for video ({path:?})."))?;

    Ok((source, playbin))
}

fn get_video_sink(source: &gst::Bin, sync: bool) -> Option<gst_app::AppSink> {
    let video_sink: gst::Element = source.property("video-sink");
    let pad = video_sink.pads().get(0).cloned().unwrap();
    let pad = pad.dynamic_cast::<gst::GhostPad>().unwrap();
    let bin = pad.parent_element().unwrap();
    let bin = bin.downcast::<gst::Bin>().unwrap();

    let app_sink = bin.by_name("video_sink").unwrap();
    let app_sink = app_sink.downcast::<gst_app::AppSink>().unwrap();
    app_sink.set_async(true);
    app_sink.set_sync(sync);
    app_sink.set_max_lateness(0);
    app_sink.set_max_buffers(10);

    let timeout = gst::ClockTime::from_seconds(5);
    if app_sink.try_pull_preroll(Some(timeout)).is_some() {
        Some(app_sink)
    } else {
        None
    }
}

fn get_audio_sink(source: &gst::Bin, sync: bool) -> Option<gst_app::AppSink> {
    let audio_sink: gst::Element = source.property("audio-sink");
    let pad = audio_sink.pads().get(0).cloned().unwrap();
    let pad = pad.dynamic_cast::<gst::GhostPad>().unwrap();
    let bin = pad.parent_element().unwrap();
    let bin = bin.downcast::<gst::Bin>().unwrap();

    let app_sink = bin.by_name("audio_sink").unwrap();
    let app_sink = app_sink.downcast::<gst_app::AppSink>().unwrap();
    app_sink.set_async(true);
    app_sink.set_sync(sync);
    app_sink.set_max_lateness(0);
    app_sink.set_max_buffers(10);

    let timeout = gst::ClockTime::from_seconds(5);
    if app_sink.try_pull_preroll(Some(timeout)).is_some() {
        Some(app_sink)
    } else {
        None
    }
}

fn video_meta_from_sink(video_sink: &gst_app::AppSink) -> Result<(u32, u32, f64)> {
    let pad = video_sink.static_pad("sink").ok_or(Error::Caps)?;
    let caps = pad.current_caps().ok_or(Error::Caps)?;
    let caps = caps.structure(0).ok_or(Error::Caps)?;
    let width = caps.get::<i32>("width").map_err(|_| Error::Caps)? as u32;
    let height = caps.get::<i32>("height").map_err(|_| Error::Caps)? as u32;
    let video_rate = caps
        .get::<gst::Fraction>("framerate")
        .map_err(|_| Error::Caps)?;
    let video_rate = Rational32::new(video_rate.numer() as _, video_rate.denom() as _)
        .to_f64()
        .unwrap();
    Ok((width, height, video_rate))
}

fn audio_meta_from_sink(audio_sink: &gst_app::AppSink) -> Result<(u16, u32)> {
    let pad = audio_sink.static_pad("sink").ok_or(Error::Caps)?;
    let caps = pad.current_caps().ok_or(Error::Caps)?;
    let caps = caps.structure(0).ok_or(Error::Caps)?;
    let channels = caps.get::<i32>("channels").map_err(|_| Error::Caps)? as u16;
    let audio_rate = caps.get::<i32>("rate").map_err(|_| Error::Caps)? as u32;
    Ok((channels, audio_rate))
}

#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Glib(#[from] glib::Error),
    #[error("{0}")]
    Bool(#[from] glib::BoolError),
    #[error("failed to get the gstreamer bus")]
    Bus,
    #[error("{0}")]
    StateChange(#[from] gst::StateChangeError),
    #[error("failed to cast gstreamer element")]
    Cast,
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("failed to get media capabilities")]
    Caps,
    #[error("failed to query media duration or position")]
    Duration,
    #[error("failed to sync with playback")]
    Sync,
}
