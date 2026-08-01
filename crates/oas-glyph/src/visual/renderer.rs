//! # SVG Renderer
//!
//! Renders a `GlyphGrid` + `GlyphPalette` into an SVG string.
//! Each cell is drawn as a variable-size block with the appropriate shape.

use crate::encoding::grid::{CellShape, CellType, GlyphGrid, GRID_SIZE};
use crate::visual::palette::{Color, GlyphPalette};
use crate::Result;

/// Render configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RenderConfig {
    /// Total image size in pixels (square).
    pub size: u32,
    /// Padding around the grid (pixels).
    pub padding: u32,
    /// Corner radius for the outer frame (0 = sharp corners).
    pub corner_radius: u32,
    /// Whether to include the background rectangle.
    pub include_background: bool,
    /// Gap between cells (pixels).
    pub cell_gap: f64,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            size: 512,
            padding: 24,
            corner_radius: 16,
            include_background: true,
            cell_gap: 1.5,
        }
    }
}

/// Render a glyph grid to SVG.
pub fn render_svg(
    grid: &GlyphGrid,
    palette: &GlyphPalette,
    config: &RenderConfig,
) -> Result<String> {
    let inner_size = config.size - 2 * config.padding;
    let cell_size = inner_size as f64 / GRID_SIZE as f64;

    let mut svg = String::with_capacity(16384);

    // SVG header
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="{}" height="{}">"#,
        config.size, config.size, config.size, config.size
    ));

    // Defs for reusable elements
    svg.push_str("<defs>");
    // Drop shadow filter
    svg.push_str(r#"<filter id="glow"><feGaussianBlur stdDeviation="2" result="blur"/><feMerge><feMergeNode in="blur"/><feMergeNode in="SourceGraphic"/></feMerge></filter>"#);
    svg.push_str("</defs>");

    // Background
    if config.include_background {
        svg.push_str(&format!(
            r#"<rect x="0" y="0" width="{}" height="{}" rx="{}" fill="{}"/>"#,
            config.size,
            config.size,
            config.corner_radius,
            palette.background.to_hex()
        ));
    }

    // Render each cell
    for row in 0..GRID_SIZE {
        for col in 0..GRID_SIZE {
            let cell = &grid.cells[row][col];
            let x = config.padding as f64 + col as f64 * cell_size;
            let y = config.padding as f64 + row as f64 * cell_size;

            match cell.cell_type {
                CellType::Finder => {
                    render_finder_cell(
                        &mut svg,
                        x,
                        y,
                        cell_size,
                        config.cell_gap,
                        cell.value,
                        palette,
                    );
                }
                CellType::Kind => {
                    render_kind_cell(
                        &mut svg,
                        x,
                        y,
                        cell_size,
                        config.cell_gap,
                        cell.value,
                        palette,
                    );
                }
                CellType::Alignment => {
                    render_alignment_cell(&mut svg, x, y, cell_size, palette);
                }
                CellType::Format => {
                    render_format_cell(
                        &mut svg,
                        x,
                        y,
                        cell_size,
                        config.cell_gap,
                        cell.value,
                        palette,
                    );
                }
                CellType::Data => {
                    render_data_cell(&mut svg, x, y, cell_size, config.cell_gap, cell, palette);
                }
            }
        }
    }

    svg.push_str("</svg>");
    Ok(svg)
}

/// Render a finder pattern cell.
fn render_finder_cell(
    svg: &mut String,
    x: f64,
    y: f64,
    size: f64,
    gap: f64,
    value: u8,
    palette: &GlyphPalette,
) {
    let is_dark = value > 32;
    let color = if is_dark {
        &palette.finder_dark
    } else {
        &palette.finder_light
    };
    let inset = gap / 2.0;
    svg.push_str(&format!(
        r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="2" fill="{}"/>"#,
        x + inset,
        y + inset,
        size - gap,
        size - gap,
        color.to_hex()
    ));
}

/// Render a kind region cell.
fn render_kind_cell(
    svg: &mut String,
    x: f64,
    y: f64,
    size: f64,
    gap: f64,
    value: u8,
    palette: &GlyphPalette,
) {
    let intensity = value as f64 / 63.0;
    let color = Color::lerp(&palette.background, &palette.kind_accent, intensity);
    let inset = gap / 2.0;
    svg.push_str(&format!(
        r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="1" fill="{}"/>"#,
        x + inset,
        y + inset,
        size - gap,
        size - gap,
        color.to_hex()
    ));
}

