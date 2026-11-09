//! Generate an album card without Telegram or Redis.
use anyhow::{Context, Result};
use rusty_maid::modules::{
    album_card,
    apple_music_album::{AlbumLink, AlbumPreviews},
};

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let url = args
        .next()
        .context("Usage: album_preview APPLE_MUSIC_URL [OUTPUT.png]")?;
    let output = args.next().unwrap_or_else(|| "album.png".into());
    let link = AlbumLink::parse(&url.parse()?).context("Not an Apple Music album URL")?;
    let proxy = std::env::var("APPLE_MUSIC_PROXY").ok();
    let album = AlbumPreviews::new(proxy.as_deref())?.get(&link).await?;
    let png = album_card::render(&album)?;
    std::fs::write(&output, png)?;
    println!("Saved {output}\n{}", album.caption_html());
    Ok(())
}
