pub mod weibo {
    use anyhow::Result;
    use std::sync::Arc;
    use teloxide::{prelude::*, types::ChatId, Bot};

    pub struct Config {
        groups: Vec<ChatId>,
        limits: u8,
        period: std::time::Duration,
    }

    impl Config {
        /// Default with 10 limits and 30mins periods
        pub fn new() -> Self {
            Self {
                groups: Vec::new(),
                limits: 10,
                period: std::time::Duration::from_secs(1800),
            }
        }

        pub fn append_groups(mut self, groups: &[i64]) -> Self {
            let mut groups: Vec<ChatId> = groups.iter().map(|id| ChatId(*id)).collect();
            self.groups.append(&mut groups);
            self
        }

        pub fn period(self, period: std::time::Duration) -> Self {
            Self { period, ..self }
        }

        pub fn limit(self, limits: u8) -> Self {
            Self { limits, ..self }
        }
    }

    async fn watch() -> Result<()> {
        Ok(())
    }

    async fn handle_response(bot: AutoSend<Bot>, groups: Arc<Vec<ChatId>>, response: Result<()>) {
        match response {
            Ok(()) => {
                for group in groups.iter() {
                    if let Err(error) = bot.send_message(*group, "ye").await {
                        tracing::error!("fail to send data to {group}: {error}")
                    };
                }
            }
            Err(error) => {
                tracing::error!("Fail to fetch weibo top content: {error}")
            }
        }
    }

    async fn mainloop(
        bot: AutoSend<Bot>,
        mut heartbeat: tokio::time::Interval,
        signal: tokio::sync::watch::Receiver<u8>,
        groups: Arc<Vec<ChatId>>,
    ) {
        loop {
            let mut rx = signal.clone();
            let bot = bot.clone();
            tokio::select! {
                _ = heartbeat.tick() => {
                    let response = watch().await;
                    handle_response(bot, groups.clone(), response).await;
                }
                _ = rx.changed() => {
                    tracing::info!("Weibo Watcher is exiting");
                }
            }
        }
    }

    pub fn spawn(bot: AutoSend<Bot>, cfg: Config) -> tokio::sync::watch::Sender<u8> {
        let heartbeat = tokio::time::interval(cfg.period);
        let (tx, rx) = tokio::sync::watch::channel::<u8>(1);
        let groups = Arc::new(cfg.groups);

        tokio::task::spawn(mainloop(bot, heartbeat, rx, groups));

        tx
    }
}
