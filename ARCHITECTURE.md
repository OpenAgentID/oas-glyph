# OpenAgent Glyph — Visual Identity for the Open Agent Standard

## What Is It

A **OpenAgent Glyph** is a scannable visual identity artifact for OAS entities. It encodes a `did:oas` address into a multi-channel block grid that serves simultaneously as:

1. **A profile avatar** — every entity gets a unique, deterministic visual identity
2. **A scannable identifier** — camera scan → DID resolution → full identity document
3. **A kind indicator** — entity type (human, agent, org, tool) recognizable at a glance

Think of it as what would happen if QR codes were designed by identity cryptographers instead of barcode engineers.

## Why Not QR Codes

QR codes encode arbitrary URLs as black-and-white binary modules. They work, but they're:

- **Ugly** — no one uses a QR code as a profile picture
- **Generic** — every QR code looks the same until scanned
- **Wasteful** — QR encodes arbitrary-length URLs; we have a fixed-structure DID with known constraints
- **Identity-blind** — a QR code for a human root looks identical to one for a tool

OpenAgent Glyphs exploit the fact that `did:oas` has a *known, fixed structure* to build a purpose-built visual encoding that's denser, prettier, and semantically richer than QR.

## Encoding Architecture

### Data Payload

The DID `did:oas:<namespace>:<kind>:<identifier>` is encoded as:

```
Byte 0:       [version:4][kind:4]        — 1 byte
Byte 1:       namespace_hash             — 1 byte (BLAKE3 truncated)
Byte 2:       identifier_length          — 1 byte
Bytes 3..N:   identifier (raw Ed25519)   — 32 bytes typical
Bytes N..N+4: checksum (BLAKE3)          — 4 bytes
                                         ≈ 38 bytes total
```

The namespace is hashed (not stored verbatim) because:
- Namespaces are registered and looked up, not decoded blind
- The hash serves as verification during resolution
- Saves precious bytes for the identifier

### Three-Channel Cell Encoding

Each grid cell encodes **6 bits** across three visual channels:

| Channel    | Bits | Visual Property                          |
|------------|------|------------------------------------------|
| Block size | 2    | Cell fill percentage (35%, 55%, 75%, 95%) |
| Shape      | 2    | Square, circle, diamond, triangle         |
| Hue shift  | 2    | Position within identity-derived palette   |

This yields **6 bits per cell**. A 12×12 grid with ~95 data cells provides **570 bits** of raw capacity — more than enough for a 38-byte payload + 30% Reed-Solomon ECC.

### Grid Layout (12×12 = 144 cells)

```
┌──┬────────────┬──┐
│FP│  data ···  │FP│   FP = Finder Pattern (3×3, three corners)
├──┤            ├──┤   KR = Kind Region (3×3, bottom-left)
│  │            │  │   AP = Alignment Pattern (center cell)
│FM│   DC ···   │  │   FM = Format strip (metadata)
│  │     AP     │  │   DC = Data Cells (~95 cells)
│  │   DC ···   │  │
├──┤            ├──┤
│KR│  data ···  │FP│
└──┴────────────┴──┘
```

**Finder Patterns** (27 cells): Three distinctive nested-square corner blocks for orientation detection. The missing fourth corner (bottom-left) is replaced by the Kind Region, breaking rotational symmetry for unambiguous orientation.

**Kind Region** (9 cells): Encodes entity type as a recognizable visual motif — shields for humans, diamonds for agents, hexagons for orgs, gears for tools. Recognizable at a glance without scanning.

**Alignment Pattern** (1 cell): Center reference point for perspective correction.

**Format Strip** (~12 cells): Encodes version, ECC level, data length.

**Data Cells** (~95 cells): Carry the encoded payload.

### Error Correction

Reed-Solomon codes at 30% redundancy. The glyph tolerates ~30% cell damage or occlusion and still decodes correctly. Combined with the multi-channel encoding (which provides implicit redundancy through visual correlation), effective error tolerance is higher.

### Identity-Derived Color Palette

Every glyph's color palette is deterministically derived from the BLAKE3 hash of the full DID:

- **Primary hue**: bytes 0-3 → 0-360° on the color wheel
- **Secondary hue**: bytes 4-7 → offset 60-180° from primary
- **Saturation**: byte 8 → 50-90%
- **Lightness**: byte 9 → 35-55%

This means:
- The same DID always produces the same colors, everywhere
- Different DIDs produce visually distinct palettes
- No two entities look alike

## API Reference

### `POST /v1/glyph/generate`

Generate a glyph from a DID.

**Request:**
```json
{
  "did": "did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
  "size": 512,
  "format": "svg",
  "padding": 24,
  "corner_radius": 16
}
```

