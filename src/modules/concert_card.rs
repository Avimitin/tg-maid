//! Render a concert card entirely in Rust, using the bot's bundled CJK fonts.
use std::io::Cursor;

use anyhow::{Context, Result};
use image::{DynamicImage, ImageFormat, Rgb, RgbImage};
use rusttype::Font;

use super::card_drawing::{decode_artwork, draw, width, wrap};

use super::apple_music::Concert;

const WIDTH: u32 = 1200;
const RIGHT: u32 = 610;
const MARGIN: u32 = 54;
const INK: [u8; 3] = [29, 29, 31];
const MUTED: [u8; 3] = [110, 110, 115];
const ACCENT: [u8; 3] = [250, 35, 59];

pub fn render(concert: &Concert) -> Result<Vec<u8>> {
    let bold = Font::try_from_bytes(include_bytes!(env!("QUOTE_TEXT_FONT_PATH")))
        .context("Unable to load concert title font")?;
    let regular = Font::try_from_bytes(include_bytes!(env!("QUOTE_USERNAME_FONT_PATH")))
        .context("Unable to load concert detail font")?;
    let artwork = concert
        .share_artwork
        .as_deref()
        .and_then(|bytes| decode_artwork(bytes).ok());
    let background = artwork
        .as_ref()
        .map(|image| image.get_pixel(0, 0).0)
        .unwrap_or([247, 247, 252]);

    let mut rows = Vec::new();
    // Each group owns its line spacing; larger gaps separate title, time and venue.
    let mut block = |text: &str,
                     font: &Font<'static>,
                     size: f32,
                     color: [u8; 3],
                     x: u32,
                     mut y: u32,
                     max_lines: usize| {
        for line in wrap(text, font, size, (WIDTH - x - MARGIN) as f32, max_lines) {
            rows.push((line, font.clone(), size, color, x, y));
            y += (size * 1.12).ceil() as u32;
        }
        y
    };
    let title_end = block(&concert.title, &bold, 54.0, INK, RIGHT, 0, 3);
    let time_y = title_end + 44;
    let time_label_end = block("Listed time", &regular, 22.0, MUTED, RIGHT + 50, time_y, 1);
    let time_end = if let Some((date, time)) = concert.listed_time.rsplit_once(", ") {
        let date_end = block(date, &regular, 36.0, INK, RIGHT + 50, time_label_end + 2, 2);
        block(time, &regular, 30.0, MUTED, RIGHT + 50, date_end + 2, 2)
    } else {
        block(
            &concert.listed_time,
            &regular,
            32.0,
            INK,
            RIGHT + 50,
            time_label_end + 2,
            3,
        )
    };
    let venue_y = time_end + 44;
    let venue_end = block(&concert.venue, &regular, 42.0, INK, RIGHT + 50, venue_y, 2);
    let details_end = if let Some(address) = &concert.address {
        block(address, &regular, 28.0, MUTED, RIGHT + 50, venue_end + 2, 3)
    } else {
        venue_end
    };

    let artists = concert
        .artists
        .iter()
        .map(|artist| artist.name.as_str())
        .collect::<Vec<_>>()
        .join(" / ");
    let artist_lines = wrap(&artists, &bold, 44.0, 490.0, 2);
    let artist_height = artist_lines.len() as u32 * 50;
    let left_height = 470
        + if artist_lines.is_empty() {
            0
        } else {
            12 + artist_height
        };
    // Center both columns on the same body, then center the body and compact
    // footer together on the canvas. This also works for longer event titles.
    let body_height = details_end.max(left_height);
    let footer_gap = 20;
    let group_height = body_height + footer_gap + 26;
    let height = (group_height + 64).max(630);
    let body_y = (height - group_height) / 2;
    let left_y = body_y + (body_height - left_height) / 2;
    let right_y = body_y + (body_height - details_end) / 2;
    let footer_y = body_y + body_height + footer_gap;
    let mut image = RgbImage::from_pixel(WIDTH, height, Rgb(background));
    if let Some(artwork) = artwork {
        let artwork = trim_share_background(&artwork);
        let artwork = DynamicImage::ImageRgb8(artwork)
            .resize(470, 470, image::imageops::FilterType::Lanczos3)
            .to_rgb8();
        let x = 61 + (470 - artwork.width()) / 2;
        let y = left_y + (470 - artwork.height()) / 2;
        image::imageops::replace(&mut image, &artwork, x.into(), y.into());
    } else {
        // The event remains readable when Apple does not provide share artwork.
        let cx = 296.0_f32;
        let cy = left_y as f32 + 235.0;
        for y in 0..height {
            for x in 61..531 {
                let distance = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
                let coverage = (207.0 - distance).clamp(0.0, 1.0);
                let pixel = image.get_pixel_mut(x, y);
                for (channel, tint) in pixel.0.iter_mut().zip([252, 229, 235]) {
                    *channel = (*channel as f32 * (1.0 - coverage) + tint as f32 * coverage) as u8;
                }
            }
        }
        let note = "♫";
        draw(
            &mut image,
            &regular,
            180.0,
            (cx - width(note, &regular, 180.0) / 2.0) as u32,
            (cy - 110.0) as u32,
            ACCENT,
            note,
        );
    }
    for (index, artist) in artist_lines.iter().enumerate() {
        let x = (296.0 - width(artist, &bold, 44.0) / 2.0).max(0.0) as u32;
        draw(
            &mut image,
            &bold,
            44.0,
            x,
            left_y + 470 + 12 + index as u32 * 50,
            INK,
            artist,
        );
    }
    for (line, font, size, color, x, y) in rows {
        draw(&mut image, &font, size, x, right_y + y, color, &line);
    }
    draw_icon(&mut image, RIGHT, right_y + time_y + 6, Icon::Clock);
    draw_icon(&mut image, RIGHT, right_y + venue_y + 10, Icon::Location);
    let footer = "Apple Music";
    let footer_x = (WIDTH as f32 - width(footer, &regular, 22.0)) / 2.0;
    draw(
        &mut image,
        &regular,
        22.0,
        footer_x as u32,
        footer_y,
        MUTED,
        footer,
    );
    let mut png = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(image).write_to(&mut png, ImageFormat::Png)?;
    Ok(png.into_inner())
}

