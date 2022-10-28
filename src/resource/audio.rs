use crate::server::Config;
use eyre::{eyre, Context, Result};
use rodio::buffer::SamplesBuffer;
use rodio::source::Buffered;
use rodio::{Decoder, Source};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub type AudioBuffer = Buffered<SamplesBuffer<i16>>;

pub fn audio_from_file(path: &Path, _config: &Config) -> Result<AudioBuffer> {
    let decoder = Decoder::new(BufReader::new(
        File::open(&path).wrap_err_with(|| format!("Failed to open audio file: {path:?}"))?,
    ))
    .wrap_err_with(|| format!("Failed to decode audio file: {path:?}"))?;

    let sample_rate = decoder.sample_rate();
    let in_channels = decoder.channels() as i16;
    let out_channels = in_channels;

    let mut c = -1;
    let mut samples = vec![];
    for s in decoder {
        c = (c + 1) % in_channels;
        samples.push(s);
    }

    Ok(SamplesBuffer::new(out_channels as u16, sample_rate, samples).buffered())
}

pub fn interlace_channels(src1: AudioBuffer, mut src2: AudioBuffer) -> Result<AudioBuffer> {
    let sample_rate = src1.sample_rate();
    let in_channels = src1.channels() as i16;
    let out_channels = in_channels + 1;

    if src2.sample_rate() != sample_rate {
        return Err(eyre!(
            "Trigger (?) has different sampling rate than corresponding audio"
        ));
    }
    if src2.channels() != 1 {
        return Err(eyre!("Trigger (?) should have exactly 1 channel"));
    }

    let mut c = -1;
    let mut samples = vec![];
    for s in src1 {
        c = (c + 1) % in_channels;
        samples.push(s);
        if c == in_channels - 1 {
            if let Some(s) = src2.next() {
                samples.push(s);
            }
        }
    }
    if src2.next().is_some() {
        return Err(eyre!("Trigger for (?) is longer than itself."));
    }

    Ok(SamplesBuffer::new(out_channels as u16, sample_rate, samples).buffered())
}

pub fn drop_channel(src: AudioBuffer) -> Result<AudioBuffer> {
    let sample_rate = src.sample_rate();
    let in_channels = src.channels() as i16;
    let out_channels = in_channels - 1;
    if out_channels == 0 {
        return Err(eyre!(
            "Audio with internal trigger should have at least one channel."
        ));
    }

    let mut c = -1;
    let mut samples = vec![];
    for s in src {
        c = (c + 1) % in_channels;
        if c < in_channels - 1 {
            samples.push(s);
        }
    }

    Ok(SamplesBuffer::new(out_channels as u16, sample_rate, samples).buffered())
}

#[derive(Copy, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Volume {
    Inherit,
    Value(f32),
}

impl Default for Volume {
    #[inline(always)]
    fn default() -> Self {
        Volume::Inherit
    }
}

impl Debug for Volume {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Volume::Value(vol) = self {
            write!(f, "{vol}")
        } else {
            write!(f, "inherit")
        }
    }
}

impl Volume {
    pub fn and(&self, other: &Self) -> Self {
        match (self, other) {
            (&Self::Value(x), &Self::Value(y)) => Self::Value(x * y),
            (Self::Value(_), _) => *self,
            (Self::Inherit, _) => *other,
        }
    }

    pub fn or(&self, other: &Self) -> Self {
        if let Self::Inherit = self {
            *other
        } else {
            *self
        }
    }

    pub fn value(&self) -> f32 {
        if let &Self::Value(x) = self {
            x
        } else {
            1.0
        }
    }
}

impl From<f32> for Volume {
    fn from(v: f32) -> Self {
        Self::Value(v.max(0.0))
    }
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UseTrigger {
    Inherit,
    Yes,
    No,
}

impl Default for UseTrigger {
    #[inline(always)]
    fn default() -> Self {
        UseTrigger::Inherit
    }
}

impl UseTrigger {
    pub fn or(&self, other: &Self) -> Self {
        if let Self::Inherit = self {
            *other
        } else {
            *self
        }
    }

    pub fn value(&self) -> bool {
        !matches!(self, UseTrigger::No)
    }
}

#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimePrecision {
    Inherit,
    RespectIntervals,
    RespectBoundaries,
}

impl Default for TimePrecision {
    #[inline(always)]
    fn default() -> Self {
        TimePrecision::Inherit
    }
}

impl TimePrecision {
    pub fn or(&self, other: &Self) -> Self {
        if let Self::Inherit = self {
            *other
        } else {
            *self
        }
    }
}