//! # OpenAgent Glyph
//!
//! Visual identity glyphs for the Open Agent Standard (OAS).
//!
//! A OpenAgent Glyph is a scannable, aesthetically unique avatar that encodes a
//! `did:oas` address into a multi-channel block grid. Unlike QR codes, glyphs
//! use variable block sizes, shapes, and colors to pack identity data into
//! a visually distinctive pattern that doubles as a profile avatar.
//!
//! ## Encoding Architecture
//!
//! The glyph encodes data across three visual channels:
//!
//! - **Block size** (4 levels) → 2 bits per cell
//! - **Shape** (4 variants: square, circle, diamond, triangle) → 2 bits per cell
//! - **Hue shift** (4 levels within identity-derived palette) → 2 bits per cell
//!
//! This yields 6 bits per cell. A 12×12 grid (minus finder patterns and metadata
//! cells) provides ~120 data cells × 6 bits = 720 raw bits, more than enough for
//! a 32-byte identifier + kind + namespace + Reed-Solomon ECC at 30% redundancy.
//!
//! ## Grid Layout (12×12)
//!
//! ```text
//! ┌──┬────────────┬──┐
//! │FP│  data ···  │FP│   FP = Finder Pattern (3×3 corners)
//! ├──┤            ├──┤   KR = Kind Region (visual entity type)
//! │  │            │  │   DC = Data Cells (encoded payload)
//! │  │   DC ···   │  │   AP = Alignment Pattern (center)
//! │  │     AP     │  │
//! │  │   DC ···   │  │
//! ├──┤            ├──┤
//! │KR│  data ···  │FP│
//! └──┴────────────┴──┘
//! ```
//!
//! ## Color Derivation
//!
//! The base color palette is deterministically derived from the BLAKE3 hash of the
//! full DID string. This ensures every identity has a unique, consistent color
//! scheme across all renderings.

pub mod encoding;
pub mod visual;
pub mod scanner;
pub mod api;

use thiserror::Error;

/// Errors that can occur during glyph operations.
#[derive(Error, Debug)]
pub enum GlyphError {
    #[error("invalid DID format: {0}")]
    InvalidDid(String),

    #[error("DID too long for glyph encoding (max {max} bytes, got {got})")]
    DidTooLong { max: usize, got: usize },

    #[error("encoding error: {0}")]
    Encoding(String),

    #[error("decoding error: {0}")]
    Decoding(String),

    #[error("Reed-Solomon error: {0}")]
    ReedSolomon(String),

    #[error("render error: {0}")]
    Render(String),

    #[error("scan error: {0}")]
    Scan(String),

    #[error("invalid glyph image: {0}")]
    InvalidImage(String),
}

pub type Result<T> = std::result::Result<T, GlyphError>;

/// The OAS entity kinds, encoded as a 4-bit value in the glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum EntityKind {
    Hmr = 0,
    Mhr = 1,
    Ao = 2,
    Agent = 3,
    AgentInstance = 4,
    Tool = 5,
    Skill = 6,
    Workflow = 7,
    Model = 8,
    Dataset = 9,
    Service = 10,
    // 11-15 reserved for future kinds
}

impl EntityKind {
    pub fn parse_kind(s: &str) -> Option<Self> {
        match s {
            "hmr" => Some(Self::Hmr),
            "mhr" => Some(Self::Mhr),
            "ao" => Some(Self::Ao),
            "agent" => Some(Self::Agent),
            "agent:instance" => Some(Self::AgentInstance),
            "tool" => Some(Self::Tool),
            "skill" => Some(Self::Skill),
            "workflow" => Some(Self::Workflow),
            "model" => Some(Self::Model),
            "dataset" => Some(Self::Dataset),
            "service" => Some(Self::Service),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Hmr => "hmr",
            Self::Mhr => "mhr",
            Self::Ao => "ao",
            Self::Agent => "agent",
            Self::AgentInstance => "agent:instance",
            Self::Tool => "tool",
            Self::Skill => "skill",
            Self::Workflow => "workflow",
            Self::Model => "model",
            Self::Dataset => "dataset",
            Self::Service => "service",
        }
    }

    /// Visual signature pattern for this kind — determines the
    /// structural motif in the Kind Region of the glyph.
    pub fn visual_signature(&self) -> KindSignature {
        match self {
            Self::Hmr => KindSignature::HumanShield,
            Self::Mhr => KindSignature::MultiShield,
            Self::Ao => KindSignature::OrgHex,
            Self::Agent => KindSignature::AgentDiamond,
            Self::AgentInstance => KindSignature::AgentDiamondHollow,
            Self::Tool => KindSignature::ToolGear,
            Self::Skill => KindSignature::SkillStar,
            Self::Workflow => KindSignature::FlowArrows,
            Self::Model => KindSignature::ModelCube,
            Self::Dataset => KindSignature::DataGrid,
            Self::Service => KindSignature::ServiceCircle,
        }
    }
}

/// Visual motifs for the Kind Region — recognizable at a glance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindSignature {
    HumanShield,        // Single filled shield
    MultiShield,        // Overlapping shields
    OrgHex,             // Hexagon (organization)
    AgentDiamond,       // Filled diamond
    AgentDiamondHollow, // Hollow diamond (instance)
    ToolGear,           // Gear teeth pattern
    SkillStar,          // Star burst
    FlowArrows,         // Directional flow
    ModelCube,          // 3D cube wireframe
    DataGrid,           // Grid dots
    ServiceCircle,      // Concentric circles
}

/// Parsed components of a `did:oas` identifier.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParsedDid {
    /// Full DID string
    pub did: String,
    /// Namespace (e.g., "l1fe", "anthropic")
    pub namespace: String,
    /// Entity kind
    pub kind: EntityKind,
    /// The identifier portion (base58btc-encoded public key hash)
    pub identifier: String,
}

impl ParsedDid {
    /// Parse a `did:oas:<namespace>:<kind>:<identifier>` string.
    pub fn parse(did: &str) -> Result<Self> {
        let parts: Vec<&str> = did.split(':').collect();

        // Minimum: did:oas:namespace:kind:identifier
        // With sub-kind: did:oas:namespace:agent:instance:identifier
        if parts.len() < 5 || parts[0] != "did" || parts[1] != "oas" {
            return Err(GlyphError::InvalidDid(format!(
                "expected did:oas:<namespace>:<kind>:<identifier>, got '{}'",
                did
            )));
        }

        let namespace = parts[2].to_string();

        // Handle compound kinds like "agent:instance"
        let (kind_str, identifier) = if parts.len() == 6 && parts[3] == "agent" && parts[4] == "instance" {
            ("agent:instance", parts[5].to_string())
        } else if parts.len() == 5 {
            (parts[3], parts[4].to_string())
        } else {
            return Err(GlyphError::InvalidDid(format!(
                "unexpected DID structure: '{}'",
                did
            )));
        };

        let kind = EntityKind::parse_kind(kind_str).ok_or_else(|| {
            GlyphError::InvalidDid(format!("unknown kind: '{}'", kind_str))
        })?;

        Ok(Self {
            did: did.to_string(),
            namespace,
            kind,
            identifier,
        })
    }
}
