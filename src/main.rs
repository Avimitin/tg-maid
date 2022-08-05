mod butler;
mod cache;
mod modules;

use teloxide::{dispatching::dialogue, prelude::*};

async fn connect_redis(addr: &str) -> redis::aio::ConnectionManager {
    let client = redis::Client::open(addr).expect("fail to open connection to redis");
    client
        .get_tokio_connection_manager()
        .await
        .expect("fail to connect to redis")
}

pub async fn run() {
    let bot = Bot::from_env().auto_send();

    let redis_conn = connect_redis(
        &std::env::var("REDIS_ADDR")
            .expect("fail to get redis addr, please check environment variable `REDIS_ADDR`."),
    )
    .await;
    let handler = butler::handler_schema();
    let status = dialogue::InMemStorage::<butler::DialogueStatus>::new();
    let fetcher = butler::Fetcher::new();
    let runtime = butler::Runtime::new(redis_conn, fetcher);

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