/// Render the alignment pattern cell.
fn render_alignment_cell(svg: &mut String, x: f64, y: f64, size: f64, palette: &GlyphPalette) {
    let cx = x + size / 2.0;
    let cy = y + size / 2.0;
    let r = size / 2.0 - 1.0;
    // Concentric circles
    svg.push_str(&format!(
        r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="{}"/>"#,
        cx,
        cy,
        r,
        palette.finder_light.to_hex()
    ));
    svg.push_str(&format!(
        r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="{}"/>"#,
        cx,
        cy,
        r * 0.5,
        palette.finder_dark.to_hex()
    ));
}

/// Render a format strip cell.
fn render_format_cell(
    svg: &mut String,
    x: f64,
    y: f64,
    size: f64,
    gap: f64,
    value: u8,
    palette: &GlyphPalette,
) {
    let intensity = value as f64 / 63.0;
    let color = Color::lerp(&palette.background, &palette.secondary, intensity * 0.5);
    let inset = gap / 2.0;
    svg.push_str(&format!(
        r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="{}"/>"#,
        x + inset,
        y + inset,
        size - gap,
        size - gap,
        color.to_hex()
    ));
}

/// Render a data cell with variable size, shape, and color.
fn render_data_cell(
    svg: &mut String,
    x: f64,
    y: f64,
    cell_size: f64,
    _gap: f64,
    cell: &crate::encoding::grid::GlyphCell,
    palette: &GlyphPalette,
) {
    let hue_idx = cell.hue_shift() as usize;
    let color = &palette.data_colors[hue_idx % 4];

    // Block size: 0=25%, 1=50%, 2=75%, 3=100% of cell
    let size_fraction = match cell.block_size() {
        0 => 0.35,
        1 => 0.55,
        2 => 0.75,
        3 => 0.95,
        _ => 0.55,
    };
    let block_size = cell_size * size_fraction;
    let offset = (cell_size - block_size) / 2.0;

    let bx = x + offset;
    let by = y + offset;
    let cx = x + cell_size / 2.0;
    let cy = y + cell_size / 2.0;

    match cell.shape() {
        CellShape::Square => {
            let r = (block_size * 0.1).max(1.0);
            svg.push_str(&format!(
                r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="{:.1}" fill="{}"/>"#,
                bx,
                by,
                block_size,
                block_size,
                r,
                color.to_hex()
            ));
        }
        CellShape::Circle => {
            let r = block_size / 2.0;
            svg.push_str(&format!(
                r#"<circle cx="{:.1}" cy="{:.1}" r="{:.1}" fill="{}"/>"#,
                cx,
                cy,
                r,
                color.to_hex()
            ));
        }
        CellShape::Diamond => {
            let half = block_size / 2.0;
            svg.push_str(&format!(
                r#"<polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" fill="{}"/>"#,
                cx,
                cy - half, // top
                cx + half,
                cy, // right
                cx,
                cy + half, // bottom
                cx - half,
                cy, // left
                color.to_hex()
            ));
        }
        CellShape::Triangle => {
            let half = block_size / 2.0;
            svg.push_str(&format!(
                r#"<polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" fill="{}"/>"#,
                cx,
                cy - half, // top
                cx + half,
                cy + half * 0.7, // bottom-right
                cx - half,
                cy + half * 0.7, // bottom-left
                color.to_hex()
            ));
        }
    }
}

/// Convenience function: DID string → SVG string.
pub fn did_to_svg(did: &str, config: Option<RenderConfig>) -> Result<String> {
    let config = config.unwrap_or_default();
    let parsed = crate::ParsedDid::parse(did)?;
    let payload = crate::encoding::payload::encode_did_payload(&parsed)?;
    let codeword = crate::encoding::reed_solomon::encode_with_ecc(&payload)?;
    let grid = crate::encoding::grid::GlyphGrid::from_codeword(&codeword, parsed.kind)?;
    let palette = GlyphPalette::from_did(did);
    render_svg(&grid, &palette, &config)
}

