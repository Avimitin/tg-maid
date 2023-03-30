use std::{
    fmt::Display,
    future::Future,
    ops::{Deref, DerefMut},
    sync::Arc,
    time::Duration,
};

use tokio::sync::watch;
use typed_builder::TypedBuilder;

use crate::app::AppData;

#[derive(Debug, Default, Clone, Copy)]
pub struct State<S>(pub S);

impl<S> Deref for State<S> {
    type Target = S;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> DerefMut for State<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(TypedBuilder)]
pub struct EventWatcher<S> {
    pub bot: teloxide::Bot,
    pub data: AppData,
    #[builder(setter( transform = |s: S| Arc::new(State(s)) ))]
    pub state: Arc<State<S>>,
}

impl<S> Clone for EventWatcher<S> {
    fn clone(&self) -> Self {
        // bot & data is already wrapped by Arc
        Self {
            bot: self.bot.clone(),
            data: self.data.clone(),
            state: Arc::clone(&self.state),
        }
    }
}

pub trait Promise: Future<Output = anyhow::Result<()>> + Send + 'static {}
impl<T> Promise for T where T: Future<Output = anyhow::Result<()>> + Send + 'static {}

impl<S> EventWatcher<S> {
    pub fn start<P, T>(self, name: impl Display, interval_secs: u64, task: T)
    where
        P: Promise,
        S: Send + Sync + 'static,
        T: Fn(EventWatcher<S>) -> P + Sync + Send + 'static,
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
                        if let Err(err) = task(watcher).await {
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
