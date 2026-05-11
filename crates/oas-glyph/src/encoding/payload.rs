//! Payload construction: DID components → compact byte representation.

use crate::{EntityKind, GlyphError, ParsedDid, Result};

/// Maximum raw payload size in bytes (before ECC).
/// With 120 data cells × 6 bits = 720 bits = 90 bytes total capacity.
/// At 30% ECC overhead, we have ~63 bytes for data.
pub const MAX_PAYLOAD_BYTES: usize = 60;

/// Glyph format version (4 bits, values 0-15).
pub const GLYPH_VERSION: u8 = 1;

/// Encode a parsed DID into a compact byte payload.
///
/// # Payload Format
///
/// ```text
/// Byte 0:       [version:4][kind:4]
/// Byte 1:       namespace_hash (BLAKE3 of namespace, truncated to 8 bits)
/// Byte 2:       identifier_length
/// Bytes 3..N:   identifier bytes (raw, decoded from base58)
/// Bytes N..N+4: checksum (BLAKE3 of bytes 0..N, truncated to 32 bits)
/// ```
///
/// The namespace is hashed rather than stored verbatim because:
/// - Namespaces are developer-registered and looked up, not decoded from the glyph
/// - The hash serves as a consistency check during resolution
/// - Saves bytes for the identifier (the critical payload)
pub fn encode_did_payload(parsed: &ParsedDid) -> Result<Vec<u8>> {
    // Decode the base58btc identifier to raw bytes
    let id_bytes = if parsed.identifier.starts_with('z') {
        // Multibase base58btc prefix
        bs58::decode(&parsed.identifier[1..])
            .into_vec()
            .map_err(|e| GlyphError::Encoding(format!("base58 decode: {}", e)))?
    } else {
        // Try raw base58
        bs58::decode(&parsed.identifier)
            .into_vec()
            .map_err(|e| GlyphError::Encoding(format!("base58 decode: {}", e)))?
    };

    if id_bytes.len() > 48 {
        return Err(GlyphError::DidTooLong {
            max: 48,
            got: id_bytes.len(),
        });
    }

    let mut payload = Vec::with_capacity(MAX_PAYLOAD_BYTES);

    // Byte 0: version (high nibble) + kind (low nibble)
    let header = (GLYPH_VERSION << 4) | (parsed.kind as u8 & 0x0F);
    payload.push(header);

    // Byte 1: namespace hash
    let ns_hash = blake3::hash(parsed.namespace.as_bytes());
    payload.push(ns_hash.as_bytes()[0]);

    // Byte 2: identifier length
    payload.push(id_bytes.len() as u8);

    // Bytes 3..N: identifier
    payload.extend_from_slice(&id_bytes);

    // Checksum: BLAKE3 of everything so far, truncated to 4 bytes
    let check = blake3::hash(&payload);
    payload.extend_from_slice(&check.as_bytes()[..4]);

    if payload.len() > MAX_PAYLOAD_BYTES {
        return Err(GlyphError::DidTooLong {
            max: MAX_PAYLOAD_BYTES,
            got: payload.len(),
        });
    }

    Ok(payload)
}

/// Decode a byte payload back into DID components.
///
/// Returns (version, kind, namespace_hash, identifier_bytes).
/// The full DID string is reconstructed by looking up the namespace
/// via the hash in a registry, or by providing it externally.
pub fn decode_did_payload(payload: &[u8]) -> Result<DecodedPayload> {
    if payload.len() < 7 {
        return Err(GlyphError::Decoding("payload too short".into()));
    }

    let version = payload[0] >> 4;
    let kind_val = payload[0] & 0x0F;
    let namespace_hash = payload[1];
    let id_len = payload[2] as usize;

    if payload.len() < 3 + id_len + 4 {
        return Err(GlyphError::Decoding("payload truncated".into()));
    }

    let id_bytes = &payload[3..3 + id_len];
    let stored_checksum = &payload[3 + id_len..3 + id_len + 4];

    // Verify checksum
    let check = blake3::hash(&payload[..3 + id_len]);
    if &check.as_bytes()[..4] != stored_checksum {
        return Err(GlyphError::Decoding("checksum mismatch".into()));
    }

    // Re-encode identifier to base58btc with multibase prefix
    let identifier = format!("z{}", bs58::encode(id_bytes).into_string());

    // Map kind value back
    let kind = match kind_val {
        0 => EntityKind::Hmr,
        1 => EntityKind::Mhr,
        2 => EntityKind::Ao,
        3 => EntityKind::Agent,
        4 => EntityKind::AgentInstance,
        5 => EntityKind::Tool,
        6 => EntityKind::Skill,
        7 => EntityKind::Workflow,
        8 => EntityKind::Model,
        9 => EntityKind::Dataset,
        10 => EntityKind::Service,
        _ => return Err(GlyphError::Decoding(format!("unknown kind: {}", kind_val))),
    };

    Ok(DecodedPayload {
        version,
        kind,
        namespace_hash,
        identifier,
    })
}

