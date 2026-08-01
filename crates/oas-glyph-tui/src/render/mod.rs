//! # Terminal Renderers
//!
//! Each sub-module implements a different terminal rendering backend.
//! All renderers produce a `String` containing the appropriate ANSI/protocol
//! escape sequences that can be printed to stdout.

pub mod ascii;
pub mod border;
pub mod braille;
pub mod halfblock;
pub mod iterm2;
pub mod kitty;
pub mod sixel;

use oas_glyph::visual::palette::{Color, GlyphPalette};
use oas_glyph::ParsedDid;

/// ANSI true-color foreground escape.
pub fn fg(c: &Color) -> String {
    format!("\x1b[38;2;{};{};{}m", c.r, c.g, c.b)
}

/// ANSI true-color background escape.
pub fn bg(c: &Color) -> String {
    format!("\x1b[48;2;{};{};{}m", c.r, c.g, c.b)
}

/// ANSI reset.
pub fn reset() -> &'static str {
    "\x1b[0m"
}

/// ANSI bold.
pub fn bold() -> &'static str {
    "\x1b[1m"
}

/// ANSI dim.
pub fn dim() -> &'static str {
    "\x1b[2m"
}

/// Render a colored palette strip below the glyph.
pub fn palette_strip(palette: &GlyphPalette) -> String {
    let mut out = String::new();
    let colors = [
        &palette.primary,
        &palette.secondary,
        &palette.data_colors[0],
        &palette.data_colors[1],
        &palette.data_colors[2],
        &palette.data_colors[3],
        &palette.kind_accent,
    ];
    out.push_str("  ");
    for c in &colors {
        out.push_str(&bg(c));
        out.push_str("   "); // 3 spaces = one color swatch
    }
    out.push_str(reset());
    out.push('\n');
    out
}

/// Render the DID as a dimmed label.
pub fn did_label(did: &str) -> String {
    // Truncate middle for long DIDs
    let display = if did.len() > 60 {
        format!("{}…{}", &did[..28], &did[did.len() - 24..])
    } else {
        did.to_string()
    };
    format!("  {}{}{}\n", dim(), display, reset())
}

/// Render an entity kind badge with color.
pub fn kind_badge(parsed: &ParsedDid) -> String {
    let palette = GlyphPalette::from_did(&parsed.did);
    let label = parsed.kind.as_str();
    let ns = &parsed.namespace;

    format!(
        "  {}{}  {} {} {}{}{} {}\n",
        bg(&palette.kind_accent),
        bold(),
        label,
        reset(),
        dim(),
        ns,
        reset(),
        "",
    )
}

/// Buffer fill mode controls how data cells are rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillMode {
    /// Every data cell fills its entire area with its color.
    /// Block-size channel is encoded via brightness modulation instead.
    /// Use for all text-based renderers (half-block, braille, ASCII).
    Solid,
    /// Data cells use the block-size channel to create variable-size shapes
    /// with transparent margins. Use for SVG/PNG pixel rendering only.
    Shaped,
}

/// Map a glyph grid to a 2D color buffer at a given resolution.
///
/// Returns a `rows × cols` grid of `Color` values.
///
/// For text renderers, use `FillMode::Solid` — every data cell fills fully,
/// and the block-size channel modulates brightness instead of creating gaps.
/// This produces dense, cohesive glyphs at any character resolution.
pub fn grid_to_color_buffer(
    grid: &oas_glyph::encoding::GlyphGrid,
    palette: &GlyphPalette,
    cols: usize,
    rows: usize,
) -> Vec<Vec<Color>> {
    grid_to_color_buffer_mode(grid, palette, cols, rows, FillMode::Solid)
}

/// Map a glyph grid with explicit fill mode control.
#[allow(clippy::needless_range_loop)]
pub fn grid_to_color_buffer_mode(
    grid: &oas_glyph::encoding::GlyphGrid,
    palette: &GlyphPalette,
    cols: usize,
    rows: usize,
    mode: FillMode,
) -> Vec<Vec<Color>> {
    use oas_glyph::encoding::grid::{CellType, GRID_SIZE};

    let mut buffer = vec![vec![palette.background; cols]; rows];
    let cell_w = cols as f64 / GRID_SIZE as f64;
    let cell_h = rows as f64 / GRID_SIZE as f64;

    for py in 0..rows {
        for px in 0..cols {
            let gr = (py as f64 / cell_h) as usize;
            let gc = (px as f64 / cell_w) as usize;

            if gr >= GRID_SIZE || gc >= GRID_SIZE {
                continue;
            }

            let cell = &grid.cells[gr][gc];
            let color = match cell.cell_type {
                CellType::Finder => {
                    if cell.value > 32 {
                        palette.finder_light
                    } else {
                        palette.finder_dark
                    }
                }
                CellType::Kind => {
                    let intensity = cell.value as f64 / 63.0;
                    Color::lerp(&palette.background, &palette.kind_accent, intensity)
                }
                CellType::Alignment => palette.finder_light,
                CellType::Format => {
                    let intensity = (cell.value as f64 / 63.0) * 0.5;
                    Color::lerp(&palette.background, &palette.secondary, intensity)
                }
                CellType::Data => {
                    let hue_idx = (cell.value & 0x03) as usize;
                    let base_color = palette.data_colors[hue_idx % 4];

                    match mode {
                        FillMode::Solid => {
                            // Block-size modulates brightness: larger = brighter.
                            // This keeps every cell filled while still encoding
                            // the size channel visually.
                            let size_idx = (cell.value >> 4) & 0x03;
                            let brightness = match size_idx {
                                0 => 0.55, // dim
                                1 => 0.72,
                                2 => 0.88,
                                3 => 1.0, // full brightness
                                _ => 0.72,
                            };
                            Color::rgb(
                                (base_color.r as f64 * brightness) as u8,
                                (base_color.g as f64 * brightness) as u8,
                                (base_color.b as f64 * brightness) as u8,
                            )
                        }
                        FillMode::Shaped => {
                            let size_idx = (cell.value >> 4) & 0x03;
                            let fracs = [0.35_f64, 0.55, 0.75, 0.95];
                            let frac = fracs[size_idx as usize];

                            let local_x = (px as f64 - gc as f64 * cell_w) / cell_w;
                            let local_y = (py as f64 - gr as f64 * cell_h) / cell_h;
                            let margin = (1.0 - frac) / 2.0;

                            if local_x >= margin
                                && local_x <= 1.0 - margin
                                && local_y >= margin
                                && local_y <= 1.0 - margin
                            {
                                base_color
                            } else {
                                palette.background
                            }
                        }
                    }
                }
            };

            buffer[py][px] = color;
        }
    }

    buffer
}
