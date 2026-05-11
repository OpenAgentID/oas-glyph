//! # Ratatui Glyph Widget
//!
//! A `ratatui::widgets::Widget` implementation that renders a OpenAgent Glyph
//! within any ratatui-based TUI application. Drop it into a layout alongside
//! tables, charts, and forms — the glyph is a first-class UI component.
//!
//! ## Usage
//!
//! ```rust
//! use oas_glyph_tui::widget::GlyphWidget;
//! use ratatui::Frame;
//!
//! fn render(frame: &mut Frame) {
//!     let did = "did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";
//!     let glyph = GlyphWidget::new(did).unwrap();
//!     frame.render_widget(glyph, frame.area());
//! }
//! ```

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color as RColor, Style},
    widgets::Widget,
};
use oas_glyph::encoding::grid::GlyphGrid;
use oas_glyph::visual::palette::{Color, GlyphPalette};

/// A ratatui widget that renders a OpenAgent Glyph.
pub struct GlyphWidget {
    grid: GlyphGrid,
    palette: GlyphPalette,
    did: String,
    show_border: bool,
    show_label: bool,
}

impl GlyphWidget {
    /// Create a new glyph widget from a DID string.
    pub fn new(did: &str) -> crate::Result<Self> {
        let parsed = oas_glyph::ParsedDid::parse(did)?;
        let payload = oas_glyph::encoding::encode_did_payload(&parsed)?;
        let codeword = oas_glyph::encoding::encode_with_ecc(&payload)?;
        let grid = GlyphGrid::from_codeword(&codeword, parsed.kind)?;
        let palette = GlyphPalette::from_did(did);

        Ok(Self {
            grid,
            palette,
            did: did.to_string(),
            show_border: true,
            show_label: true,
        })
    }

    pub fn border(mut self, show: bool) -> Self {
        self.show_border = show;
        self
    }

    pub fn label(mut self, show: bool) -> Self {
        self.show_label = show;
        self
    }
}

impl Widget for GlyphWidget {
    #[allow(clippy::needless_range_loop)]
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 {
            return; // Too small to render
        }

        // Reserve space for label if enabled
        let glyph_area = if self.show_label && area.height > 4 {
            Rect {
                x: area.x,
                y: area.y,
                width: area.width,
                height: area.height - 2, // 2 rows for label + kind
            }
        } else {
            area
        };

        // Each terminal cell covers 1 column × 2 pixel rows (using half-block)
        let avail_cols = glyph_area.width as usize;
        let avail_rows = glyph_area.height as usize;

        // Make it square-ish (accounting for 2:1 char cell aspect ratio)
        let glyph_cols = avail_cols.min(avail_rows * 2).min(48);
        let glyph_rows = glyph_cols / 2;

        // Center horizontally
        let x_offset = (avail_cols - glyph_cols) / 2;

        // Build the color buffer
        let pixel_rows = glyph_rows * 2; // Half-block: 2 pixels per char row
        let color_buf = crate::render::grid_to_color_buffer(
            &self.grid, &self.palette, glyph_cols, pixel_rows,
        );

        // Render using half-block technique
        for row in 0..glyph_rows {
            let buf_y = glyph_area.y + row as u16;
            if buf_y >= glyph_area.y + glyph_area.height {
                break;
            }

            for col in 0..glyph_cols {
                let buf_x = glyph_area.x + x_offset as u16 + col as u16;
                if buf_x >= glyph_area.x + glyph_area.width {
                    break;
                }

                let top = &color_buf[row * 2][col];
                let bottom = if row * 2 + 1 < pixel_rows {
                    &color_buf[row * 2 + 1][col]
                } else {
                    &self.palette.background
                };

                let cell = buf.cell_mut((buf_x, buf_y));
                if let Some(cell) = cell {
                    cell.set_char('▀');
                    cell.set_style(Style::default()
                        .fg(to_ratatui_color(top))
                        .bg(to_ratatui_color(bottom)));
                }
            }
        }

        // Render label below glyph
        if self.show_label {
            let label_y = glyph_area.y + glyph_rows as u16;
            if label_y < area.y + area.height {
                // Truncate DID for display
                let display_did = if self.did.len() > avail_cols {
                    format!("{}…{}", &self.did[..16], &self.did[self.did.len()-12..])
                } else {
                    self.did.clone()
                };

                let x_start = area.x + ((avail_cols - display_did.len().min(avail_cols)) / 2) as u16;
                buf.set_string(
                    x_start,
                    label_y,
                    &display_did,
                    Style::default().fg(RColor::DarkGray),
                );
            }

            // Kind badge on next line
            let kind_y = label_y + 1;
            if kind_y < area.y + area.height {
                if let Ok(parsed) = oas_glyph::ParsedDid::parse(&self.did) {
                    let kind_str = parsed.kind.as_str().to_uppercase();
                    let accent = &self.palette.kind_accent;
                    let badge = format!(" {} ", kind_str);
                    let x_start = area.x + ((avail_cols - badge.len().min(avail_cols)) / 2) as u16;
                    buf.set_string(
                        x_start,
                        kind_y,
                        &badge,
                        Style::default()
                            .fg(RColor::Black)
                            .bg(to_ratatui_color(accent)),
                    );
                }
            }
        }
    }
}

/// Convert our Color to ratatui Color.
fn to_ratatui_color(c: &Color) -> RColor {
    RColor::Rgb(c.r, c.g, c.b)
}

/// A compact inline glyph widget that renders in a single line.
/// Useful for table cells, status bars, and inline entity references.
pub struct CompactGlyphWidget {
    grid: GlyphGrid,
    palette: GlyphPalette,
}

impl CompactGlyphWidget {
    pub fn new(did: &str) -> crate::Result<Self> {
        let parsed = oas_glyph::ParsedDid::parse(did)?;
        let payload = oas_glyph::encoding::encode_did_payload(&parsed)?;
        let codeword = oas_glyph::encoding::encode_with_ecc(&payload)?;
        let grid = GlyphGrid::from_codeword(&codeword, parsed.kind)?;
        let palette = GlyphPalette::from_did(did);
        Ok(Self { grid, palette })
    }
}

#[allow(clippy::needless_range_loop)]
impl Widget for CompactGlyphWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 3 || area.height < 1 {
            return;
        }

        // Render a single-line glyph: 6 half-block characters
        let cols = 6usize.min(area.width as usize);
        let color_buf = crate::render::grid_to_color_buffer(
            &self.grid, &self.palette, cols, 2,
        );

        for col in 0..cols {
            let top = &color_buf[0][col];
            let bottom = &color_buf[1][col];

            let cell = buf.cell_mut((area.x + col as u16, area.y));
            if let Some(cell) = cell {
                cell.set_char('▀');
                cell.set_style(Style::default()
                    .fg(to_ratatui_color(top))
                    .bg(to_ratatui_color(bottom)));
            }
        }
    }
}
