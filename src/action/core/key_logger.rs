use crate::action::{Action, ActionSignal, Props, StatefulAction, INFINITE};
use crate::comm::QWriter;
use crate::resource::ResourceMap;
use crate::server::{AsyncSignal, Config, LoggerSignal, State, SyncSignal, IO};
use chrono::Local;
use eframe::egui::Ui;
use eyre::{eyre, Error, Result};
use serde::{Deserialize, Serialize};
use serde_cbor::Value;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KeyLogger(#[serde(default = "defaults::group")] String);

stateful!(KeyLogger { group: String });

mod defaults {
    #[inline(always)]
    pub fn group() -> String {
        "keypress".to_owned()
    }
}

impl Action for KeyLogger {
    fn stateful(
        &self,
        _io: &IO,
        _res: &ResourceMap,
        _config: &Config,
        _sync_writer: &QWriter<SyncSignal>,
        _async_writer: &QWriter<AsyncSignal>,
    ) -> Result<Box<dyn StatefulAction>> {
        if self.0.is_empty() {
            return Err(eyre!("KeyLogger `group` cannot be an empty string"));
        }

        Ok(Box::new(StatefulKeyLogger {
            done: false,
            group: self.0.clone(),
        }))
    }
}

impl StatefulAction for StatefulKeyLogger {
    impl_stateful!();

    #[inline(always)]
    fn props(&self) -> Props {
        INFINITE.into()
    }

    fn start(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<(), Error> {
        async_writer.push(LoggerSignal::Append(
            self.group.clone(),
            ("event".to_owned(), Value::Text("start".to_owned())),
        ));
        Ok(())
    }

    fn update(
        &mut self,
        signal: &ActionSignal,
        _sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<Vec<SyncSignal>> {
        if let ActionSignal::KeyPress(_, keys) = signal {
            let group = self.group.clone();
            let entry = (
                "key".to_string(),
                Value::Array(keys.iter().map(|k| Value::Text(format!("{k:?}"))).collect()),
            );

            async_writer.push(AsyncSignal::Logger(
                Local::now(),
                LoggerSignal::Append(group, entry),
            ));
        }

        Ok(vec![])
    }

    fn show(
        &mut self,
        _ui: &mut Ui,
        _sync_writer: &mut QWriter<SyncSignal>,
        _async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<(), Error> {
        Ok(())
    }

    #[inline]
    fn stop(
        &mut self,
        _sync_writer: &mut QWriter<SyncSignal>,
        async_writer: &mut QWriter<AsyncSignal>,
        _state: &State,
    ) -> Result<()> {
        async_writer.push(LoggerSignal::Append(
            self.group.clone(),
            ("event".to_owned(), Value::Text("stop".to_owned())),
        ));

        self.done = true;
        Ok(())
    }

    fn debug(&self) -> Vec<(&str, String)> {
        <dyn StatefulAction>::debug(self)
            .into_iter()
            .chain([("group", format!("{:?}", self.group))])
            .collect()
    }
}
