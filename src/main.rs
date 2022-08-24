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

async fn run() {
    let bot = Bot::from_env().auto_send();

    let redis_addr = std::env::var("REDIS_ADDR")
        .expect("fail to get redis addr, please check environment variable `REDIS_ADDR`.");
    let redis_conn = connect_redis(&redis_addr).await;
    let handler = maid::handler_schema();
    let status = dialogue::InMemStorage::<maid::DialogueStatus>::new();
    let fetcher = maid::Fetcher::new();
    let runtime = maid::Runtime::new(redis_conn, fetcher);

    maid::spawn_healthcheck_listner(
        std::env::var("HEALTHCHECK_PORT")
            .unwrap_or_else(|_| "11451".to_string())
            .parse::<u16>()
            .expect("Invalid health check port number!"),
    );

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

    maid::watcher::weibo::spawn(bot.clone(), weibo_listen_config);

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![runtime, status])
        .enable_ctrlc_handler()
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
