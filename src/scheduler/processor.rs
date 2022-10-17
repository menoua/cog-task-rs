use crate::action::{ActionEnum, ActionSignal, StatefulActionEnum};
use crate::config::{Config, LogCondition};
use crate::error;
use crate::error::Error::{FlowError, InternalError};
use crate::logger::{Logger, LoggerSignal};
use crate::scheduler::info::Info;
use crate::server::ServerSignal;
use crate::signal::{QReader, QWriter};
use crate::util::spin_sleeper;
use chrono::{DateTime, Local};
use eframe::egui;
use itertools::Itertools;
use petgraph::prelude::{EdgeRef, NodeIndex};
use petgraph::EdgeDirection;
use serde_json::Value;
use std::collections::HashSet;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub type Atomic = Arc<Mutex<StatefulActionEnum>>;

#[derive(Debug, Clone)]
pub enum AsyncSignal {
    Logger(DateTime<Local>, LoggerSignal),
    Finish,
}

pub struct AsyncProcessor {
    logger: Logger,
    async_reader: QReader<AsyncSignal>,
    async_writer: QWriter<AsyncSignal>,
    server_writer: QWriter<ServerSignal>,
}

#[derive(Debug, Clone)]
pub enum SyncSignal {
    UpdateGraph,
    KeyPress(HashSet<egui::Key>),
    Finish,
}

pub struct SyncProcessor {
    ctx: egui::Context,
    tree: Atomic,
    // key_monitors: HashSet<usize>,
    sync_reader: QReader<SyncSignal>,
    sync_writer: QWriter<SyncSignal>,
    async_writer: QWriter<AsyncSignal>,
    server_writer: QWriter<ServerSignal>,
}

impl AsyncProcessor {
    pub fn spawn(
        info: &Info,
        config: &Config,
        server_writer: &QWriter<ServerSignal>,
    ) -> Result<QWriter<AsyncSignal>, error::Error> {
        let async_reader = QReader::new();
        let async_writer = async_reader.writer();
        let mut proc = Self {
            logger: Logger::new(info, config)?,
            async_reader,
            async_writer,
            server_writer: server_writer.clone(),
        };

        let async_writer = proc.async_writer.clone();

        thread::spawn(move || {
            while let Some(signal) = proc.async_reader.pop() {
                match signal {
                    AsyncSignal::Logger(time, signal) => {
                        proc.logger
                            .update(time, signal, &proc.async_writer)
                            .unwrap();
                    }
                    AsyncSignal::Finish => break,
                }
            }

            let result = proc.logger.finish();
            proc.server_writer.push(ServerSignal::AsyncComplete(result));
        });

        Ok(async_writer)
    }
}

impl SyncProcessor {
    pub fn spawn(
        ctx: &egui::Context,
        tree: StatefulActionEnum,
        async_writer: &QWriter<AsyncSignal>,
        server_writer: &QWriter<ServerSignal>,
    ) -> Result<(QWriter<SyncSignal>, Arc<Mutex<StatefulActionEnum>>), error::Error> {
        let sync_reader = QReader::new();
        let sync_writer = sync_reader.writer();
        let mut proc = Self {
            ctx: ctx.clone(),
            tree: Arc::new(Mutex::new(tree)),
            // key_monitors: HashSet::new(),
            sync_reader,
            sync_writer,
            async_writer: async_writer.clone(),
            server_writer: server_writer.clone(),
        };

        let sync_writer = proc.sync_writer.clone();
        let atomic = proc.tree.clone();

        thread::spawn(move || {
            proc.init();

            while let Some(signal) = proc.sync_reader.pop() {
                match signal {
                    SyncSignal::UpdateGraph => proc.update_graph(),
                    SyncSignal::KeyPress(keys) => proc.update_keypress(keys),
                    SyncSignal::Finish => break,
                }
                .unwrap_or_else(|e| {
                    proc.server_writer.push(ServerSignal::BlockCrashed(e));
                });
            }

            // proc.tree.lock().unwrap().0.clear();
            proc.server_writer.push(ServerSignal::SyncComplete(Ok(())));
        });

        Ok((sync_writer, atomic))
    }

    fn init(&mut self) {
        let tree = &mut (*self.tree.lock().unwrap());

        self.async_writer.push(LoggerSignal::Append(
            "mainevent".to_owned(),
            ("start".to_owned(), Value::String("Success".to_owned())),
        ));

        // let time = Instant::now();
        // self.timers.extend(timers.into_iter().map(|(v, t)| {
        //     let mut sync_writer = self.sync_writer.clone();
        //     thread::spawn(move || {
        //         let offset = Instant::now() - time;
        //         spin_sleeper().sleep(t - offset);
        //         sync_writer.push(SyncSignal::UpdateGraph);
        //     });
        //     (v, Edge::Starter, time.add(t))
        // }));
    }

    fn update_graph(&mut self) -> Result<(), error::Error> {
        let mut tree = self.tree.lock().unwrap();
        // tree.inner_mut().update_tree();

        if tree.inner().is_over()? {
            self.async_writer.push(LoggerSignal::Append(
                "mainevent".to_owned(),
                ("finish".to_owned(), Value::String("Success".to_owned())),
            ));

            self.server_writer.push(ServerSignal::BlockFinished);
        }

        Ok(())
    }

    fn update_keypress(&mut self, keys: HashSet<egui::Key>) -> Result<(), error::Error> {
        // if !self.key_monitors.is_empty() {
        //     let (graph, nodes, _) = &mut (*self.tree.lock().unwrap());
        //
        //     for &i in self.key_monitors.iter() {
        //         if let Some(node) = graph.node_mut(nodes[i]) {
        //             node.action.update(
        //                 ActionSignal::KeyPress(keys.clone()),
        //                 &mut self.sync_writer,
        //                 &mut self.async_writer,
        //             )?;
        //         }
        //     }
        // }

        Ok(())
    }
}
