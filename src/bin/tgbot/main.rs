use deepl::DeepLApi;
use rusty_maid::{
    app::{AppData, RuntimeData},
    modules::{self, cache::Cacher, http::HttpClient},
};
use teloxide::{dispatching::dialogue, dptree, prelude::Dispatcher};

mod handlers;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    run().await
}

async fn run() {
    let bot = teloxide::Bot::from_env();

    let handler = handlers::handler_schema();
    let dialogue_state = dialogue::InMemStorage::<handlers::DialogueStatus>::new();
    let app_data = prepare_app_data();

    modules::health::spawn_healthcheck_listner();
    modules::bilibili::spawn_bilibili_live_room_listener(bot.clone(), app_data.clone());

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![app_data, dialogue_state])
        .enable_ctrlc_handler()
        .default_handler(|_| async move {})
        .build()
        .dispatch()
        .await;
}

fn prepare_cache() -> Cacher {
    let redis_addr = std::env::var("REDIS_ADDR")
        .expect("fail to get redis addr, please check environment variable `REDIS_ADDR`.");
    let client = redis::Client::open(redis_addr).expect("fail to open client");
    Cacher::new(client)
}

fn prepare_deepl() -> DeepLApi {
    let authkey =
        std::env::var("DEEPL_API_KEY").unwrap_or_else(|_| panic!("no deepl auth key found"));
    DeepLApi::with(&authkey).new()
}

fn prepare_app_data() -> AppData {
    let data = RuntimeData::builder()
        .cacher(prepare_cache())
        .requester(HttpClient::new())
        .deepl(prepare_deepl())
        .build();

    data.into()
}
