//! # Glyph Scanner
//!
//! Decodes a OpenAgent Glyph image back into a `did:oas` identifier.
//!
//! ## Scanning Pipeline
//!
//! 1. Locate finder patterns (three distinctive corner blocks)
//! 2. Determine orientation and perspective transform
//! 3. Sample each cell position to determine its visual properties
//! 4. Reverse the multi-channel encoding to recover 6-bit values
//! 5. Reconstruct the codeword and apply Reed-Solomon error correction
//! 6. Decode the payload to recover the DID components
//!
//! ## Current Status
//!
//! This module provides the scanning algorithm framework. Full production
//! scanning requires OpenCV or a similar computer vision library for robust
//! finder pattern detection under varied lighting and perspective conditions.
//! The current implementation works with clean, axis-aligned glyph images.

use crate::encoding::grid::{CellType, GRID_SIZE};
use crate::encoding::payload::decode_did_payload;
use crate::encoding::reed_solomon::decode_with_ecc;
// GlyphPalette is used indirectly for color analysis during scanning.
use crate::{GlyphError, Result};

/// Result of scanning a glyph image.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScanResult {
    /// The decoded DID (requires namespace lookup for full reconstruction)
    pub identifier: String,
    /// Entity kind
    pub kind: crate::EntityKind,
    /// Namespace hash (for verification against known namespaces)
    pub namespace_hash: u8,
    /// Confidence score (0.0-1.0)
    pub confidence: f64,
    /// Whether the payload passed checksum verification
    pub checksum_valid: bool,
}

impl ScanResult {
    /// Reconstruct the full DID given a namespace.
    /// Verifies the namespace against the stored hash.
    pub fn to_did(&self, namespace: &str) -> Option<String> {
        let hash = blake3::hash(namespace.as_bytes());
        if hash.as_bytes()[0] == self.namespace_hash {
            Some(format!(
                "did:oas:{}:{}:{}",
                namespace,
                self.kind.as_str(),
                self.identifier
            ))
        } else {
            None
        }
    }

    /// Try multiple known namespaces and return the first match.
    pub fn resolve_namespace(&self, candidates: &[&str]) -> Option<String> {
        for ns in candidates {
            if let Some(did) = self.to_did(ns) {
                return Some(did);
            }
        }
        None
    }
}

/// Scan a glyph from raw pixel data.
///
/// # Arguments
/// * `pixels` - RGBA pixel data
/// * `width` - Image width
/// * `height` - Image height
/// * `data_len` - Expected data payload length (before ECC)
pub fn scan_glyph(pixels: &[u8], width: u32, height: u32, data_len: usize) -> Result<ScanResult> {
    if pixels.len() != (width * height * 4) as usize {
        return Err(GlyphError::InvalidImage(
            "pixel buffer size mismatch".into(),
        ));
    }

    // Step 1: Detect grid bounds by finding finder patterns
    let bounds = detect_grid_bounds(pixels, width, height)?;

    // Step 2: Sample cell values from the grid
    let cell_values = sample_grid(pixels, width, &bounds)?;

    // Step 3: Extract data cell values in order
    let mut six_bit_values = Vec::new();
    for (row, row_values) in cell_values.iter().enumerate() {
        for (col, &val) in row_values.iter().enumerate() {
            let cell_type = crate::encoding::grid::classify_cell_pub(row, col);
            if cell_type == CellType::Data {
                six_bit_values.push(val);
            }
        }
    }

    // Step 4: Convert 6-bit values back to bytes
    let codeword = six_bit_to_bytes_scan(&six_bit_values)?;

    // Step 5: Apply ECC and decode
    let payload = decode_with_ecc(&codeword, data_len)?;
    let decoded = decode_did_payload(&payload)?;

    Ok(ScanResult {
        identifier: decoded.identifier,
        kind: decoded.kind,
        namespace_hash: decoded.namespace_hash,
        confidence: 0.95, // placeholder; real impl computes from ECC error count
        checksum_valid: true,
    })
}

/// Grid bounds detected from finder patterns.
#[derive(Debug)]
struct GridBounds {
    /// Top-left corner of the grid in pixel coordinates
    x: f64,
    y: f64,
    /// Cell size in pixels
    cell_size: f64,
}

/// Detect the grid bounds by locating finder patterns.
///
/// Strategy:
/// 1. Search for the 3x3 nested-square finder pattern at each corner using
///    brightness gradient analysis (high-contrast dark-light-dark transitions).
/// 2. If 3 finder corners are located, compute grid bounds from their positions.
/// 3. Falls back to dimension-based estimation for clean, axis-aligned images.
fn detect_grid_bounds(pixels: &[u8], width: u32, height: u32) -> Result<GridBounds> {
    // Attempt finder pattern detection via brightness analysis
    if let Some(bounds) = detect_finders_by_brightness(pixels, width, height) {
        return Ok(bounds);
    }

    // Fallback: estimate from image dimensions (axis-aligned images)
    let padding_estimate = width.min(height) as f64 * 0.05;
    let grid_extent = width.min(height) as f64 - 2.0 * padding_estimate;
    let cell_size = grid_extent / GRID_SIZE as f64;

    Ok(GridBounds {
        x: padding_estimate,
        y: padding_estimate,
        cell_size,
    })
}