/// Render to PNG bytes via resvg.
pub fn did_to_png(did: &str, size: u32) -> Result<Vec<u8>> {
    let config = RenderConfig {
        size,
        ..RenderConfig::default()
    };
    let svg_str = did_to_svg(did, Some(config))?;

    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(&svg_str, &options)
        .map_err(|e| crate::GlyphError::Render(format!("SVG parse: {}", e)))?;

    let px_size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(px_size.width(), px_size.height())
        .ok_or_else(|| crate::GlyphError::Render("failed to create pixmap".into()))?;

    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
    );

    pixmap
        .encode_png()
        .map_err(|e| crate::GlyphError::Render(format!("PNG encode: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_DID: &str = "did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";

    #[test]
    fn generates_valid_svg() {
        let svg = did_to_svg(TEST_DID, None).unwrap();
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        assert!(svg.contains("rect")); // Has rectangles
        assert!(svg.contains("circle")); // Has circles
    }

    #[test]
    fn svg_contains_finder_pattern_elements() {
        let svg = did_to_svg(TEST_DID, None).unwrap();
        // Finder patterns produce rect elements with rx="2"
        assert!(
            svg.contains(r#"rx="2""#),
            "SVG should contain finder pattern rects"
        );
    }

    #[test]
    fn svg_contains_alignment_circle() {
        let svg = did_to_svg(TEST_DID, None).unwrap();
        // Alignment pattern is rendered as concentric circles
        let circle_count = svg.matches("<circle").count();
        assert!(
            circle_count >= 2,
            "SVG should contain alignment circles, found {}",
            circle_count
        );
    }

    #[test]
    fn svg_contains_polygon_shapes() {
        let svg = did_to_svg(TEST_DID, None).unwrap();
        // Diamond and triangle shapes use polygon
        assert!(
            svg.contains("<polygon"),
            "SVG should contain polygon elements for diamonds/triangles"
        );
    }

    #[test]
    fn svg_has_viewbox() {
        let svg = did_to_svg(TEST_DID, None).unwrap();
        assert!(svg.contains("viewBox"), "SVG must have a viewBox attribute");
    }

    #[test]
    fn svg_has_background_rect() {
        let svg = did_to_svg(TEST_DID, None).unwrap();
        // Default config includes background
        assert!(
            svg.contains(r#"x="0" y="0""#),
            "SVG should have background rect at origin"
        );
    }

    #[test]
    fn png_output_valid_magic_bytes() {
        let png = did_to_png(TEST_DID, 128).unwrap();
        // PNG magic bytes: 0x89 0x50 0x4E 0x47 0x0D 0x0A 0x1A 0x0A
        assert!(png.len() > 8, "PNG output too short");
        assert_eq!(png[0], 0x89, "PNG magic byte 0");
        assert_eq!(png[1], 0x50, "PNG magic byte 1 (P)");
        assert_eq!(png[2], 0x4E, "PNG magic byte 2 (N)");
        assert_eq!(png[3], 0x47, "PNG magic byte 3 (G)");
    }

    #[test]
    fn png_output_non_trivial_size() {
        let png = did_to_png(TEST_DID, 256).unwrap();
        // A 256px glyph should produce at least a few KB
        assert!(png.len() > 1000, "PNG too small: {} bytes", png.len());
    }

    #[test]
    fn custom_render_config() {
        let config = RenderConfig {
            size: 256,
            padding: 8,
            corner_radius: 0,
            include_background: false,
            cell_gap: 0.0,
        };
        let svg = did_to_svg(TEST_DID, Some(config)).unwrap();
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains(r#"width="256""#));
    }

    #[test]
    fn different_dids_produce_different_svg() {
        let svg1 = did_to_svg(TEST_DID, None).unwrap();
        let svg2 = did_to_svg(
            "did:oas:l1fe:hmr:z6MkiTBz1ymuepAQ4HEHYSF1H8quG5GLVVQR3djdX3mDooWp",
            None,
        )
        .unwrap();
        assert_ne!(svg1, svg2, "different DIDs must produce different SVGs");
    }

    #[test]
    fn invalid_did_returns_error() {
        let result = did_to_svg("not-a-did", None);
        assert!(result.is_err());
    }
}
