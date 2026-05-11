//! # Visual Rendering
//!
//! Renders a `GlyphGrid` into SVG or PNG output. The renderer:
//!
//! 1. Derives a color palette from the DID's BLAKE3 hash
//! 2. Maps each cell's 6-bit value to visual properties (size, shape, hue)
//! 3. Renders the grid with variable-size blocks creating the distinctive
//!    pixelated-but-artistic appearance

pub mod palette;
pub mod renderer;

pub use palette::*;
pub use renderer::*;
