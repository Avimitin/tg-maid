mod cache;
mod maid;
mod modules;

use teloxide::{dispatching::dialogue, prelude::*};

async fn connect_redis(addr: &str) -> redis::aio::ConnectionManager {
    let client = redis::Client::open(addr).expect("fail to open connection to redis");
    client
        .get_tokio_connection_manager()
        .await
        .expect("fail to connect to redis")
}

#[cfg(feature = "weibo")]
fn setup_weibo_watcher(bot: AutoSend<Bot>) {
    let allow_weibo_watcher_groups = std::env::var("WEIBO_NOTIFY_GROUPS")
        .unwrap_or_else(|_| panic!("no notify group specify"))
        .split(',')
        .map(|gid| {
            gid.parse::<i64>()
                .unwrap_or_else(|_| panic!("invalid gid: {gid}"))
        })
        .collect::<Vec<i64>>();

    let weibo_listen_config = maid::watcher::weibo::Config::new()
        .limit(10)
        .period(std::time::Duration::from_secs(21600))
        .append_groups(&allow_weibo_watcher_groups);

    maid::watcher::weibo::spawn(bot, weibo_listen_config);
}

#[cfg(feature = "osu")]
async fn setup_osu_watcher(bot: AutoSend<Bot>, redis_addr: &str) {
    let register_osu_account = std::env::var("REGISTER_OSU_ACCOUNT")
        .unwrap_or_else(|_| panic!("no osu account was given"))
        .split(',')
        .map(|account| account.into())
        .collect::<Vec<Box<str>>>();
    let osu_event_notify_group = std::env::var("OSU_EVENT_NOTIFY_TO")
        .unwrap_or_else(|_| panic!("You must at least give one group id for notify"))
        .split(',')
        .map(|id| {
            let id = id
                .parse::<i64>()
                .unwrap_or_else(|err| panic!("{id} can't be parse into number: {err}"));
            ChatId(id)
        })
        .collect::<Vec<ChatId>>();
    let token = std::env::var("OSU_API_TOKEN")
        .unwrap_or_else(|_| panic!("I need the api token to make a request"));
    let settings =
        maid::watcher::osu::Settings::new(token, register_osu_account, osu_event_notify_group);
    let redis_conn = connect_redis(redis_addr).await;
    maid::watcher::osu::spawn_watcher(settings, bot, redis_conn);
}

async fn setup_bilibili_watcher(bot: AutoSend<Bot>, redis_addr: &str) {
    let bili_event_notify_group = std::env::var("BILI_NOTIFY_GROUP")
        .unwrap_or_else(|_| panic!("You must at least give one group id for notify"))
        .split(',')
        .map(|id| {
            let id = id
                .parse::<i64>()
                .unwrap_or_else(|err| panic!("{id} can't be parse into number: {err}"));
            ChatId(id)
        })
        .collect::<Vec<ChatId>>();
    let watch_room_ids = std::env::var("BILI_WATCH_USER_ID")
        .unwrap_or_else(|_| panic!("You must at least give one room id for watching"))
        .split(',')
        .map(|id| {
            id.parse::<u32>()
                .unwrap_or_else(|err| panic!("{id} can't be parse into number: {err}"))
        })
        .collect::<Vec<u32>>();

    let redis_conn = connect_redis(redis_addr).await;
    let config = maid::watcher::bili::Config {
        watch: watch_room_ids,
        notify: bili_event_notify_group,
    };
    maid::watcher::bili::spawn_watcher(config, bot, redis_conn);
}

fn setup_deepl_translator() -> deepl::DeepLApi {
    let authkey =
        std::env::var("DEEPL_API_KEY").unwrap_or_else(|_| panic!("no deepl auth key found"));
    deepl::DeepLApi::new(&authkey)
}

async fn run() {
    let bot = Bot::from_env().auto_send();

    let redis_addr = std::env::var("REDIS_ADDR")
        .expect("fail to get redis addr, please check environment variable `REDIS_ADDR`.");
    let redis_conn = connect_redis(&redis_addr).await;
    let handler = maid::handler_schema();
    let status = dialogue::InMemStorage::<maid::DialogueStatus>::new();
    let fetcher = maid::Fetcher::new();
    let translator = setup_deepl_translator();
    let runtime = maid::Runtime::new(redis_conn, fetcher, translator);

    maid::spawn_healthcheck_listner(
        std::env::var("HEALTHCHECK_PORT")
            .unwrap_or_else(|_| "11451".to_string())
            .parse::<u16>()
            .expect("Invalid health check port number!"),
    );

    #[cfg(feature = "weibo")]
    setup_weibo_watcher(bot.clone());

    #[cfg(feature = "osu")]
    setup_osu_watcher(bot.clone(), &redis_addr).await;

    setup_bilibili_watcher(bot.clone(), &redis_addr).await;

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![runtime, status])
        .enable_ctrlc_handler()
        .default_handler(|_| async move {})
        .build()
        .dispatch()
        .await;
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();

    run().await;
}
