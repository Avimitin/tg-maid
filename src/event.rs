use std::{fmt::Display, future::Future, sync::Arc, time::Duration};

use serde_json::{Map, Value};
use tokio::sync::{watch, Mutex};
use typed_builder::TypedBuilder;

use crate::app::AppData;

#[derive(TypedBuilder)]
pub struct EventWatcher {
    pub bot: teloxide::Bot,
    pub data: AppData,
    pub state: Arc<Mutex<Map<String, Value>>>,
}

impl Clone for EventWatcher {
    fn clone(&self) -> Self {
        // bot & data is already wrapped by Arc
        Self {
            bot: self.bot.clone(),
            data: self.data.clone(),
            state: Arc::clone(&self.state),
        }
    }
}

pub trait Promise: Future<Output = anyhow::Result<()>> + Send + Sync {}
impl<T> Promise for T where T: Future<Output = anyhow::Result<()>> + Send + Sync {}

pub trait Task<E: Promise>: Fn(EventWatcher) -> E + Send + Sync + 'static {}

impl EventWatcher {
    pub async fn start<P, T>(self, task: T, name: impl Display, interval_secs: u64)
    where
        P: Promise,
        T: Task<P>,
    {
        let (tx, rx) = watch::channel(1_u8);
        let mut heartbeat = tokio::time::interval(Duration::from_secs(interval_secs));

        tokio::spawn(async move {
            loop {
                let watcher = self.clone();
                let mut rx = rx.clone();

                tokio::select! {
                    _ = rx.changed() => {
                        break;
                    }
                    _ = heartbeat.tick() => {
                        let result = task(watcher).await;
                        if let Err(err) = result {
                            tracing::error!("{}", err)
                        }
                    }
                }
            }
        });

        let name = name.to_string();

        let quit_on_ctrl_c = || async move {
            tokio::signal::ctrl_c().await.ok();
            tracing::info!("Quiting event watcher for {}...", name);
            tx.send(0)
                .unwrap_or_else(|_| panic!("fail to send signal into event watcher {}", name));
        };

        tokio::spawn(quit_on_ctrl_c());
    }
}
