use deepl::DeepLApi;
use rusty_maid::{
    app::{AppData, RuntimeData},
    cache::Cacher,
    helper,
    http::HttpClient,
    modules,
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
    let app_data = prepare_app_data().await;

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

async fn prepare_osu() -> rosu_v2::Osu {
    let client_id: u64 = helper::parse_from_env("OSU_CLIENT_ID");
    let client_secret = helper::env_get_var("OSU_CLIENT_SECRET");

    rosu_v2::Osu::new(client_id, client_secret)
        .await
        .unwrap_or_else(|err| panic!("fail to create osu client: {err}"))
}

async fn prepare_app_data() -> AppData {
    let data = RuntimeData::builder()
        .cacher(prepare_cache())
        .requester(HttpClient::new())
        .deepl(prepare_deepl())
        .osu(prepare_osu().await)
        .build();

    data.into()
}