/// Decoded payload components.
#[derive(Debug, Clone)]
pub struct DecodedPayload {
    pub version: u8,
    pub kind: EntityKind,
    pub namespace_hash: u8,
    pub identifier: String,
}

impl DecodedPayload {
    /// Reconstruct a full DID string given the namespace.
    pub fn to_did(&self, namespace: &str) -> String {
        format!(
            "did:oas:{}:{}:{}",
            namespace,
            self.kind.as_str(),
            self.identifier
        )
    }

    /// Verify that a candidate namespace matches the stored hash.
    pub fn verify_namespace(&self, namespace: &str) -> bool {
        let hash = blake3::hash(namespace.as_bytes());
        hash.as_bytes()[0] == self.namespace_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_encode_decode() {
        let did = "did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";
        let parsed = ParsedDid::parse(did).unwrap();
        let payload = encode_did_payload(&parsed).unwrap();
        let decoded = decode_did_payload(&payload).unwrap();

        assert_eq!(decoded.kind, EntityKind::Agent);
        assert!(decoded.verify_namespace("l1fe"));
        assert_eq!(decoded.version, GLYPH_VERSION);

        let reconstructed = decoded.to_did("l1fe");
        assert_eq!(reconstructed, did);
    }

    #[test]
    fn test_all_kinds() {
        let kinds = [
            "hmr", "mhr", "ao", "agent", "tool", "skill",
            "workflow", "model", "dataset", "service",
        ];
        for kind in kinds {
            let did = format!(
                "did:oas:test:{}:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
                kind
            );
            let parsed = ParsedDid::parse(&did).unwrap();
            let payload = encode_did_payload(&parsed).unwrap();
            let decoded = decode_did_payload(&payload).unwrap();
            assert_eq!(decoded.kind.as_str(), kind);
        }
    }

    #[test]
    fn roundtrip_agent_instance_kind() {
        let did = "did:oas:l1fe:agent:instance:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";
        let parsed = ParsedDid::parse(did).unwrap();
        assert_eq!(parsed.kind, EntityKind::AgentInstance);
        let payload = encode_did_payload(&parsed).unwrap();
        let decoded = decode_did_payload(&payload).unwrap();
        assert_eq!(decoded.kind, EntityKind::AgentInstance);
    }

    #[test]
    fn roundtrip_all_11_entity_kinds() {
        let simple_kinds = [
            ("hmr", EntityKind::Hmr),
            ("mhr", EntityKind::Mhr),
            ("ao", EntityKind::Ao),
            ("agent", EntityKind::Agent),
            ("tool", EntityKind::Tool),
            ("skill", EntityKind::Skill),
            ("workflow", EntityKind::Workflow),
            ("model", EntityKind::Model),
            ("dataset", EntityKind::Dataset),
            ("service", EntityKind::Service),
        ];

        for (kind_str, expected_kind) in simple_kinds {
            let did = format!(
                "did:oas:l1fe:{}:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
                kind_str
            );
            let parsed = ParsedDid::parse(&did).unwrap();
            assert_eq!(parsed.kind, expected_kind, "kind mismatch for {}", kind_str);
            let payload = encode_did_payload(&parsed).unwrap();
            let decoded = decode_did_payload(&payload).unwrap();
            assert_eq!(decoded.kind, expected_kind);
            assert_eq!(decoded.to_did("l1fe"), did);
        }

        // Agent instance (compound kind)
        let did = "did:oas:l1fe:agent:instance:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";
        let parsed = ParsedDid::parse(did).unwrap();
        assert_eq!(parsed.kind, EntityKind::AgentInstance);
        let payload = encode_did_payload(&parsed).unwrap();
        let decoded = decode_did_payload(&payload).unwrap();
        assert_eq!(decoded.kind, EntityKind::AgentInstance);
    }

    #[test]
    fn rejects_invalid_did_prefix() {
        assert!(ParsedDid::parse("notadid:oas:l1fe:agent:zABC").is_err());
        assert!(ParsedDid::parse("did:xyz:l1fe:agent:zABC").is_err());
    }

    #[test]
    fn rejects_too_few_parts() {
        assert!(ParsedDid::parse("did:oas:l1fe").is_err());
        assert!(ParsedDid::parse("did:oas").is_err());
        assert!(ParsedDid::parse("did:oas:l1fe:agent").is_err());
    }

    #[test]
    fn rejects_unknown_kind() {
        assert!(ParsedDid::parse("did:oas:l1fe:unknownkind:zABCDEF").is_err());
    }

    #[test]
    fn various_namespace_values() {
        let namespaces = ["l1fe", "anthropic", "openai", "a"];
        for ns in namespaces {
            let did = format!(
                "did:oas:{}:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
                ns
            );
            let parsed = ParsedDid::parse(&did).unwrap();
            assert_eq!(parsed.namespace, ns);
            let payload = encode_did_payload(&parsed).unwrap();
            let decoded = decode_did_payload(&payload).unwrap();
            assert!(decoded.verify_namespace(ns));
        }
    }

    #[test]
    fn namespace_hash_rejects_wrong_namespace() {
        let did = "did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";
        let parsed = ParsedDid::parse(did).unwrap();
        let payload = encode_did_payload(&parsed).unwrap();
        let decoded = decode_did_payload(&payload).unwrap();
        // The wrong namespace should very likely fail (1/256 chance of false positive)
        // Use a clearly different namespace.
        assert!(!decoded.verify_namespace("definitely_not_l1fe_namespace"));
    }

    #[test]
    fn payload_too_short_rejected() {
        // Less than 7 bytes should fail
        let short = vec![0u8; 6];
        assert!(decode_did_payload(&short).is_err());
    }

    #[test]
    fn payload_truncated_rejected() {
        // Valid header but truncated identifier
        let did = "did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";
        let parsed = ParsedDid::parse(did).unwrap();
        let payload = encode_did_payload(&parsed).unwrap();
        // Truncate to remove the checksum
        let truncated = &payload[..payload.len() - 2];
        assert!(decode_did_payload(truncated).is_err());
    }

    #[test]
    fn checksum_mismatch_detected() {
        let did = "did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";
        let parsed = ParsedDid::parse(did).unwrap();
        let mut payload = encode_did_payload(&parsed).unwrap();
        // Corrupt the last byte (part of checksum)
        let last = payload.len() - 1;
        payload[last] ^= 0xFF;
        assert!(decode_did_payload(&payload).is_err());
    }

    #[test]
    fn version_is_embedded() {
        let did = "did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";
        let parsed = ParsedDid::parse(did).unwrap();
        let payload = encode_did_payload(&parsed).unwrap();
        let decoded = decode_did_payload(&payload).unwrap();
        assert_eq!(decoded.version, GLYPH_VERSION);
    }

    #[test]
    fn max_payload_size_respected() {
        // A short identifier should produce a payload well under MAX_PAYLOAD_BYTES
        let did = "did:oas:l1fe:hmr:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";
        let parsed = ParsedDid::parse(did).unwrap();
        let payload = encode_did_payload(&parsed).unwrap();
        assert!(
            payload.len() <= MAX_PAYLOAD_BYTES,
            "payload {} bytes > max {}",
            payload.len(),
            MAX_PAYLOAD_BYTES
        );
    }

    #[test]
    fn identifier_without_multibase_prefix() {
        // Identifier without 'z' prefix should also work (raw base58)
        let did = "did:oas:l1fe:agent:6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";
        let parsed = ParsedDid::parse(did).unwrap();
        let result = encode_did_payload(&parsed);
        assert!(result.is_ok());
    }
}
