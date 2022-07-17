mod bot;
mod currency;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    crate::bot::run().await;
}