/// Brightness at a pixel coordinate.
fn pixel_brightness(pixels: &[u8], width: u32, px: u32, py: u32) -> f64 {
    let idx = ((py * width + px) * 4) as usize;
    if idx + 2 < pixels.len() {
        let r = pixels[idx] as f64;
        let g = pixels[idx + 1] as f64;
        let b = pixels[idx + 2] as f64;
        0.299 * r + 0.587 * g + 0.114 * b
    } else {
        0.0
    }
}

/// Check whether a region centered at (cx, cy) with given cell_size exhibits
/// the nested-square finder pattern: dark border, light interior, dark center.
fn is_finder_pattern(
    pixels: &[u8],
    width: u32,
    height: u32,
    cx: u32,
    cy: u32,
    cell_size: f64,
) -> bool {
    let half = (cell_size * 1.5) as i32; // half of 3-cell region
    let center_r = (cell_size * 0.3) as i32;

    // Check center is bright or very dark (high intensity or low)
    if cx as i32 - half < 0 || cy as i32 - half < 0 {
        return false;
    }
    if (cx as i32 + half) as u32 >= width || (cy as i32 + half) as u32 >= height {
        return false;
    }

    let _center_brightness = pixel_brightness(pixels, width, cx, cy);

    // Sample border brightness (average of 4 border midpoints)
    let border_samples = [
        (cx as i32 - half, cy as i32),
        (cx as i32 + half, cy as i32),
        (cx as i32, cy as i32 - half),
        (cx as i32, cy as i32 + half),
    ];

    let border_brightness: f64 = border_samples
        .iter()
        .filter(|(bx, by)| *bx >= 0 && *by >= 0 && (*bx as u32) < width && (*by as u32) < height)
        .map(|(bx, by)| pixel_brightness(pixels, width, *bx as u32, *by as u32))
        .sum::<f64>()
        / 4.0;

    // Finder pattern: border and center are similar intensity (both part of the
    // dark frame), with the ring between them being lighter.
    // We check for high contrast between border region and the intermediate ring.
    let ring_x = cx as i32 + center_r;
    let ring_y = cy as i32 + center_r;
    if ring_x < 0 || ring_y < 0 || ring_x as u32 >= width || ring_y as u32 >= height {
        return false;
    }
    let ring_brightness = pixel_brightness(pixels, width, ring_x as u32, ring_y as u32);

    // The pattern has contrast: border differs from ring by at least some threshold
    let contrast = (border_brightness - ring_brightness).abs();
    contrast > 30.0
}

/// Detected finder corner position.
#[derive(Debug, Clone, Copy)]
struct FinderCorner {
    /// Center x of the 3x3 finder pattern region in pixels
    x: f64,
    /// Center y of the 3x3 finder pattern region in pixels
    y: f64,
}

