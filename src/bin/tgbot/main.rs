use anyhow::Context;
use clearurl::UrlCleaner;
use deepl::DeepLApi;
use rusty_maid::{
    app::{AppData, RuntimeData},
    cache::Cacher,
    config::Config,
    http::HttpClient,
    modules,
};
use teloxide::{dispatching::dialogue, dptree, prelude::Dispatcher};

mod handlers;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    run().await
}

async fn run() -> anyhow::Result<()> {
    use std::time::Duration;
    let config = Config::get_global_config();
    let bot = if let Some(proxy_url) = config.proxy.telegram() {
        // use teloxide default config
        let client = reqwest::Client::builder()
            .proxy(reqwest::Proxy::all(proxy_url)?)
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(17))
            .tcp_nodelay(true)
            .build()?;
        teloxide::Bot::with_client(&config.bot_token, client)
    } else {
        teloxide::Bot::new(&config.bot_token)
    };

    let handler = handlers::handler_schema();
    let dialogue_state = dialogue::InMemStorage::<handlers::DialogueStatus>::new();
    let app_data = prepare_app_data(&config).await;

    modules::health::spawn_healthcheck_listner(config.health_check_port);
    modules::bilibili::spawn_bilibili_live_room_listener(bot.clone(), app_data.clone(), &config);

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![app_data, dialogue_state])
        .enable_ctrlc_handler()
        .default_handler(|_| async move {})
        .build()
        .dispatch()
        .await;

    Ok(())
}

fn prepare_cache(cfg: &Config) -> Cacher {
    let client = redis::Client::open(cfg.redis_addr.as_str()).expect("fail to open client");
    Cacher::new(client)
}

pub fn prepare_deepl(cfg: &Config) -> DeepLApi {
    use std::time::Duration;
    let mut api_builder = DeepLApi::with(&cfg.deepl.api_key);
    if let Some(proxy_url) = cfg.proxy.deepl() {
        let client = reqwest::Client::builder()
            .proxy(reqwest::Proxy::all(proxy_url).expect("proxy url not available"))
            .timeout(Duration::from_secs(30))
            .build()
            .expect("request can not creat");
        api_builder.client(client);
    }
    api_builder.new()
}

fn prepare_quote_maker() -> make_quote::QuoteProducer<'static> {
    let bold = include_bytes!(env!("QUOTE_TEXT_FONT_PATH"));
    let light = include_bytes!(env!("QUOTE_USERNAME_FONT_PATH"));

    make_quote::QuoteProducer::builder()
        .font(bold, light)
        .build()
}

fn url_cleaner() -> UrlCleaner {
    let path = std::env::var("URL_CLEANER_RULE_FILE")
        .with_context(|| "url clearner rule file env not set")
        .unwrap();
    UrlCleaner::from_file(&path).unwrap()
}

async fn prepare_app_data(cfg: &Config) -> AppData {
    let data = RuntimeData::builder()
        .cacher(prepare_cache(cfg))
        .requester(HttpClient::new())
        .deepl(prepare_deepl(cfg))
        .quote_maker(prepare_quote_maker())
        .url_cleaner(url_cleaner())
        .build();

    data.into()
}
