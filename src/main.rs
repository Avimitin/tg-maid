mod bot;
mod modules;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    crate::bot::run().await;
}
