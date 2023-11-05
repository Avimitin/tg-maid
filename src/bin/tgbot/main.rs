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
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    run().await
}

async fn run() -> anyhow::Result<()> {
    let config = Config::from_path()?;

    let bot = teloxide::Bot::new(&config.bot_token);

    let handler = handlers::handler_schema();
    let dialogue_state = dialogue::InMemStorage::<handlers::DialogueStatus>::new();
    let app_data = prepare_app_data(&config).await;

    modules::health::spawn_healthcheck_listner(config.health_check_port);
    modules::bilibili::spawn_bilibili_live_room_listener(bot.clone(), app_data.clone(), &config);
    modules::osu::spawn_osu_user_event_watcher(bot.clone(), app_data.clone(), &config);

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

fn prepare_deepl(cfg: &Config) -> DeepLApi {
    DeepLApi::with(&cfg.deepl.api_key).new()
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

async fn prepare_osu(cfg: &Config) -> rosu_v2::Osu {
    rosu_v2::Osu::new(cfg.osu.client_id, &cfg.osu.client_secret)
        .await
        .unwrap_or_else(|err| panic!("fail to create osu client: {err}"))
}

async fn prepare_app_data(cfg: &Config) -> AppData {
    let data = RuntimeData::builder()
        .cacher(prepare_cache(cfg))
        .requester(HttpClient::new())
        .deepl(prepare_deepl(cfg))
        .osu(prepare_osu(cfg).await)
        .quote_maker(prepare_quote_maker())
        .url_cleaner(url_cleaner())
        .build();

    data.into()
}
