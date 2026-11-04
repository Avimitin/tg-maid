//! Generate a preview locally, without a Telegram token or Redis.
use anyhow::{Context, Result};
use rusty_maid::modules::{
    apple_music::{ConcertLink, ConcertPreviews},
    concert_card,
};

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let url = args
        .next()
        .context("Usage: concert_preview APPLE_MUSIC_URL [OUTPUT.png]")?;
    let output = args.next().unwrap_or_else(|| "concert.png".into());
    let link = ConcertLink::parse(&url.parse()?).context("Not an Apple Music concert URL")?;
    let proxy = std::env::var("APPLE_MUSIC_PROXY").ok();
    let concert = ConcertPreviews::new(proxy.as_deref())?.get(&link).await?;
    let png = concert_card::render(&concert)?;
    std::fs::write(&output, png)?;
    println!("Saved {output}\n{}", concert.caption_html(&link.url));
    Ok(())
}
