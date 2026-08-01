//! # Sixel Graphics Renderer
//!
//! Sixel is a bitmap graphics protocol from the DEC VT340 era (1987).
//! Each "sixel" is a column of 6 vertical pixels encoded as a single byte.
//! Despite its age, it's well-supported by modern terminals like foot, mlterm,
//! contour, and XTerm with +sixel compilation flag.
//!
//! ## Encoding
//!
//! Sixel data is framed as: `\x1bPq{sixel_data}\x1b\\`
//!
//! Within the data:
//! - `#N;2;R;G;B` defines color register N as RGB (0-100 scale)
//! - Characters `?` through `~` (ASCII 63-126) encode 6 vertical pixels
//! - `$` = carriage return (same line)
//! - `-` = newline (next 6-pixel row)
//! - `!N{char}` = repeat char N times (run-length encoding)

use crate::{RenderOptions, Result, TuiError};

/// Render a PNG image as Sixel graphics.
///
/// Note: Full production implementation requires decoding the PNG to raw pixels
/// and converting to sixel format. For the MVP, this outputs a placeholder
/// message. Production path: use the `image` crate to decode, quantize to
/// the sixel color register limit (256 colors), then encode scanlines.
pub fn render(png_data: &[u8], _opts: &RenderOptions) -> Result<String> {
    // Decode PNG to RGBA pixels
    let img = image::load_from_memory(png_data)
        .map_err(|e| TuiError::Render(format!("PNG decode: {}", e)))?;
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);

    // Quantize to 64 colors (sixel supports up to 256 registers)
    let palette = quantize_palette(&rgba, 64);

    let mut sixel = String::with_capacity(w * h);

    // Sixel header: DCS q
    sixel.push_str("\x1bPq");

    // Set raster attributes: "pixel aspect ratio 1:1, width, height
    sixel.push_str(&format!("\"1;1;{};{}", w, h));

    // Define color registers
    for (i, &(r, g, b)) in palette.iter().enumerate() {
        // Sixel RGB is 0-100 scale
        let rs = (r as u32 * 100 / 255) as u8;
        let gs = (g as u32 * 100 / 255) as u8;
        let bs = (b as u32 * 100 / 255) as u8;
        sixel.push_str(&format!("#{};2;{};{};{}", i, rs, gs, bs));
    }

    // Encode scanlines in groups of 6 rows
    let pixels = rgba.as_raw();
    for band_y in (0..h).step_by(6) {
        for (ci, &(_pr, _pg, _pb)) in palette.iter().enumerate() {
            sixel.push_str(&format!("#{}", ci));

            let mut run_char: Option<u8> = None;
            let mut run_len = 0u32;

            for x in 0..w {
                let mut sixel_byte: u8 = 0;
                for dy in 0..6 {
                    let y = band_y + dy;
                    if y < h {
                        let idx = (y * w + x) * 4;
                        let r = pixels[idx];
                        let g = pixels[idx + 1];
                        let b = pixels[idx + 2];

                        // Check if this pixel matches the current color register
                        if closest_palette_index(r, g, b, &palette) == ci {
                            sixel_byte |= 1 << dy;
                        }
                    }
                }

                let ch = sixel_byte + 63; // ASCII offset

                if run_char == Some(ch) {
                    run_len += 1;
                } else {
                    // Flush previous run
                    flush_run(&mut sixel, run_char, run_len);
                    run_char = Some(ch);
                    run_len = 1;
                }
            }
            flush_run(&mut sixel, run_char, run_len);
            sixel.push('$'); // Carriage return (same band)
        }
        sixel.push('-'); // Next band
    }

    // Sixel terminator: ST
    sixel.push_str("\x1b\\");
    sixel.push('\n');

    Ok(sixel)
}

fn flush_run(out: &mut String, ch: Option<u8>, len: u32) {
    if let Some(c) = ch {
        if len > 3 {
            out.push_str(&format!("!{}{}", len, c as char));
        } else {
            for _ in 0..len {
                out.push(c as char);
            }
        }
    }
}

/// Simple median-cut color quantization.
fn quantize_palette(img: &image::RgbaImage, max_colors: usize) -> Vec<(u8, u8, u8)> {
    use std::collections::HashMap;

    let mut color_counts: HashMap<(u8, u8, u8), u32> = HashMap::new();
    for pixel in img.pixels() {
        let key = (
            pixel[0] & 0xF0, // Reduce to 4-bit per channel for counting
            pixel[1] & 0xF0,
            pixel[2] & 0xF0,
        );
        *color_counts.entry(key).or_default() += 1;
    }

    let mut colors: Vec<_> = color_counts.into_iter().collect();
    colors.sort_by_key(|b| std::cmp::Reverse(b.1)); // Most frequent first
    colors.truncate(max_colors);

    colors.into_iter().map(|((r, g, b), _)| (r, g, b)).collect()
}

fn closest_palette_index(r: u8, g: u8, b: u8, palette: &[(u8, u8, u8)]) -> usize {
    let mut best = 0;
    let mut best_dist = u32::MAX;
    for (i, &(pr, pg, pb)) in palette.iter().enumerate() {
        let dr = r.abs_diff(pr) as u32;
        let dg = g.abs_diff(pg) as u32;
        let db = b.abs_diff(pb) as u32;
        let dist = dr * dr + dg * dg + db * db;
        if dist < best_dist {
            best_dist = dist;
            best = i;
        }
    }
    best
}