/// Attempt to detect finder patterns using brightness gradient analysis.
///
/// Searches the four quadrants of the image for the nested-square pattern.
/// The glyph has finders at top-left, top-right, and bottom-right.
fn detect_finders_by_brightness(pixels: &[u8], width: u32, height: u32) -> Option<GridBounds> {
    if width < 24 || height < 24 {
        return None;
    }

    let min_dim = width.min(height) as f64;
    // Estimate cell size from image dimensions (rough starting point)
    let est_cell = min_dim / (GRID_SIZE as f64 + 2.0); // +2 for padding
    let search_margin = (min_dim * 0.35) as u32;
    let step = (est_cell * 0.5).max(2.0) as u32;

    let mut tl_corner: Option<FinderCorner> = None;
    let mut tr_corner: Option<FinderCorner> = None;
    let mut br_corner: Option<FinderCorner> = None;

    // Search top-left quadrant
    let mut best_contrast = 0.0f64;
    let mut y = step;
    while y < search_margin {
        let mut x = step;
        while x < search_margin {
            if is_finder_pattern(pixels, width, height, x, y, est_cell) {
                let c = pixel_brightness(pixels, width, x, y);
                if c > best_contrast || tl_corner.is_none() {
                    best_contrast = c;
                    tl_corner = Some(FinderCorner {
                        x: x as f64,
                        y: y as f64,
                    });
                }
            }
            x += step;
        }
        y += step;
    }

    // Search top-right quadrant
    best_contrast = 0.0;
    y = step;
    while y < search_margin {
        let mut x = width - search_margin;
        while x < width - step {
            if is_finder_pattern(pixels, width, height, x, y, est_cell) {
                let c = pixel_brightness(pixels, width, x, y);
                if c > best_contrast || tr_corner.is_none() {
                    best_contrast = c;
                    tr_corner = Some(FinderCorner {
                        x: x as f64,
                        y: y as f64,
                    });
                }
            }
            x += step;
        }
        y += step;
    }

    // Search bottom-right quadrant
    best_contrast = 0.0;
    y = height - search_margin;
    while y < height - step {
        let mut x = width - search_margin;
        while x < width - step {
            if is_finder_pattern(pixels, width, height, x, y, est_cell) {
                let c = pixel_brightness(pixels, width, x, y);
                if c > best_contrast || br_corner.is_none() {
                    best_contrast = c;
                    br_corner = Some(FinderCorner {
                        x: x as f64,
                        y: y as f64,
                    });
                }
            }
            x += step;
        }
        y += step;
    }

    // Need at least 2 corners to compute grid geometry
    let (tl, tr, _br) = match (tl_corner, tr_corner, br_corner) {
        (Some(tl), Some(tr), Some(br)) => (tl, tr, br),
        (Some(tl), Some(tr), None) => {
            // Infer BR from TL and TR
            let br = FinderCorner {
                x: tr.x,
                y: tl.y + (tr.x - tl.x), // assume square grid
            };
            (tl, tr, br)
        }
        (Some(tl), None, Some(br)) => {
            // Infer TR from TL and BR
            let tr = FinderCorner { x: br.x, y: tl.y };
            (tl, tr, br)
        }
        _ => return None,
    };

    // Compute cell size from the distance between TL and TR centers.
    // Finder patterns are 3x3 cells, so TL center is at cell (1,1) and
    // TR center is at cell (1, GRID_SIZE-2). Distance = (GRID_SIZE-3) cells.
    let dx = tr.x - tl.x;
    let cell_size = dx / (GRID_SIZE - 3) as f64;

    if cell_size < 2.0 {
        return None;
    }

    // Grid origin: TL finder center is at cell (1,1), so origin is 1 cell back
    let grid_x = tl.x - cell_size;
    let grid_y = tl.y - cell_size;

    Some(GridBounds {
        x: grid_x.max(0.0),
        y: grid_y.max(0.0),
        cell_size,
    })
}

/// Sample the grid to extract cell values.
fn sample_grid(pixels: &[u8], width: u32, bounds: &GridBounds) -> Result<Vec<Vec<u8>>> {
    let mut values = vec![vec![0u8; GRID_SIZE]; GRID_SIZE];

    for (row, row_values) in values.iter_mut().enumerate() {
        for (col, cell_val) in row_values.iter_mut().enumerate() {
            // Sample center of each cell
            let px = (bounds.x + (col as f64 + 0.5) * bounds.cell_size) as u32;
            let py = (bounds.y + (row as f64 + 0.5) * bounds.cell_size) as u32;

            if px < width && py < (pixels.len() as u32 / (width * 4)) {
                let idx = ((py * width + px) * 4) as usize;
                if idx + 3 < pixels.len() {
                    let r = pixels[idx] as f64;
                    let g = pixels[idx + 1] as f64;
                    let b = pixels[idx + 2] as f64;

                    // Map RGB to 6-bit value via luminance + hue analysis
                    let lum = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;
                    let hue_approx = approximate_hue(r, g, b);

                    // Size channel from luminance (2 bits)
                    let size_bits = ((lum * 3.99) as u8).min(3);

                    // Shape channel from local variance (placeholder: derive from hue quadrant)
                    let shape_bits = ((hue_approx / 90.0) as u8).min(3);

                    // Hue channel from hue position
                    let hue_bits = (((hue_approx % 90.0) / 22.5) as u8).min(3);

                    *cell_val = (size_bits << 4) | (shape_bits << 2) | hue_bits;
                }
            }
        }
    }

    Ok(values)
}

/// Approximate hue from RGB (0-360).
fn approximate_hue(r: f64, g: f64, b: f64) -> f64 {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    if delta < 1.0 {
        return 0.0;
    }

    let hue = if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };

    if hue < 0.0 {
        hue + 360.0
    } else {
        hue
    }
}

/// Convert 6-bit values back to bytes.
fn six_bit_to_bytes_scan(values: &[u8]) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;

    for &val in values {
        buffer = (buffer << 6) | ((val & 0x3F) as u32);
        bits += 6;
        while bits >= 8 {
            bits -= 8;
            bytes.push(((buffer >> bits) & 0xFF) as u8);
        }
    }

    Ok(bytes)
}

// Re-export classify_cell as pub for scanner use
impl crate::encoding::grid::GlyphGrid {
    // This is handled via a module-level function instead
}
