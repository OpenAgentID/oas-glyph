//! # Unicode Half-Block Renderer
//!
//! Uses the "upper half block" character (▀, U+2580) to pack two vertical pixels
//! into each character cell. The top pixel is the foreground color and the bottom
//! pixel is the background color. With true-color (24-bit) ANSI escapes, this
//! gives us effective 2:1 vertical resolution — a 24-row glyph renders in 12
//! character rows.
//!
//! This is the default renderer for any terminal with `COLORTERM=truecolor`.
//! It produces beautiful results in Alacritty, WezTerm, iTerm2, Kitty, GNOME
//! Terminal, Windows Terminal, and most modern emulators.

use crate::{RenderOptions, GlyphSize, Result};
use super::{fg, bg, reset, grid_to_color_buffer};
use oas_glyph::encoding::GlyphGrid;
use oas_glyph::visual::palette::GlyphPalette;

/// The half-block character: top pixel = foreground, bottom pixel = background.
const UPPER_HALF: char = '▀';
/// Full block when both pixels are the same color.
const FULL_BLOCK: char = '█';
/// Lower half block (alternative encoding). Reserved for future use.
#[allow(dead_code)]
const LOWER_HALF: char = '▄';

/// Render a glyph grid using Unicode half-block characters.
pub fn render(
    grid: &GlyphGrid,
    palette: &GlyphPalette,
    opts: &RenderOptions,
) -> Result<String> {
    let (cols, pixel_rows) = resolve_size(opts);

    // pixel_rows must be even (two pixels per character row)
    let pixel_rows = if pixel_rows % 2 != 0 { pixel_rows + 1 } else { pixel_rows };

    let buffer = grid_to_color_buffer(grid, palette, cols, pixel_rows);
    let char_rows = pixel_rows / 2;

    let mut output = String::with_capacity(char_rows * cols * 40);

    for cr in 0..char_rows {
        output.push_str("  "); // Left margin

        for (top, bottom) in buffer[cr * 2].iter().zip(buffer[cr * 2 + 1].iter()).take(cols) {

            if top.r == bottom.r && top.g == bottom.g && top.b == bottom.b {
                // Same color → full block (saves bytes)
                output.push_str(&fg(top));
                output.push(FULL_BLOCK);
            } else {
                // Different → half block with fg=top, bg=bottom
                output.push_str(&fg(top));
                output.push_str(&bg(bottom));
                output.push(UPPER_HALF);
            }
        }

        output.push_str(reset());
        output.push('\n');
    }

    Ok(output)
}

/// Resolve the render size in (columns, pixel_rows).
///
/// Terminal cells are ~1:2 (width:height). Half-block gives 2 pixels per row.
/// For a visually square glyph: pixel_rows = cols (since each char row = 2px tall
/// and each char col = 1px wide, we need 2× as many pixel rows as cols, but
/// half-block halves the row count, so cols == pixel_rows gives square output).
fn resolve_size(opts: &RenderOptions) -> (usize, usize) {
    match opts.size {
        GlyphSize::Cells(w, _h) => (w as usize, w as usize), // square
        GlyphSize::Pixels(w, h) => {
            ((w / 4) as usize, (h / 4) as usize)
        }
        GlyphSize::Compact => (16, 16),
        GlyphSize::Auto => {
            let term_cols = crossterm::terminal::size()
                .map(|(c, _)| c)
                .unwrap_or(80);
            // ~35% of terminal width, square output
            let cols = ((term_cols as f64 * 0.35) as usize).clamp(16, 40);
            (cols, cols)
        }
    }
}

/// Render a compact inline glyph (single line, minimal width).
///
/// Uses a 6-cell wide representation with the kind badge colored.
pub fn render_compact(
    grid: &GlyphGrid,
    palette: &GlyphPalette,
) -> String {
    let buffer = grid_to_color_buffer(grid, palette, 6, 6);
    let mut out = String::new();

    // 6 pixels tall → 3 character rows, but for compact we do 1 row
    // by sampling every other row
    for (top, bottom) in buffer[1].iter().zip(buffer[2].iter()).take(6) {
        out.push_str(&fg(top));
        out.push_str(&bg(bottom));
        out.push(UPPER_HALF);
    }
    out.push_str(reset());
    out
}
