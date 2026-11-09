//! Apple Music album cards rendered entirely in Rust.
use std::io::Cursor;

use anyhow::{Context, Result};
use image::{
    imageops::{blur, resize, FilterType},
    DynamicImage, ImageFormat, Rgb, RgbImage,
};
use rusttype::Font;

use super::{
    apple_music_album::Album,
    card_drawing::{decode_artwork, draw, width, wrap},
};

const WIDTH: u32 = 1200;
const HEIGHT: u32 = 630;
const COVER: u32 = 480;
const WHITE: [u8; 3] = [255, 255, 255];
const GREY: [u8; 3] = [215, 215, 219];

pub fn render(album: &Album) -> Result<Vec<u8>> {
    let bold = Font::try_from_bytes(include_bytes!(env!("QUOTE_TEXT_FONT_PATH")))
        .context("Invalid title font")?;
    let regular = Font::try_from_bytes(include_bytes!(env!("CARD_REGULAR_FONT_PATH")))
        .context("Invalid artist font")?;
    let light = Font::try_from_bytes(include_bytes!(env!("QUOTE_USERNAME_FONT_PATH")))
        .context("Invalid album font")?;
    let artwork = album
        .artwork
        .as_deref()
        .and_then(|data| decode_artwork(data).ok());
    let mut image = background(artwork.as_ref());
    if let Some(artwork) = artwork {
        let cover = DynamicImage::ImageRgb8(artwork)
            .resize_to_fill(COVER, COVER, FilterType::Lanczos3)
            .to_rgb8();
        // Rounded square with antialiased edges and a restrained soft shadow.
        for y in 20..575 {
            for x in 20..575 {
                let d = rounded_distance(x as f32 - 55.0, y as f32 - 63.0);
                let shadow = 0.26 * (-d.max(0.0).powi(2) / 260.0).exp();
                let pixel = image.get_pixel_mut(x, y);
                for channel in &mut pixel.0 {
                    *channel = (*channel as f32 * (1.0 - shadow)) as u8;
                }
            }
        }
        for (x, y, pixel) in cover.enumerate_pixels() {
            let alpha = (0.5 - rounded_distance(x as f32 + 0.5, y as f32 + 0.5)).clamp(0.0, 1.0);
            let target = image.get_pixel_mut(x + 55, y + 55);
            for (dst, src) in target.0.iter_mut().zip(pixel.0) {
                *dst = (*dst as f32 * (1.0 - alpha) + src as f32 * alpha).round() as u8;
            }
        }
    } else {
        for y in 0..COVER {
            for x in 0..COVER {
                if rounded_distance(x as f32, y as f32) < 0.0 {
                    image.put_pixel(55 + x, 55 + y, Rgb([71, 69, 80]));
                }
            }
        }
        let x = (295.0 - width("♫", &light, 170.0) / 2.0) as u32;
        draw(&mut image, &light, 170.0, x, 190, GREY, "♫");
    }

    let title = album.track_title.as_deref().unwrap_or(&album.title);
    let meta_lines: Vec<_> = metadata(album)
        .iter()
        .flat_map(|line| wrap(line, &regular, 24.0, 565.0, 2))
        .collect();
    let metadata_gap = if meta_lines.is_empty() { 0.0 } else { 56.0 };
    let metadata_height = meta_lines.len() as f32 * 32.0;
    let mut scale = 1.0_f32;
    let (title_lines, artist_lines, album_lines, group_height) = loop {
        let title_lines = wrap(title, &bold, 76.0 * scale, 565.0, 3);
        let artist_lines = wrap(&album.artist, &regular, 32.0 * scale, 565.0, 3);
        let album_lines = if album.track_title.is_some() {
            wrap(&album.title, &regular, 32.0 * scale, 565.0, 2)
        } else {
            Vec::new()
        };
        // Tight line spacing keeps title, artist and album together as one group.
        let h = title_lines.len() as f32 * 76.0 * scale
            + (artist_lines.len() + album_lines.len()) as f32 * 34.0 * scale;
        if h + metadata_gap + metadata_height <= COVER as f32 || scale <= 0.55 {
            break (title_lines, artist_lines, album_lines, h);
        }
        scale -= 0.05;
    };
    // Balance both text groups together against the artwork's vertical center.
    let mut y = 295.0 - (group_height + metadata_gap + metadata_height) / 2.0;
    for line in title_lines {
        draw(&mut image, &bold, 76.0 * scale, 580, y as u32, WHITE, &line);
        y += 76.0 * scale;
    }
    for line in artist_lines {
        draw(
            &mut image,
            &regular,
            32.0 * scale,
            580,
            y as u32,
            WHITE,
            &line,
        );
        y += 34.0 * scale;
    }
    for line in album_lines {
        draw(
            &mut image,
            &regular,
            32.0 * scale,
            580,
            y as u32,
            GREY,
            &line,
        );
        y += 34.0 * scale;
    }
    y += metadata_gap;
    for line in meta_lines {
        draw(&mut image, &regular, 24.0, 580, y as u32, GREY, &line);
        y += 32.0;
    }
    draw_brand(&mut image)?;
    let mut png = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(image).write_to(&mut png, ImageFormat::Png)?;
    Ok(png.into_inner())
}