#[derive(Clone, Copy)]
enum Icon {
    Clock,
    Location,
}

/// Small outline symbols drawn with signed distances for smooth edges, without
/// relying on platform emoji fonts or external icon assets.
fn draw_icon(image: &mut RgbImage, x: u32, y: u32, icon: Icon) {
    for iy in 0..36 {
        for ix in 0..36 {
            let p = (ix as f32 + 0.5, iy as f32 + 0.5);
            let distance = icon_distance(p, icon);
            let shadow_distance = icon_distance((p.0 - 0.6, p.1 - 1.2), icon);
            let shadow = 0.12 * (-shadow_distance.max(0.0).powi(2) / 4.0).exp();
            let coverage = (1.8 - distance).clamp(0.0, 1.0);
            if x + ix < image.width() && y + iy < image.height() {
                let pixel = image.get_pixel_mut(x + ix, y + iy);
                for (channel, ink) in pixel.0.iter_mut().zip(INK) {
                    let shaded = *channel as f32 * (1.0 - shadow);
                    *channel = (shaded * (1.0 - coverage) + ink as f32 * coverage).round() as u8;
                }
            }
        }
    }
}

fn icon_distance(p: (f32, f32), icon: Icon) -> f32 {
    match icon {
        Icon::Clock => {
            let ring = (((p.0 - 18.0).powi(2) + (p.1 - 18.0).powi(2)).sqrt() - 14.0).abs();
            ring.min(segment_distance(p, (18.0, 9.0), (18.0, 18.0)))
                .min(segment_distance(p, (18.0, 18.0), (24.0, 21.5)))
        }
        Icon::Location => {
            let hole = (((p.0 - 18.0).powi(2) + (p.1 - 13.0).powi(2)).sqrt() - 3.5).abs();
            let circle = if p.1 <= 17.0 {
                (((p.0 - 18.0).powi(2) + (p.1 - 13.0).powi(2)).sqrt() - 10.0).abs()
            } else {
                f32::INFINITY
            };
            hole.min(circle)
                .min(segment_distance(p, (8.83, 17.0), (18.0, 32.0)))
                .min(segment_distance(p, (27.17, 17.0), (18.0, 32.0)))
        }
    }
}

fn segment_distance(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let delta = (b.0 - a.0, b.1 - a.1);
    let t = (((p.0 - a.0) * delta.0 + (p.1 - a.1) * delta.1)
        / (delta.0 * delta.0 + delta.1 * delta.1))
        .clamp(0.0, 1.0);
    ((p.0 - a.0 - t * delta.0).powi(2) + (p.1 - a.1 - t * delta.1).powi(2)).sqrt()
}

/// Crop the empty margins of Apple's share card, preserving its photo, calendar,
/// and shadow as a single composition. No artist artwork is synthesized.
fn trim_share_background(image: &RgbImage) -> RgbImage {
    let background = image.get_pixel(0, 0).0;
    let (mut left, mut top, mut right, mut bottom) = (image.width(), image.height(), 0, 0);
    for (x, y, pixel) in image.enumerate_pixels() {
        let difference: u32 = pixel
            .0
            .iter()
            .zip(background)
            .map(|(channel, bg)| (*channel as i32 - bg as i32).unsigned_abs())
            .sum();
        if difference > 36 {
            left = left.min(x);
            top = top.min(y);
            right = right.max(x);
            bottom = bottom.max(y);
        }
    }
    if left >= right || top >= bottom {
        return image.clone();
    }
    let padding = image.width().min(image.height()) / 25;
    left = left.saturating_sub(padding);
    top = top.saturating_sub(padding);
    right = (right + padding + 1).min(image.width());
    bottom = (bottom + padding + 1).min(image.height());
    image::imageops::crop_imm(image, left, top, right - left, bottom - top).to_image()
}
