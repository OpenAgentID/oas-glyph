//! # ASCII Block Renderer
//!
//! Maximum-compatibility fallback using ASCII block characters (░▒▓█) and
//! ANSI 256-color palette. Works in virtually any terminal emulator made
//! in the last 20 years.
//!
//! Each character cell maps to one grid cell. Block density encodes brightness.
//! Color uses the nearest match in the 256-color ANSI palette.

use super::{grid_to_color_buffer, reset};
use crate::{GlyphSize, RenderOptions, Result};
use oas_glyph::encoding::GlyphGrid;
use oas_glyph::visual::palette::GlyphPalette;

/// Density characters from lightest to darkest.
const DENSITY: [char; 5] = [' ', '░', '▒', '▓', '█'];

/// Render using ASCII blocks with 256-color ANSI.
#[allow(clippy::needless_range_loop)]
pub fn render(grid: &GlyphGrid, palette: &GlyphPalette, opts: &RenderOptions) -> Result<String> {
    let cols = resolve_cols(opts);
    let rows = cols; // Square aspect (each char is ~2:1 but we keep it simple)

    let buffer = grid_to_color_buffer(grid, palette, cols, rows);

    let mut output = String::with_capacity(rows * cols * 20);

    for row in 0..rows {
        output.push_str("  ");
        for col in 0..cols {
            let c = &buffer[row][col];
            let lum = (0.299 * c.r as f64 + 0.587 * c.g as f64 + 0.114 * c.b as f64) / 255.0;

            // Map luminance to density character
            let di = (lum * 4.0).round() as usize;
            let ch = DENSITY[di.min(4)];

            // Map to nearest ANSI 256 color
            let ansi = rgb_to_ansi256(c.r, c.g, c.b);
            output.push_str(&format!("\x1b[38;5;{}m", ansi));
            output.push(ch);
        }
        output.push_str(reset());
        output.push('\n');
    }

    Ok(output)
}

/// Map RGB to nearest ANSI 256-color code.
fn rgb_to_ansi256(r: u8, g: u8, b: u8) -> u8 {
    // Check if it's close to a grayscale value (232-255 range)
    if r.abs_diff(g) < 8 && g.abs_diff(b) < 8 {
        let gray = (r as u16 + g as u16 + b as u16) / 3;
        if gray < 8 {
            return 16; // black
        }
        if gray > 248 {
            return 231; // white
        }
        return (((gray - 8) as f64 / 247.0 * 24.0).round() as u8) + 232;
    }

    // Map to the 6×6×6 color cube (16-231)
    let ri = (r as f64 / 255.0 * 5.0).round() as u8;
    let gi = (g as f64 / 255.0 * 5.0).round() as u8;
    let bi = (b as f64 / 255.0 * 5.0).round() as u8;

    16 + 36 * ri + 6 * gi + bi
}

fn resolve_cols(opts: &RenderOptions) -> usize {
    match opts.size {
        GlyphSize::Cells(w, _) => w as usize,
        GlyphSize::Compact => 12,
        GlyphSize::Auto => {
            let term_cols = crossterm::terminal::size().map(|(c, _)| c).unwrap_or(80);
            ((term_cols as f64 * 0.3) as usize).clamp(12, 36)
        }
        GlyphSize::Pixels(w, _) => (w / 8) as usize,
    }
}
