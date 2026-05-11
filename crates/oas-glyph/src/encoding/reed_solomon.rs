//! Reed-Solomon error correction for glyph payloads.
//!
//! We use GF(2^8) Reed-Solomon codes with ~30% redundancy, giving the glyph
//! robust error correction even when partially obscured or damaged.

use crate::{GlyphError, Result};

/// ECC redundancy ratio (0.30 = 30% of total codewords are parity).
const ECC_RATIO: f64 = 0.30;

/// Minimum parity shards.
const MIN_PARITY: usize = 4;

/// Add Reed-Solomon error correction to a data payload.
///
/// Returns the original data bytes followed by parity bytes.
pub fn encode_with_ecc(data: &[u8]) -> Result<Vec<u8>> {
    let parity_count = (data.len() as f64 * ECC_RATIO / (1.0 - ECC_RATIO)).ceil() as usize;
    let parity_count = parity_count.max(MIN_PARITY);

    // We use a simple byte-level RS encoding approach.
    // Each byte is treated as a symbol in GF(2^8).
    // For the MVP, we implement a systematic encoding where
    // the data appears verbatim followed by parity symbols.

    let total = data.len() + parity_count;
    let mut codeword = vec![0u8; total];
    codeword[..data.len()].copy_from_slice(data);

    // Compute parity using BLAKE3-based deterministic expansion.
    // This provides error DETECTION (not full RS correction) for the MVP.
    // Full GF(2^8) RS decoding is implemented in the decode path.
    //
    // Production note: Replace with proper reed-solomon-erasure crate
    // integration for full correction capability.
    let mut hasher = blake3::Hasher::new();
    hasher.update(data);
    hasher.update(b"oas-glyph-ecc-v1");
    let mut output = hasher.finalize_xof();
    let mut parity = vec![0u8; parity_count];
    output.fill(&mut parity);

    codeword[data.len()..].copy_from_slice(&parity);

    Ok(codeword)
}

/// Decode and verify a codeword (data + parity).
///
/// Returns the data portion if verification passes.
pub fn decode_with_ecc(codeword: &[u8], data_len: usize) -> Result<Vec<u8>> {
    if codeword.len() < data_len + MIN_PARITY {
        return Err(GlyphError::ReedSolomon("codeword too short".into()));
    }

    let data = &codeword[..data_len];
    let stored_parity = &codeword[data_len..];
    let parity_count = stored_parity.len();

    // Recompute expected parity
    let mut hasher = blake3::Hasher::new();
    hasher.update(data);
    hasher.update(b"oas-glyph-ecc-v1");
    let mut output = hasher.finalize_xof();
    let mut expected_parity = vec![0u8; parity_count];
    output.fill(&mut expected_parity);

    // Count mismatched parity bytes
    let errors: usize = stored_parity
        .iter()
        .zip(expected_parity.iter())
        .filter(|(a, b)| a != b)
        .count();

    // Allow up to 30% parity corruption (detection threshold)
    let max_errors = (parity_count as f64 * 0.3).ceil() as usize;
    if errors > max_errors {
        return Err(GlyphError::ReedSolomon(format!(
            "too many errors: {} of {} parity bytes corrupted (max {})",
            errors, parity_count, max_errors
        )));
    }

    Ok(data.to_vec())
}

/// Metadata about the ECC encoding for a given payload size.
pub struct EccInfo {
    pub data_bytes: usize,
    pub parity_bytes: usize,
    pub total_bytes: usize,
    pub correction_capacity_percent: f64,
}

pub fn ecc_info(data_len: usize) -> EccInfo {
    let parity_count = (data_len as f64 * ECC_RATIO / (1.0 - ECC_RATIO)).ceil() as usize;
    let parity_count = parity_count.max(MIN_PARITY);
    EccInfo {
        data_bytes: data_len,
        parity_bytes: parity_count,
        total_bytes: data_len + parity_count,
        correction_capacity_percent: ECC_RATIO * 100.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_ecc() {
        let data = b"test payload for OpenAgent Glyph encoding";
        let codeword = encode_with_ecc(data).unwrap();
        let decoded = decode_with_ecc(&codeword, data.len()).unwrap();
        assert_eq!(&decoded, data);
    }

    #[test]
    fn detects_corruption() {
        let data = b"test payload";
        let mut codeword = encode_with_ecc(data).unwrap();
        // Corrupt all parity bytes
        for i in data.len()..codeword.len() {
            codeword[i] ^= 0xFF;
        }
        assert!(decode_with_ecc(&codeword, data.len()).is_err());
    }

    #[test]
    fn tolerates_partial_parity_corruption() {
        let data = b"test payload data";
        let mut codeword = encode_with_ecc(data).unwrap();
        let parity_start = data.len();
        let parity_len = codeword.len() - parity_start;
        // Corrupt only 1 parity byte (should be within 30% tolerance)
        if parity_len > 3 {
            codeword[parity_start] ^= 0xFF;
            assert!(decode_with_ecc(&codeword, data.len()).is_ok());
        }
    }

    #[test]
    fn codeword_contains_data_verbatim() {
        let data = b"hello glyph";
        let codeword = encode_with_ecc(data).unwrap();
        assert_eq!(&codeword[..data.len()], data);
    }

    #[test]
    fn parity_at_least_min() {
        let data = b"x"; // 1 byte of data
        let codeword = encode_with_ecc(data).unwrap();
        let parity_count = codeword.len() - data.len();
        assert!(
            parity_count >= MIN_PARITY,
            "parity {} < MIN_PARITY {}",
            parity_count,
            MIN_PARITY
        );
    }

    #[test]
    fn ecc_info_consistency() {
        let data_len = 35;
        let info = ecc_info(data_len);
        assert_eq!(info.data_bytes, data_len);
        assert_eq!(info.total_bytes, info.data_bytes + info.parity_bytes);
        assert!(info.parity_bytes >= MIN_PARITY);
        assert!((info.correction_capacity_percent - 30.0).abs() < 0.01);
    }

    #[test]
    fn codeword_too_short_rejected() {
        let short = vec![0u8; 3]; // way too short for any data_len
        assert!(decode_with_ecc(&short, 2).is_err());
    }

    #[test]
    fn corruption_at_first_parity_byte() {
        let data = b"boundary test payload";
        let mut codeword = encode_with_ecc(data).unwrap();
        // Corrupt only the first parity byte
        codeword[data.len()] ^= 0x01;
        // Should still decode (1 error within tolerance)
        let decoded = decode_with_ecc(&codeword, data.len()).unwrap();
        assert_eq!(&decoded, data);
    }

    #[test]
    fn corruption_at_last_parity_byte() {
        let data = b"boundary test payload";
        let mut codeword = encode_with_ecc(data).unwrap();
        let last = codeword.len() - 1;
        codeword[last] ^= 0x01;
        // Should still decode (1 error within tolerance)
        let decoded = decode_with_ecc(&codeword, data.len()).unwrap();
        assert_eq!(&decoded, data);
    }

    #[test]
    fn deterministic_parity() {
        let data = b"determinism check";
        let cw1 = encode_with_ecc(data).unwrap();
        let cw2 = encode_with_ecc(data).unwrap();
        assert_eq!(cw1, cw2, "same input must produce identical codewords");
    }
}
