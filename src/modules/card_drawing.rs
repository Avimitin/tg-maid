//! Shared bounded image decoding and CJK text layout for preview cards.
use anyhow::Result;
use image::RgbImage;
use rusttype::{point, Font, Point, PositionedGlyph, Scale};
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
    width_tracked(text, font, size, 0.0)
}

pub(super) fn width_tracked(text: &str, font: &Font<'_>, size: f32, tracking: f32) -> f32 {
    layout_tracked(text, font, size, tracking, point(0.0, 0.0))
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
    wrap_tracked(text, font, size, max_width, max_lines, 0.0)
}

pub(super) fn wrap_tracked(
    text: &str,
    font: &Font<'_>,
    size: f32,
    max_width: f32,
    max_lines: usize,
    tracking: f32,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for ch in text.chars() {
        line.push(ch);
        if width_tracked(&line, font, size, tracking) > max_width {
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
            while width_tracked(&format!("{last}…"), font, size, tracking) > max_width {
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
    draw_tracked(image, font, size, 0.0, (x, y), color, text);
}

pub(super) fn draw_tracked(
    image: &mut RgbImage,
    font: &Font<'_>,
    size: f32,
    tracking: f32,
    (x, y): (u32, u32),
    color: [u8; 3],
    text: &str,
) {
    let scale = Scale::uniform(size);
    let baseline = y as f32 + font.v_metrics(scale).ascent;
    for glyph in layout_tracked(text, font, size, tracking, point(x as f32, baseline)) {
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

fn layout_tracked<'text, 'font>(
    text: &'text str,
    font: &'text Font<'font>,
    size: f32,
    tracking: f32,
    origin: Point<f32>,
) -> impl Iterator<Item = PositionedGlyph<'font>> + 'text {
    let mut offset = 0.0;
    let mut right_edge: Option<f32> = None;
    font.layout(text, Scale::uniform(size), origin)
        .enumerate()
        .map(move |(index, mut glyph)| {
            let advancing = glyph.unpositioned().h_metrics().advance_width > 0.0;
            if index > 0 && advancing {
                offset += tracking;
            }
            let mut position = glyph.position();
            position.x += offset;
            if tracking < 0.0 && advancing {
                if let Some(bounds) = glyph.unpositioned().exact_bounding_box() {
                    // Keep a pixel between outlines when tightening letters. Zero-advance
                    // combining marks retain their original placement on the base glyph.
                    if let Some(right) = right_edge {
                        let clearance = (right + 1.0 - position.x - bounds.min.x).max(0.0);
                        offset += clearance;
                        position.x += clearance;
                    }
                    right_edge = Some(position.x + bounds.max.x);
                }
            }
            glyph.set_position(position);
            glyph
        })
}
