mod bot;
mod currency;

#[cfg(feature = "redis")]
mod redis;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    crate::bot::run().await;
}
