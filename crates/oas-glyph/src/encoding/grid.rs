//! # Grid Layout
//!
//! Maps encoded codeword bytes onto the 12×12 glyph grid, assigning each data
//! cell a multi-channel value (size, shape, hue_shift).
//!
//! ## Grid Regions
//!
//! - **Finder Patterns (FP)**: 3×3 blocks at three corners (TL, TR, BR) for
//!   orientation detection. Missing corner (BL) contains the Kind Region.
//! - **Kind Region (KR)**: 3×3 block at bottom-left encoding entity type visually.
//! - **Alignment Pattern (AP)**: Single cell at grid center for perspective correction.
//! - **Format Strip**: Row 3 and column 3 encode metadata (version, ECC level, data length).
//! - **Data Cells**: All remaining cells carry encoded payload data.

use crate::{EntityKind, GlyphError, Result};

/// Grid dimension (12×12).
pub const GRID_SIZE: usize = 12;

/// Total cells in the grid.
pub const TOTAL_CELLS: usize = GRID_SIZE * GRID_SIZE;

/// Cell types in the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellType {
    /// Finder pattern corner block
    Finder,
    /// Kind region (visual entity type indicator)
    Kind,
    /// Alignment pattern (center reference)
    Alignment,
    /// Format/metadata strip
    Format,
    /// Data cell (carries encoded payload)
    Data,
}

/// A single cell in the glyph grid.
#[derive(Debug, Clone, Copy)]
pub struct GlyphCell {
    pub row: usize,
    pub col: usize,
    pub cell_type: CellType,
    /// 6-bit value for data cells (0-63), split as:
    /// - bits [5:4] → block size (0-3)
    /// - bits [3:2] → shape (0-3)
    /// - bits [1:0] → hue shift (0-3)
    pub value: u8,
}

impl GlyphCell {
    pub fn block_size(&self) -> u8 {
        (self.value >> 4) & 0x03
    }

    pub fn shape(&self) -> CellShape {
        match (self.value >> 2) & 0x03 {
            0 => CellShape::Square,
            1 => CellShape::Circle,
            2 => CellShape::Diamond,
            3 => CellShape::Triangle,
            _ => unreachable!(),
        }
    }

    pub fn hue_shift(&self) -> u8 {
        self.value & 0x03
    }
}

/// Visual shapes for data cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CellShape {
    Square,
    Circle,
    Diamond,
    Triangle,
}

/// The complete glyph grid.
#[derive(Debug, Clone)]
pub struct GlyphGrid {
    pub cells: Vec<Vec<GlyphCell>>,
    pub kind: EntityKind,
    pub data_cell_count: usize,
}

impl GlyphGrid {
    /// Create a new grid and map the codeword onto data cells.
    #[allow(clippy::needless_range_loop)]
    pub fn from_codeword(codeword: &[u8], kind: EntityKind) -> Result<Self> {
        let mut cells = vec![
            vec![
                GlyphCell {
                    row: 0,
                    col: 0,
                    cell_type: CellType::Data,
                    value: 0,
                };
                GRID_SIZE
            ];
            GRID_SIZE
        ];

        // Initialize cell positions and types
        let mut data_positions = Vec::new();

        for row in 0..GRID_SIZE {
            for col in 0..GRID_SIZE {
                cells[row][col].row = row;
                cells[row][col].col = col;

                let cell_type = classify_cell(row, col);
                cells[row][col].cell_type = cell_type;

                if cell_type == CellType::Data {
                    data_positions.push((row, col));
                }
            }
        }

        // Set finder pattern values (distinctive high-contrast pattern)
        set_finder_patterns(&mut cells);

        // Set kind region
        set_kind_region(&mut cells, kind);

        // Set alignment pattern
        set_alignment_pattern(&mut cells);

        // Convert codeword bytes to 6-bit values and map onto data cells
        let six_bit_values = bytes_to_six_bit(codeword)?;

        if six_bit_values.len() > data_positions.len() {
            return Err(GlyphError::Encoding(format!(
                "codeword produces {} 6-bit values but only {} data cells available",
                six_bit_values.len(),
                data_positions.len()
            )));
        }

        for (i, &(row, col)) in data_positions.iter().enumerate() {
            if i < six_bit_values.len() {
                cells[row][col].value = six_bit_values[i];
            } else {
                // Padding cells get a value derived from position for visual interest
                cells[row][col].value = ((row * GRID_SIZE + col) % 64) as u8;
            }
        }

        Ok(Self {
            cells,
            kind,
            data_cell_count: data_positions.len(),
        })
    }

