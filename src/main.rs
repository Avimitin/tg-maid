mod modules;

use crate::modules::{handlers, runtime, weather};
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

    let redis_conn = connect_redis("").await;
    let handler = handlers::handler_schema();
    let status = dialogue::InMemStorage::<handlers::DialogueStatus>::new();
    let runtime = runtime::Runtime::new(redis_conn.clone(), redis_conn, weather::WttrInApi::new());

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

    run().await;
}
