//! Shared bounded image decoding and CJK text layout for preview cards.
use anyhow::Result;
use image::RgbImage;
use rusttype::{point, Font, Scale};
use std::io::Cursor;

pub(super) fn decode_artwork(bytes: &[u8]) -> Result<RgbImage> {
    let mut reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    Ok(reader.decode()?.to_rgb8())
}

pub(super) fn width(text: &str, font: &Font<'_>, size: f32) -> f32 {
    font.layout(text, Scale::uniform(size), point(0.0, 0.0))
        .last()
        .map(|g| g.position().x + g.unpositioned().h_metrics().advance_width)
        .unwrap_or(0.0)
}

pub(super) fn wrap(
    text: &str,
    font: &Font<'_>,
    size: f32,
    max_width: f32,
    max_lines: usize,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for ch in text.chars() {
        line.push(ch);
        if width(&line, font, size) > max_width {
            if let Some(space) = line.rfind(' ').filter(|space| *space > 0) {
                let rest = line[space + 1..].to_string();
                line.truncate(space);
                lines.push(line);
                line = rest;
            } else {
                line.pop();
                lines.push(line);
                line = ch.to_string();
            }
        }
    }
    if !line.trim().is_empty() {
        lines.push(line.trim().to_string());
    }
    if lines.len() > max_lines {
        lines.truncate(max_lines);
        if let Some(last) = lines.last_mut() {
            while width(&format!("{last}…"), font, size) > max_width {
                last.pop();
            }
            last.push('…');
        }
    }
    lines
}

pub(super) fn draw(
    image: &mut RgbImage,
    font: &Font<'_>,
    size: f32,
    x: u32,
    y: u32,
    color: [u8; 3],
    text: &str,
) {
    let scale = Scale::uniform(size);
    let baseline = y as f32 + font.v_metrics(scale).ascent;
    for glyph in font.layout(text, scale, point(x as f32, baseline)) {
        if let Some(bounds) = glyph.pixel_bounding_box() {
            glyph.draw(|gx, gy, coverage| {
                let px = bounds.min.x + gx as i32;
                let py = bounds.min.y + gy as i32;
                if px >= 0 && py >= 0 && px < image.width() as i32 && py < image.height() as i32 {
                    let pixel = image.get_pixel_mut(px as u32, py as u32);
                    for (channel, foreground) in pixel.0.iter_mut().zip(color) {
                        *channel = (*channel as f32 * (1.0 - coverage)
                            + foreground as f32 * coverage)
                            .round() as u8;
                    }
                }
            });
        }
    }
}
