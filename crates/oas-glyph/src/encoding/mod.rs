//! # Glyph Encoding
//!
//! Transforms a `did:oas` identifier into a bit-level payload that maps onto the
//! visual grid. The encoding pipeline:
//!
//! 1. Parse the DID into structured components
//! 2. Encode namespace (hash to 8 bits), kind (4 bits), identifier (variable)
//! 3. Apply Reed-Solomon error correction (30% redundancy)
//! 4. Map corrected codewords onto the grid's data cells
//! 5. Split each cell's value into three visual channels (size, shape, hue)

pub mod grid;
pub mod payload;
pub mod reed_solomon;

pub use grid::*;
pub use payload::*;
pub use reed_solomon::*;
