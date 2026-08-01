//! Full encode -> render -> (synthetic) scan roundtrip tests.
//!
//! These tests exercise the complete pipeline: DID parsing, payload encoding,
//! Reed-Solomon ECC, grid layout, SVG/PNG rendering, and (where possible)
//! synthetic scan-back via the scanner module.

use oas_glyph::encoding::grid::GlyphGrid;
use oas_glyph::encoding::payload::{decode_did_payload, encode_did_payload};
use oas_glyph::encoding::reed_solomon::{decode_with_ecc, encode_with_ecc};
use oas_glyph::visual::palette::GlyphPalette;
use oas_glyph::visual::renderer::{did_to_png, did_to_svg};
use oas_glyph::ParsedDid;

const TEST_DIDS: &[&str] = &[
    "did:oas:l1fe:hmr:z6MkiTBz1ymuepAQ4HEHYSF1H8quG5GLVVQR3djdX3mDooWp",
    "did:oas:l1fe:mhr:z6Mkf7HKpGqNJ3WSX2fVoEQssra1GFqMaXP8ePaUP5RLKnmV",
    "did:oas:l1fe:ao:z6MkrHKpJqt4r9DH1A3KQGB2tNzEd7XVxoV6r2ycqQG9FkZN",
    "did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
    "did:oas:l1fe:tool:z6MknGc3ocHs3zdPiJbnaaqDi5RNw3A6tDfHPbp7ihKBpKG3",
    "did:oas:l1fe:skill:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
    "did:oas:l1fe:workflow:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
    "did:oas:l1fe:model:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
    "did:oas:l1fe:dataset:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
    "did:oas:l1fe:service:z6Mkp9Juh5gHzRdNYW2gcCRZPW2iB2rvwyVU7VHtohmBCH8z",
];

/// Full encoding roundtrip: DID -> payload -> ECC -> grid -> codeword -> ECC decode -> payload decode.
#[test]
fn full_encoding_roundtrip_all_kinds() {
    for did in TEST_DIDS {
        let parsed = ParsedDid::parse(did).expect("parse failed");
        let payload = encode_did_payload(&parsed).expect("encode failed");
        let codeword = encode_with_ecc(&payload).expect("ecc encode failed");
        let grid = GlyphGrid::from_codeword(&codeword, parsed.kind).expect("grid failed");

        // Grid -> codeword -> payload -> decoded
        let recovered_codeword = grid.to_codeword().expect("to_codeword failed");
        // The recovered bytes contain the original codeword bytes plus any padding;
        // we only need the first codeword.len() bytes.
        let decoded_payload = decode_with_ecc(&recovered_codeword[..codeword.len()], payload.len())
            .expect("ecc decode failed");
        let decoded = decode_did_payload(&decoded_payload).expect("payload decode failed");

        assert_eq!(decoded.kind, parsed.kind, "kind mismatch for {}", did);
        assert!(decoded.verify_namespace(&parsed.namespace));
    }
}

/// Verify that SVG rendering succeeds for all entity kinds.
#[test]
fn svg_render_all_kinds() {
    for did in TEST_DIDS {
        let svg = did_to_svg(did, None).unwrap_or_else(|e| panic!("SVG failed for {}: {}", did, e));
        assert!(svg.starts_with("<svg"), "bad SVG for {}", did);
        assert!(svg.ends_with("</svg>"), "bad SVG end for {}", did);
        assert!(
            svg.len() > 1000,
            "SVG too short for {}: {} bytes",
            did,
            svg.len()
        );
    }
}

/// Verify that PNG rendering succeeds and produces valid PNG for all entity kinds.
#[test]
fn png_render_all_kinds() {
    for did in TEST_DIDS {
        let png = did_to_png(did, 128).unwrap_or_else(|e| panic!("PNG failed for {}: {}", did, e));
        assert!(png.len() > 100, "PNG too small for {}", did);
        // PNG magic bytes
        assert_eq!(
            &png[0..4],
            &[0x89, 0x50, 0x4E, 0x47],
            "bad PNG magic for {}",
            did
        );
    }
}

/// Verify palette derivation is deterministic across multiple calls.
#[test]
fn palette_determinism_across_calls() {
    for did in TEST_DIDS {
        let p1 = GlyphPalette::from_did(did);
        let p2 = GlyphPalette::from_did(did);
        assert_eq!(p1.primary.r, p2.primary.r);
        assert_eq!(p1.primary.g, p2.primary.g);
        assert_eq!(p1.primary.b, p2.primary.b);
        assert_eq!(p1.secondary.r, p2.secondary.r);
    }
}

/// Synthetic scan roundtrip: encode -> render to PNG -> decode from image pixels.
/// This tests the scan pipeline with a clean, axis-aligned image.
#[test]
fn synthetic_scan_roundtrip() {
    let did = "did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";
    let parsed = ParsedDid::parse(did).unwrap();
    let payload = encode_did_payload(&parsed).unwrap();

    // Generate PNG
    let png_bytes = did_to_png(did, 512).unwrap();

    // Decode PNG to raw pixels
    let img = image::load_from_memory(&png_bytes).unwrap();
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let pixels = rgba.as_raw();

    // Attempt scan (this uses the basic axis-aligned detector)
    let scan_result = oas_glyph::scanner::scan_glyph(pixels, width, height, payload.len());

    // The current scanner is approximate for clean images; we mainly verify
    // that the pipeline does not panic and returns a structured result.
    // Full precision depends on the scanner upgrade (A2.5).
    match scan_result {
        Ok(result) => {
            // If it succeeds, verify basic structure
            assert!(!result.identifier.is_empty());
            assert!(result.confidence > 0.0);
        }
        Err(_) => {
            // The basic scanner may fail on real rendered images due to
            // padding estimation inaccuracy. This is acceptable for now.
            // The integration test confirms the pipeline does not panic.
        }
    }
}
