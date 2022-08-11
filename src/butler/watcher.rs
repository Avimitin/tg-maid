pub mod weibo {
    use anyhow::Result;
    use serde::Deserialize;
    use std::sync::Arc;
    use teloxide::{prelude::*, types::ChatId, Bot};

    #[derive(Deserialize)]
    struct Response {
        update_time: String,
        data: Vec<Data>,
    }

    impl Response {
        fn to_telegram_html(&self, limit: u8) -> String {
            self.data.iter().take(limit as usize).fold(
                format!("微博热搜:\n更新时间: {}", self.update_time),
                |sum, data| {
                    format!(
                        r#"{sum}
* <a href="{}">{}. {}</a>
  热度: {}"#,
                        data.url, data.index, data.title, data.hot
                    )
                },
            )
        }
    }

    #[derive(Deserialize)]
    struct Data {
        index: u8,
        title: String,
        hot: String,
        url: String,
    }

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

    async fn get_trending(client: &reqwest::Client) -> Result<Response> {
        Ok(client
            .get("https://api.vvhan.com/api/hotlist?type=wbHot")
            .send()
            .await?
            .json()
            .await?)
    }

    async fn handle_response(rt: Arc<Runtime>, response: Result<Response>) {
        match response {
            Ok(resp) => {
                let text = resp.to_telegram_html(rt.cfg.limits);
                for group in rt.cfg.groups.iter() {
                    if let Err(error) = rt
                        .bot
                        .send_message(*group, &text)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await
                    {
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
        rt: Arc<Runtime>,
        rx: tokio::sync::watch::Receiver<u8>,
        mut heartbeat: tokio::time::Interval,
    ) {
        loop {
            let mut rx = rx.clone();
            let rt = rt.clone();
            tokio::select! {
                _ = heartbeat.tick() => {
                    let response = get_trending(&rt.c).await;
                    handle_response(rt, response).await;
                }

                _ = rx.changed() => {
                    tracing::info!("Weibo Watcher is exiting");
                    break;
                }
            }
        }
    }

    struct Runtime {
        bot: AutoSend<Bot>,
        c: reqwest::Client,
        cfg: Config,
    }

    pub fn spawn(bot: AutoSend<Bot>, cfg: Config) {
        let heartbeat = tokio::time::interval(cfg.period);
        let (tx, rx) = tokio::sync::watch::channel::<u8>(1);

        let runtime = Arc::new(Runtime {
            bot,
            cfg,
            c: reqwest::Client::new(),
        });

        tokio::task::spawn(mainloop(runtime, rx, heartbeat));
        tokio::task::spawn(async move {
            tokio::signal::ctrl_c()
                .await
                .expect("Error occur when waiting for ctrl-c signal");
            tx.send(0).expect("fail to shutdown weibo listener");
        });
    }
}
