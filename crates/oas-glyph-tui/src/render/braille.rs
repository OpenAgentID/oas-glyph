//! # Braille Dot Pattern Renderer
//!
//! Each Unicode Braille character (U+2800 to U+28FF) encodes a 2×4 grid of dots.
//! This gives us 2× horizontal and 4× vertical resolution compared to normal text.
//!
//! ## Key design decisions for dense rendering:
//!
//! 1. **Solid fill mode** — block-size channel modulates brightness, not gap size.
//!    Every data cell fills fully so the glyph reads as a cohesive block.
//!
//! 2. **Correct aspect ratio** — terminal cells are ~1:2 (width:height). Braille
//!    characters are 2 dots wide × 4 dots tall, matching this ratio. For a square
//!    glyph, we need `char_cols` columns and `char_cols` rows (the 2:4 dot ratio
//!    inside each character compensates for the 1:2 cell ratio).
//!
//! 3. **Luminance threshold tuned low** — dots turn "on" at any brightness above
//!    the background, so subtle format-strip and dim data cells still contribute
//!    to the visual density.

use crate::{RenderOptions, GlyphSize, Result};
use super::{fg, reset, grid_to_color_buffer};
use oas_glyph::encoding::GlyphGrid;
use oas_glyph::visual::palette::{Color, GlyphPalette};

/// Braille base codepoint.
const BRAILLE_BASE: u32 = 0x2800;

/// Dot bit positions for the 2×4 grid.
const BRAILLE_DOTS: [[u8; 2]; 4] = [
    [0x01, 0x08], // row 0
    [0x02, 0x10], // row 1
    [0x04, 0x20], // row 2
    [0x40, 0x80], // row 3
];

/// Render using Braille dot patterns with per-character color.
#[allow(clippy::needless_range_loop)]
pub fn render(
    grid: &GlyphGrid,
    palette: &GlyphPalette,
    opts: &RenderOptions,
) -> Result<String> {
    let (char_cols, char_rows) = resolve_size(opts);

    // Each Braille character covers 2 dots wide × 4 dots tall.
    let dot_cols = char_cols * 2;
    let dot_rows = char_rows * 4;

    // Solid fill — every cell is filled, no gaps.
    let buffer = grid_to_color_buffer(grid, palette, dot_cols, dot_rows);

    // Background luminance threshold
    let bg_lum = luminance(&palette.background);
    let threshold = bg_lum + 0.06;

    let mut output = String::with_capacity(char_rows * char_cols * 24);

    for cr in 0..char_rows {
        output.push_str("  ");

        for cc in 0..char_cols {
            let mut pattern: u8 = 0;
            let mut color_r: u32 = 0;
            let mut color_g: u32 = 0;
            let mut color_b: u32 = 0;
            let mut lit_count: u32 = 0;

            for dy in 0..4 {
                for dx in 0..2 {
                    let py = cr * 4 + dy;
                    let px = cc * 2 + dx;

                    if py < dot_rows && px < dot_cols {
                        let c = &buffer[py][px];
                        let lum = luminance(c);

                        if lum > threshold {
                            pattern |= BRAILLE_DOTS[dy][dx];
                            color_r += c.r as u32;
                            color_g += c.g as u32;
                            color_b += c.b as u32;
                            lit_count += 1;
                        }
                    }
                }
            }

            if lit_count > 0 {
                let avg = Color::rgb(
                    (color_r / lit_count) as u8,
                    (color_g / lit_count) as u8,
                    (color_b / lit_count) as u8,
                );
                output.push_str(&fg(&avg));
            }

            let ch = char::from_u32(BRAILLE_BASE + pattern as u32).unwrap_or(' ');
            output.push(ch);
        }

        output.push_str(reset());
        output.push('\n');
    }

    Ok(output)
}

fn luminance(c: &Color) -> f64 {
    (0.299 * c.r as f64 + 0.587 * c.g as f64 + 0.114 * c.b as f64) / 255.0
}

/// For a square glyph, char_cols == char_rows.
/// Braille's 2:4 dot grid inside each cell handles the aspect ratio.
fn resolve_size(opts: &RenderOptions) -> (usize, usize) {
    match opts.size {
        GlyphSize::Cells(w, h) => (w as usize, h as usize),
        GlyphSize::Compact => (12, 12),
        GlyphSize::Auto => {
            let term_cols = crossterm::terminal::size()
                .map(|(c, _)| c)
                .unwrap_or(80);
            let side = ((term_cols as f64 * 0.30) as usize).clamp(12, 32);
            (side, side)
        }
        GlyphSize::Pixels(w, _) => {
            let side = (w / 8) as usize;
            (side, side)
        }
    }
}
