mod bot;
mod currency;

fn main() {
    dotenv::dotenv().ok();

    crate::bot::run();
}