    /// Extract the codeword bytes from data cells.
    pub fn to_codeword(&self) -> Result<Vec<u8>> {
        let mut six_bit_values = Vec::new();

        for row in 0..GRID_SIZE {
            for col in 0..GRID_SIZE {
                if self.cells[row][col].cell_type == CellType::Data {
                    six_bit_values.push(self.cells[row][col].value);
                }
            }
        }

        six_bit_to_bytes(&six_bit_values)
    }

    /// Get the number of available data cells.
    pub fn capacity_bits(&self) -> usize {
        self.data_cell_count * 6
    }
}

/// Public version of classify_cell for scanner module.
pub fn classify_cell_pub(row: usize, col: usize) -> CellType {
    classify_cell(row, col)
}

/// Classify a cell position into its type.
fn classify_cell(row: usize, col: usize) -> CellType {
    // Top-left finder (3×3)
    if row < 3 && col < 3 {
        return CellType::Finder;
    }
    // Top-right finder (3×3)
    if row < 3 && col >= GRID_SIZE - 3 {
        return CellType::Finder;
    }
    // Bottom-right finder (3×3)
    if row >= GRID_SIZE - 3 && col >= GRID_SIZE - 3 {
        return CellType::Finder;
    }
    // Bottom-left kind region (3×3)
    if row >= GRID_SIZE - 3 && col < 3 {
        return CellType::Kind;
    }
    // Center alignment (single cell)
    if row == GRID_SIZE / 2 && col == GRID_SIZE / 2 {
        return CellType::Alignment;
    }
    // Format strips (row 3 and col 3, between finder corners)
    if row == 3 && (3..GRID_SIZE - 3).contains(&col) {
        return CellType::Format;
    }
    if col == 3 && (3..GRID_SIZE - 3).contains(&row) {
        return CellType::Format;
    }

    CellType::Data
}

/// Set the finder pattern cells to their distinctive values.
/// Finder patterns use a nested square motif: dark-light-dark.
fn set_finder_patterns(cells: &mut [Vec<GlyphCell>]) {
    let corners = [
        (0, 0),                         // Top-left
        (0, GRID_SIZE - 3),             // Top-right
        (GRID_SIZE - 3, GRID_SIZE - 3), // Bottom-right
    ];

    for &(sr, sc) in &corners {
        for dr in 0..3 {
            for dc in 0..3 {
                let is_border = dr == 0 || dr == 2 || dc == 0 || dc == 2;
                let is_center = dr == 1 && dc == 1;
                cells[sr + dr][sc + dc].value = if is_center {
                    0b111111 // Full intensity center
                } else if is_border {
                    0b101010 // Medium intensity border
                } else {
                    0b000000 // Background
                };
            }
        }
    }
}

/// Set the kind region cells based on entity type.
fn set_kind_region(cells: &mut [Vec<GlyphCell>], kind: EntityKind) {
    let base_row = GRID_SIZE - 3;
    let kind_val = kind as u8;

    // Encode kind as a 3×3 pattern
    // The pattern is deterministic per kind for visual recognition
    let patterns: [[u8; 3]; 3] = match kind {
        EntityKind::Hmr => [
            [0b110000, 0b111111, 0b110000],
            [0b111111, 0b111111, 0b111111],
            [0b110000, 0b111111, 0b110000],
        ],
        EntityKind::Mhr => [
            [0b111111, 0b000000, 0b111111],
            [0b000000, 0b111111, 0b000000],
            [0b111111, 0b000000, 0b111111],
        ],
        EntityKind::Ao => [
            [0b010101, 0b111111, 0b010101],
            [0b111111, 0b010101, 0b111111],
            [0b010101, 0b111111, 0b010101],
        ],
        _ => {
            // Generic pattern derived from kind value
            let v = kind_val * 17; // spread across byte
            [
                [
                    v.wrapping_mul(3) & 0x3F,
                    v.wrapping_mul(5) & 0x3F,
                    v.wrapping_mul(7) & 0x3F,
                ],
                [
                    v.wrapping_mul(11) & 0x3F,
                    v.wrapping_mul(13) & 0x3F,
                    v.wrapping_mul(17) & 0x3F,
                ],
                [
                    v.wrapping_mul(19) & 0x3F,
                    v.wrapping_mul(23) & 0x3F,
                    v.wrapping_mul(29) & 0x3F,
                ],
            ]
        }
    };

    for dr in 0..3 {
        for dc in 0..3 {
            cells[base_row + dr][dc].value = patterns[dr][dc];
        }
    }
}

