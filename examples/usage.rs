//! # OpenAgent Glyph Usage Examples
//!
//! ```rust
//! use oas_glyph::{ParsedDid, EntityKind};
//! use oas_glyph::encoding::{encode_did_payload, encode_with_ecc, GlyphGrid};
//! use oas_glyph::visual::{GlyphPalette, did_to_svg, did_to_png, RenderConfig};
//!
//! fn main() {
//!     // === Generate a glyph from a DID ===
//!
//!     let did = "did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";
//!
//!     // One-liner: DID → SVG
//!     let svg = did_to_svg(did, None).unwrap();
//!     std::fs::write("agent-glyph.svg", &svg).unwrap();
//!
//!     // One-liner: DID → PNG (1024px)
//!     let png = did_to_png(did, 1024).unwrap();
//!     std::fs::write("agent-glyph.png", &png).unwrap();
//!
//!     // Custom render config
//!     let config = RenderConfig {
//!         size: 256,
//!         padding: 12,
//!         corner_radius: 8,
//!         include_background: true,
//!         cell_gap: 1.0,
//!     };
//!     let svg_small = did_to_svg(did, Some(config)).unwrap();
//!
//!
//!     // === Inspect the palette ===
//!
//!     let palette = GlyphPalette::from_did(did);
//!     println!("Primary color: {}", palette.primary.to_hex());
//!     println!("Secondary color: {}", palette.secondary.to_hex());
//!     for (i, c) in palette.data_colors.iter().enumerate() {
//!         println!("Data color {}: {}", i, c.to_hex());
//!     }
//!
//!
//!     // === Parse DID components ===
//!
//!     let parsed = ParsedDid::parse(did).unwrap();
//!     println!("Namespace: {}", parsed.namespace);
//!     println!("Kind: {:?}", parsed.kind);
//!     println!("Identifier: {}", parsed.identifier);
//!
//!
//!     // === Step-by-step encoding (for debugging) ===
//!
//!     let payload = encode_did_payload(&parsed).unwrap();
//!     println!("Raw payload: {} bytes", payload.len());
//!
//!     let codeword = encode_with_ecc(&payload).unwrap();
//!     println!("With ECC: {} bytes", codeword.len());
//!
//!     let grid = GlyphGrid::from_codeword(&codeword, parsed.kind).unwrap();
//!     println!("Data cells: {}", grid.data_cell_count);
//!     println!("Capacity: {} bits", grid.capacity_bits());
//!
//!
//!     // === Generate glyphs for different entity types ===
//!
//!     let entities = vec![
//!         ("did:oas:l1fe:hmr:z6MkiTBz1ymuepAQ4HEHYSF1H8quG5GLVVQR3djdX3mDooWp", "Human Root"),
//!         ("did:oas:l1fe:mhr:z6Mkf7HKpGqNJ3WSX2fVoEQssra1GFqMaXP8ePaUP5RLKnmV", "Multi-Human Root"),
//!         ("did:oas:l1fe:ao:z6MkrHKpJqt4r9DH1A3KQGB2tNzEd7XVxoV6r2ycqQG9FkZN", "Autonomous Org"),
//!         ("did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK", "Agent"),
//!         ("did:oas:l1fe:tool:z6MknGc3ocHs3zdPiJbnaaqDi5RNw3A6tDfHPbp7ihKBpKG3", "Tool"),
//!         ("did:oas:l1fe:service:z6Mkp9Juh5gHzRdNYW2gcCRZPW2iB2rvwyVU7VHtohmBCH8z", "Service"),
//!     ];
//!
//!     for (did, label) in entities {
//!         let svg = did_to_svg(did, None).unwrap();
//!         let filename = format!("glyph-{}.svg", label.to_lowercase().replace(' ', "-"));
//!         std::fs::write(&filename, &svg).unwrap();
//!         println!("Generated: {}", filename);
//!     }
//!
//!
//!     // === Run the API server ===
//!     // 
//!     // Start:  cargo run
//!     // 
//!     // Then:
//!     //   curl -X POST http://localhost:3377/v1/glyph/generate \
//!     //     -H 'Content-Type: application/json' \
//!     //     -d '{"did":"did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK"}'
//!     //
//!     //   curl 'http://localhost:3377/v1/glyph/svg/did%3Aoas%3Al1fe%3Aagent%3Az6Mkh...'
//!     //
//!     //   curl 'http://localhost:3377/v1/glyph/png/did%3Aoas%3Al1fe%3Aagent%3Az6Mkh...?size=1024' > glyph.png
//! }
//! ```

fn main() {
    println!("See doc comments above for usage examples.");
    println!("Run the API server with: cargo run");
    println!("Or use the library directly in your Rust code.");
}
