use crate::action::{
    Action, ActionEnum, ActionSignal, Props, StatefulAction, StatefulActionEnum, INFINITE, VISUAL,
};
use crate::config::Config;
use crate::error::Error;
use crate::io::IO;
use crate::resource::ResourceMap;
use crate::scheduler::processor::{AsyncSignal, SyncSignal};
use crate::signal::QWriter;
use crate::style::{style_ui, Style};
use crate::{error, style};
use eframe::egui;
use eframe::egui::CentralPanel;
use egui_extras::{Size, StripBuilder};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Counter {
    #[serde(default = "defaults::from")]
    from: u32,
    #[serde(default)]
    style: String,
}

stateful!(Counter { count: u32 });

mod defaults {
    #[inline(always)]
    pub fn from() -> u32 {
        3
    }
}

impl From<u32> for Counter {
    fn from(i: u32) -> Self {
        Self {
            from: i,
            style: "".to_owned(),
        }
    }
}

impl Action for Counter {
    #[inline(always)]
    fn resources(&self, _config: &Config) -> Vec<PathBuf> {
        vec![]
    }

    fn stateful(
        &self,
        io: &IO,
        res: &ResourceMap,
        config: &Config,
        sync_writer: &QWriter<SyncSignal>,
        async_writer: &QWriter<AsyncSignal>,
    ) -> Result<StatefulActionEnum, error::Error> {
        Ok(StatefulCounter {
            id: 0,
            done: self.from == 0,
            count: self.from,
            // style: Style::new("action-counter", &self.style),
        }
        .into())
    }
}

impl StatefulAction for StatefulCounter {
    impl_stateful!();

    #[inline(always)]
    fn props(&self) -> Props {
        VISUAL.into()
    }

    fn start(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), Error> {
        Ok(())
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), Error> {
        Ok(())
    }

    fn show(
        &mut self,
        ui: &mut egui::Ui,
        sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        enum Interaction {
            None,
            Decrement,
        };

        let mut interaction = Interaction::None;

        let button = egui::Button::new(format!("Click me {} more times", self.count));

        StripBuilder::new(ui)
            .size(Size::remainder())
            .size(Size::exact(420.0))
            .size(Size::remainder())
            .horizontal(|mut strip| {
                strip.empty();
                strip.strip(|builder| {
                    builder
                        .size(Size::remainder())
                        .size(Size::exact(80.0))
                        .size(Size::remainder())
                        .vertical(|mut strip| {
                            strip.empty();
                            strip.cell(|ui| {
                                ui.centered_and_justified(|ui| {
                                    style_ui(ui, Style::SelectButton);
                                    if ui.add(button).clicked() {
                                        interaction = Interaction::Decrement;
                                    }
                                });
                            });
                            strip.empty();
                        });
                });
                strip.empty();
            });

        match interaction {
            Interaction::None => {}
            Interaction::Decrement => {
                self.count = self.count.saturating_sub(1);
                if self.count == 0 {
                    self.done = true;
                    sync_writer.push(SyncSignal::UpdateGraph);
                }
            }
        }

        Ok(())
    }

    #[inline(always)]
    fn stop(
        &mut self,
        sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
    ) -> Result<(), error::Error> {
        self.done = true;
        Ok(())
    }

    fn debug(&self) -> Vec<(&str, String)> {
        <dyn StatefulAction>::debug(self)
            .into_iter()
            .chain([("count", format!("{:?}", self.count))])
            .collect()
    }
}