/// Set the alignment pattern (center cell).
fn set_alignment_pattern(cells: &mut [Vec<GlyphCell>]) {
    let mid = GRID_SIZE / 2;
    cells[mid][mid].value = 0b111111; // Maximum contrast
}

/// Convert a byte slice into a vector of 6-bit values.
///
/// Every 3 bytes (24 bits) map to 4 six-bit values.
fn bytes_to_six_bit(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut values = Vec::with_capacity((bytes.len() * 8).div_ceil(6));
    let mut bit_buffer: u32 = 0;
    let mut bits_in_buffer: u32 = 0;

    for &byte in bytes {
        bit_buffer = (bit_buffer << 8) | (byte as u32);
        bits_in_buffer += 8;

        while bits_in_buffer >= 6 {
            bits_in_buffer -= 6;
            let val = (bit_buffer >> bits_in_buffer) & 0x3F;
            values.push(val as u8);
        }
    }

    // Flush remaining bits (padded with zeros)
    if bits_in_buffer > 0 {
        let val = (bit_buffer << (6 - bits_in_buffer)) & 0x3F;
        values.push(val as u8);
    }

    Ok(values)
}

/// Convert a vector of 6-bit values back to bytes.
fn six_bit_to_bytes(values: &[u8]) -> Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity((values.len() * 6) / 8);
    let mut bit_buffer: u32 = 0;
    let mut bits_in_buffer: u32 = 0;

    for &val in values {
        if val > 63 {
            return Err(GlyphError::Decoding(format!(
                "six-bit value out of range: {}",
                val
            )));
        }
        bit_buffer = (bit_buffer << 6) | (val as u32);
        bits_in_buffer += 6;

        while bits_in_buffer >= 8 {
            bits_in_buffer -= 8;
            let byte = (bit_buffer >> bits_in_buffer) & 0xFF;
            bytes.push(byte as u8);
        }
    }

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_bit_roundtrip() {
        let original = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x42, 0x13];
        let six_bit = bytes_to_six_bit(&original).unwrap();
        let recovered = six_bit_to_bytes(&six_bit).unwrap();
        assert_eq!(&recovered[..original.len()], &original);
    }

    #[test]
    fn data_cell_count() {
        let dummy_codeword = vec![0u8; 40];
        let grid = GlyphGrid::from_codeword(&dummy_codeword, EntityKind::Agent).unwrap();
        // 144 total - 27 finder - 9 kind - 1 alignment - ~12 format = ~95 data cells
        assert!(grid.data_cell_count > 80, "got {}", grid.data_cell_count);
    }

    #[test]
    fn grid_cell_classification() {
        assert_eq!(classify_cell(0, 0), CellType::Finder);
        assert_eq!(classify_cell(0, 11), CellType::Finder);
        assert_eq!(classify_cell(11, 11), CellType::Finder);
        assert_eq!(classify_cell(11, 0), CellType::Kind);
        assert_eq!(classify_cell(6, 6), CellType::Alignment);
        assert_eq!(classify_cell(3, 5), CellType::Format);
        assert_eq!(classify_cell(5, 5), CellType::Data);
    }

    #[test]
    fn grid_is_12x12() {
        assert_eq!(GRID_SIZE, 12);
        assert_eq!(TOTAL_CELLS, 144);
    }

    #[test]
    fn grid_dimensions_match() {
        let dummy = vec![0u8; 40];
        let grid = GlyphGrid::from_codeword(&dummy, EntityKind::Agent).unwrap();
        assert_eq!(grid.cells.len(), GRID_SIZE);
        for row in &grid.cells {
            assert_eq!(row.len(), GRID_SIZE);
        }
    }

    #[test]
    fn finder_patterns_at_three_corners() {
        // Top-left 3x3
        for r in 0..3 {
            for c in 0..3 {
                assert_eq!(classify_cell(r, c), CellType::Finder, "TL ({}, {})", r, c);
            }
        }
        // Top-right 3x3
        for r in 0..3 {
            for c in 9..12 {
                assert_eq!(classify_cell(r, c), CellType::Finder, "TR ({}, {})", r, c);
            }
        }
        // Bottom-right 3x3
        for r in 9..12 {
            for c in 9..12 {
                assert_eq!(classify_cell(r, c), CellType::Finder, "BR ({}, {})", r, c);
            }
        }
    }

    #[test]
    fn kind_region_at_bottom_left() {
        for r in 9..12 {
            for c in 0..3 {
                assert_eq!(classify_cell(r, c), CellType::Kind, "KR ({}, {})", r, c);
            }
        }
    }

    #[test]
    fn alignment_at_center() {
        assert_eq!(classify_cell(6, 6), CellType::Alignment);
    }

    #[test]
    fn data_cell_count_invariant() {
        // Count data cells by enumerating all positions
        let mut data_count = 0;
        let mut finder_count = 0;
        let mut kind_count = 0;
        let mut alignment_count = 0;
        let mut format_count = 0;
        for r in 0..GRID_SIZE {
            for c in 0..GRID_SIZE {
                match classify_cell(r, c) {
                    CellType::Data => data_count += 1,
                    CellType::Finder => finder_count += 1,
                    CellType::Kind => kind_count += 1,
                    CellType::Alignment => alignment_count += 1,
                    CellType::Format => format_count += 1,
                }
            }
        }
        assert_eq!(finder_count, 27, "3 corners * 3x3 = 27 finder cells");
        assert_eq!(kind_count, 9, "1 corner * 3x3 = 9 kind cells");
        assert_eq!(alignment_count, 1, "single center alignment cell");
        assert_eq!(
            data_count + finder_count + kind_count + alignment_count + format_count,
            TOTAL_CELLS,
            "all cells accounted for"
        );
        // Verify the grid agrees with enumeration
        let dummy = vec![0u8; 40];
        let grid = GlyphGrid::from_codeword(&dummy, EntityKind::Agent).unwrap();
        assert_eq!(grid.data_cell_count, data_count);
    }

    #[test]
    fn six_bit_values_all_in_range() {
        let data = vec![0xFF; 20];
        let six_bit = bytes_to_six_bit(&data).unwrap();
        for &v in &six_bit {
            assert!(v <= 63, "six-bit value {} > 63", v);
        }
    }

    #[test]
    fn six_bit_rejects_out_of_range() {
        let bad_values = vec![64u8]; // 64 is out of 6-bit range
        assert!(six_bit_to_bytes(&bad_values).is_err());
    }

    #[test]
    fn capacity_bits_consistent() {
        let dummy = vec![0u8; 40];
        let grid = GlyphGrid::from_codeword(&dummy, EntityKind::Hmr).unwrap();
        assert_eq!(grid.capacity_bits(), grid.data_cell_count * 6);
    }

    #[test]
    fn cell_accessors() {
        let cell = GlyphCell {
            row: 0,
            col: 0,
            cell_type: CellType::Data,
            value: 0b110100, // size=3, shape=1(circle), hue=0
        };
        assert_eq!(cell.block_size(), 3);
        assert_eq!(cell.shape(), CellShape::Circle);
        assert_eq!(cell.hue_shift(), 0);
    }

    #[test]
    fn cell_shape_all_variants() {
        for shape_bits in 0..4u8 {
            let cell = GlyphCell {
                row: 0,
                col: 0,
                cell_type: CellType::Data,
                value: shape_bits << 2,
            };
            let shape = cell.shape();
            match shape_bits {
                0 => assert_eq!(shape, CellShape::Square),
                1 => assert_eq!(shape, CellShape::Circle),
                2 => assert_eq!(shape, CellShape::Diamond),
                3 => assert_eq!(shape, CellShape::Triangle),
                _ => unreachable!(),
            }
        }
    }
}
