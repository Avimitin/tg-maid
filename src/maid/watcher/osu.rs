use std::collections::hash_map::DefaultHasher;
use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};

use anyhow::Result;
use scraper::{Html, Selector};
use serde::Deserialize;
use teloxide::prelude::*;
use tokio::sync::Mutex;

const USER_API_ENDPOINT: &str = "https://osu.ppy.sh/api/get_user";

pub struct Settings {
    token: String,
    subscribe: Vec<Box<str>>,
    notifier: Vec<ChatId>,
}

impl Settings {
    pub fn new(token: String, subscribe: Vec<Box<str>>, notifier: Vec<ChatId>) -> Self {
        Self {
            token,
            subscribe,
            notifier,
        }
    }
}

struct Runtime<C: OsuEventCache> {
    client: reqwest::Client,
    config: Settings,
    bot: AutoSend<Bot>,
    cache: Arc<Mutex<C>>,
}

#[derive(Deserialize, Debug)]
struct Response {
    events: Vec<UserEvent>,
}

#[derive(Deserialize, Debug)]
struct UserEvent {
    display_html: String,
}

#[derive(Hash)]
struct UserEventHtmlExt {
    who: String,
    achieve: String,
    on: String,
    user_link: String,
    map_link: String,
}

impl UserEventHtmlExt {
    fn to_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }

    fn to_html(&self) -> String {
        format!(
            r#"<a href="{}">{}</a> {}<a href="{}">{}</a>"#,
            self.user_link, self.who, self.achieve, self.map_link, self.on
        )
    }
}

pub enum EventCacheStatus {
    Exist,
    None,
}

#[async_trait::async_trait]
pub trait OsuEventCache: Send + Sync {
    async fn store_osu_event_cache(&mut self, event_hash: u64) -> Result<EventCacheStatus>;
}

impl std::fmt::Display for UserEventHtmlExt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.who, self.achieve, self.on)
    }
}

impl UserEventHtmlExt {
    fn parse_from(s: &str) -> Result<Self> {
        let html = Html::parse_fragment(s);
        let selector = Selector::parse("*").unwrap();
        let text = html
            .select(&selector)
            .map(|element| element.text().collect::<Vec<_>>())
            .next();

        let link_selector = Selector::parse("a").unwrap();
        let links = html
            .select(&link_selector)
            .map(|elem| elem.value().attr("href").unwrap())
            .collect::<Vec<&str>>();

        let text = text.ok_or_else(|| anyhow::anyhow!("no event found"))?;
        if text.is_empty() {
            anyhow::bail!("no event found")
        }

        let length = text.len();

        let mut text = text.into_iter();
        let mut consumed = 1;
        let mut who = text.next().unwrap();
        if who.trim().is_empty() {
            who = text.next().unwrap();
            consumed += 1;
        }

        let achieve = text
            .clone()
            .take(length - 2 - consumed)
            .collect::<Vec<&str>>()
            .concat()
            .trim_start()
            .to_string();

        let on = format!(
            "{}{}",
            text.nth(length - 2 - consumed).unwrap(),
            text.next().unwrap()
        );

        const ENDPOINT: &str = "https://osu.ppy.sh";
        let user_link = format!("{ENDPOINT}{}", links[0]);
        let map_link = format!("{ENDPOINT}{}", links[1]);

        Ok(Self {
            who: who.to_string(),
            achieve,
            on,
            user_link,
            map_link,
        })
    }
}

async fn fetch_user_info(client: &reqwest::Client, token: &str, user: &str) -> Result<Response> {
    let mut response = client
        .get(USER_API_ENDPOINT)
        .query(&[("k", token), ("u", user)])
        .send()
        .await?
        .json::<Vec<Response>>()
        .await?;

    if response.is_empty() {
        anyhow::bail!("no user found")
    }

    Ok(response.remove(0))
}

#[tokio::test]
async fn test_fetch_user_info() {
    dotenv::dotenv().ok();
    let token = std::env::var("OSU_API_TOKEN").unwrap();

    let resp = fetch_user_info(&reqwest::Client::new(), token.as_str(), "lifeline")
        .await
        .unwrap();

    let event = resp.events.get(0).unwrap();
    let event = UserEventHtmlExt::parse_from(&event.display_html).unwrap();
    println!("{event}")
}

async fn watch_and_response<C: OsuEventCache>(rt: Arc<Runtime<C>>) {
    for user in &rt.config.subscribe {
        let response = fetch_user_info(&rt.client, &rt.config.token, user).await;
        match response {
            Ok(info) => {
                let event = info.events.get(0);
                if event.is_none() {
                    continue;
                }
                let event = event.unwrap();
                let event = UserEventHtmlExt::parse_from(&event.display_html);
                if event.is_err() {
                    continue;
                }
                let event = event.unwrap();

                {
                    let mut cache = rt.cache.lock().await;
                    let result = cache.store_osu_event_cache(event.to_hash()).await;
                    match result {
                        Ok(EventCacheStatus::Exist) => continue,
                        Err(e) => {
                            tracing::error!("Fail to cache osu event: {e}");
                            continue;
                        }
                        _ => (),
                    }
                } // early drop the mutex lock

                for chat in &rt.config.notifier {
                    let send_result = rt
                        .bot
                        .send_message(*chat, format!("New OSU Event:\n{}", event.to_html()))
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .disable_web_page_preview(true)
                        .await;
                    if let Err(e) = send_result {
                        tracing::error!("fail to send osu event to {chat}: {e}")
                    }
                }
            }
            Err(e) => {
                tracing::error!("fetching data from osu: {e}")
            }
        }
    }
}

pub fn spawn_watcher<C: OsuEventCache + 'static>(cfg: Settings, bot: AutoSend<Bot>, cache: C) {
    let (tx, rx) = tokio::sync::watch::channel(1);
    let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(60));
    let rt = Arc::new(Runtime {
        client: reqwest::Client::new(),
        config: cfg,
        bot,
        cache: Arc::new(Mutex::new(cache)),
    });

    tokio::spawn(async move {
        loop {
            let mut rx = rx.clone();
            let rt = Arc::clone(&rt);

            tokio::select! {
                _ = rx.changed() => {
                    break;
                }
                _ = heartbeat.tick() => {
                    watch_and_response(rt).await;
                }
            }
        }
    });

    tokio::spawn(super::notify_on_ctrl_c(tx));
}