**Response:**
```json
{
  "did": "did:oas:l1fe:agent:z6Mkh...",
  "kind": "agent",
  "namespace": "l1fe",
  "format": "svg",
  "data": "<svg ...>...</svg>",
  "palette": {
    "background": "#141214",
    "primary": "#2d8c5a",
    "secondary": "#5a2d8c",
    "data_colors": ["#4ab87a", "#3a9c6a", "#2d7c5a", "#6a3a9c"],
    "finder_dark": "#1a1c1a",
    "finder_light": "#d4d8d4",
    "kind_accent": "#9c5ae6"
  }
}
```

### `GET /v1/glyph/svg/:did`

Direct SVG output. URL-encode the DID.

```
GET /v1/glyph/svg/did%3Aoas%3Al1fe%3Aagent%3Az6Mkh...?size=256
Content-Type: image/svg+xml
```

### `GET /v1/glyph/png/:did`

Direct PNG output.

```
GET /v1/glyph/png/did%3Aoas%3Al1fe%3Aagent%3Az6Mkh...?size=1024
Content-Type: image/png
```

### `GET /v1/palette/:did`

Return just the identity-derived color palette.

### `POST /v1/glyph/info`

Return encoding metadata (grid capacity, ECC stats, payload breakdown).

## Scanning Pipeline

1. **Detect finder patterns** — locate three distinctive corner blocks
2. **Determine orientation** — missing corner (Kind Region) breaks symmetry
3. **Perspective correction** — alignment pattern provides center reference
4. **Cell sampling** — extract RGB values at each cell center
5. **Channel decomposition** — reverse size/shape/hue to 6-bit values
6. **Codeword reconstruction** — reassemble byte stream from 6-bit cells
7. **Error correction** — Reed-Solomon decode with repair
8. **Payload decode** — extract kind, namespace hash, identifier
9. **DID resolution** — look up namespace in MARS registry, reconstruct full DID

## Integration with OAS Ecosystem

### MARS Registry

When a glyph is scanned, the recovered namespace hash is checked against known MARS-registered namespaces to reconstruct the full DID. The resolver then fetches the OAS Identity Document from Weave DHT or Sigil chain.

### Sigil Chain Anchoring

Glyphs can be anchored on-chain: the BLAKE3 hash of the glyph's SVG/PNG is stored alongside the DID document hash, proving the glyph was generated from the canonical identity.

### Agent Profiles

Every OAS entity — agents, tools, orgs, human roots — gets a OpenAgent Glyph as its visual identity. The Kind Region makes entity types instantly recognizable:

- **Human Root (HMR)**: Shield motif — accountability anchor
- **Multi-Human Root (MHR)**: Overlapping shields — group trust
- **Autonomous Org (AO)**: Hexagon — organizational structure
- **Agent**: Diamond — precision and capability
- **Tool**: Gear — functional utility

## Crate Structure

```
oas-glyph/
├── src/
│   ├── lib.rs              # Core types: ParsedDid, EntityKind, errors
│   ├── main.rs             # Axum server binary
│   ├── encoding/
│   │   ├── mod.rs
│   │   ├── payload.rs      # DID → compact byte payload
│   │   ├── grid.rs         # Byte payload → 12×12 cell grid
│   │   └── reed_solomon.rs # Error correction encoding/decoding
│   ├── visual/
│   │   ├── mod.rs
│   │   ├── palette.rs      # BLAKE3-derived identity colors
│   │   └── renderer.rs     # Grid + palette → SVG/PNG
│   ├── scanner/
│   │   └── mod.rs          # Image → DID decoding pipeline
│   └── api/
│       └── mod.rs          # Axum HTTP handlers
└── Cargo.toml
```

## Cryptographic Alignment

All crypto primitives align with the OAS stack:

| Primitive    | Usage in Glyph          | OAS Alignment                  |
|-------------|-------------------------|--------------------------------|
| BLAKE3      | Palette derivation, ECC | Content hashing, audit chains  |
| Ed25519     | Identifier encoding     | All entity signing keys        |
| Base58btc   | Identifier decode       | Multibase public key encoding  |
| HKDF-SHA256 | (future: derived glyphs)| Lineage key derivation         |

## Future Extensions

- **Animated glyphs** — WebGL/Lottie rendering with subtle identity-derived motion
- **Lineage visualization** — visual chain connecting child glyph to parent to human root
- **Glyph watermarking** — embed glyphs as invisible watermarks in agent-generated content
- **AR scanning** — real-time camera scanning with overlay showing identity details
- **Glyph NFT** — anchor glyph as unique visual identity token on Sigil chain
- **Size-adaptive grids** — 8×8 for thumbnails, 16×16 for high-fidelity prints