fn draw_brand(image: &mut RgbImage) -> Result<()> {
    // Rasterized from the accompanying Apple Music SVG at 4× its native size.
    // Embed it so card rendering needs no platform logo font or network request.
    let logo = image::load_from_memory(include_bytes!("assets/apple-music-white.png"))?.to_rgba8();
    let logo = resize(&logo, 144, 27, FilterType::Lanczos3);
    let left = (WIDTH - logo.width()) / 2;
    for (x, y, pixel) in logo.enumerate_pixels() {
        let alpha = pixel[3] as f32 / 255.0;
        let target = image.get_pixel_mut(left + x, 578 + y);
        for (dst, src) in target.0.iter_mut().zip(pixel.0) {
            *dst = (*dst as f32 * (1.0 - alpha) + src as f32 * alpha).round() as u8;
        }
    }
    Ok(())
}

fn metadata(album: &Album) -> Vec<String> {
    let mut rows = Vec::new();
    if let Some(date) = &album.release_date {
        rows.push(format!("Released {date}"));
    }
    let mut details = Vec::new();
    if let Some(count) = album.track_count {
        details.push(format!(
            "{count} {}",
            if count == 1 { "song" } else { "songs" }
        ));
    }
    if let Some(genre) = &album.genre {
        details.push(genre.clone());
    }
    if !details.is_empty() {
        rows.push(details.join(" · "));
    }
    rows
}

fn rounded_distance(x: f32, y: f32) -> f32 {
    let qx = (x - COVER as f32 / 2.0).abs() - (COVER as f32 / 2.0 - 24.0);
    let qy = (y - COVER as f32 / 2.0).abs() - (COVER as f32 / 2.0 - 24.0);
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - 24.0
}

fn background(artwork: Option<&RgbImage>) -> RgbImage {
    let Some(artwork) = artwork else {
        return RgbImage::from_pixel(WIDTH, HEIGHT, Rgb([45, 43, 55]));
    };
    // Blur at small resolution: broad artwork colors become a smooth gradient,
    // with no recognizable illustration behind the text.
    let small = resize(artwork, 96, 64, FilterType::Triangle);
    let small = blur(&small, 15.0);
    let mut background = resize(&small, WIDTH, HEIGHT, FilterType::CatmullRom);
    for (x, y, pixel) in background.enumerate_pixels_mut() {
        let dim = 0.47 - 0.10 * (y as f32 / HEIGHT as f32) - 0.07 * (x as f32 / WIDTH as f32);
        for channel in &mut pixel.0 {
            *channel = (*channel as f32 * dim + 12.0).round() as u8;
        }
    }
    background
}
