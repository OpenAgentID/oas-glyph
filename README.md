# OpenAgent Glyph

**Visual identity system for the Open Agent Standard.**

OpenAgent Glyphs are scannable, aesthetically unique avatars that encode `did:oas` addresses into multi-channel block grids. Every OAS entity — agents, tools, orgs, human roots — gets a deterministic visual identity that doubles as a machine-readable identifier.

`oas-glyph` is a **lower-level presentation and transport library** in the OAS stack. It does not redefine OAS identity. The core OAS SDKs remain the source of truth for DID syntax, identity documents, lineage, attestation, and resolution. Glyph turns that same OAS identity into a production-ready visual artifact that can be rendered, scanned, and embedded anywhere.

```
oas-glyph/
├── Cargo.toml                    # Workspace root
├── ARCHITECTURE.md               # Full technical architecture
├── examples/
│   └── usage.rs                  # Library usage examples
│
└── crates/
    ├── oas-glyph/              # Core library
    │   ├── src/
    │   │   ├── lib.rs            # ParsedDid, EntityKind, errors
    │   │   ├── main.rs           # HTTP API server (axum)
    │   │   ├── encoding/
    │   │   │   ├── payload.rs    # DID → compact byte payload
    │   │   │   ├── grid.rs       # Payload → 12×12 cell grid
    │   │   │   └── reed_solomon.rs
    │   │   ├── visual/
    │   │   │   ├── palette.rs    # BLAKE3-derived identity colors
    │   │   │   └── renderer.rs   # Grid → SVG / PNG
    │   │   ├── scanner/          # Image → DID decode pipeline
    │   │   └── api/              # REST endpoints
    │   └── Cargo.toml
    │
    └── oas-glyph-tui/          # Terminal renderer
        ├── src/
        │   ├── lib.rs            # render_to_stdout(), RenderOptions
        │   ├── main.rs           # CLI binary (clap)
        │   ├── detect/           # Terminal capability auto-detection
        │   ├── render/
        │   │   ├── halfblock.rs  # Unicode ▀▄ (true-color)
        │   │   ├── braille.rs    # Unicode ⠿⣿ (high-res mono)
        │   │   ├── ascii.rs      # ASCII ░▒▓█ (256-color)
        │   │   ├── kitty.rs      # Kitty graphics protocol
        │   │   ├── iterm2.rs     # iTerm2 inline images
        │   │   ├── sixel.rs      # Sixel bitmap
        │   │   └── border.rs     # Box-drawing frames
        │   └── widget/           # Ratatui Widget impls
        ├── README.md
        └── Cargo.toml
```

## Quick Start

```bash
# Build everything
cargo build --workspace

# Run the HTTP API server
cargo run -p oas-glyph --bin oas-glyph-server

# Render a glyph in your terminal
cargo run -p oas-glyph-tui -- render did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK

# Gallery of all entity kinds
cargo run -p oas-glyph-tui -- gallery

# Check terminal graphics capabilities
cargo run -p oas-glyph-tui -- diag

# Inspect encoding details
cargo run -p oas-glyph-tui -- inspect did:oas:l1fe:agent:z6Mkh...
```

## OAS Implementation Scope

OpenAgent Glyph is OAS-native at every layer:

- Input identity is always `did:oas`
- encoded kind semantics come from the OAS entity taxonomy
- identifier material assumes the OAS Ed25519/base58btc stack
- scan results are meant to hand off into OAS DID resolution, not bypass it
- palette, finder, and error-correction logic are deterministic functions of the OAS identity

What stays out of scope:

- no alternate DID method
- no replacement identity-document model
- no alternate trust chain outside OAS lineage and resolution
- no visual redesign of the glyph system just to “add OAS”

## Crate Overview

### `oas-glyph` — Core Library & API

The encoding engine and visual renderer. Use as a library dependency or run as an HTTP service.

**As a library:**
```rust
use oas_glyph::visual::renderer::{did_to_svg, did_to_png};

let svg = did_to_svg("did:oas:l1fe:agent:z6Mkh...", None)?;
let png = did_to_png("did:oas:l1fe:agent:z6Mkh...", 512)?;
```

**As an API:**
```bash
curl -X POST http://localhost:3377/v1/glyph/generate \
  -H 'Content-Type: application/json' \
  -d '{"did":"did:oas:l1fe:agent:z6Mkh...","format":"svg"}'
```

### `oas-glyph-tui` — Terminal Renderer & CLI

Six rendering backends, auto-detected from highest to lowest fidelity:

| Tier | Backend | How |
|------|---------|-----|
| 1 | Kitty | PNG via graphics protocol |
| 2 | iTerm2 | PNG via inline image escape |
| 3 | Sixel | Bitmap via DEC sixel protocol |
| 4 | Half-block | Unicode `▀▄` with true-color fg/bg |
| 5 | Braille | Unicode `⠿⣿` dot patterns |
| 6 | ASCII | `░▒▓█` with 256-color ANSI |

**As a library (embed in your TUI):**
```rust
use oas_glyph_tui::widget::GlyphWidget;

let glyph = GlyphWidget::new("did:oas:l1fe:agent:z6Mkh...")?;
frame.render_widget(glyph, area);
```

## Encoding

Each glyph packs a `did:oas` address into a 12×12 cell grid using three visual channels per cell (block size, shape, hue) for 6 bits per cell. Reed-Solomon error correction at 30% redundancy makes glyphs scannable even when partially obscured.

Colors are deterministically derived from the BLAKE3 hash of the full DID — every entity gets a unique, consistent palette across all renderings.

The glyph therefore behaves like a visual front-end for OAS identity:

- the **DID** remains the canonical identity
- the **glyph** is the deterministic visual carrier
- the **resolver** remains responsible for document fetch, lineage verification, and higher-level trust decisions

See [ARCHITECTURE.md](./ARCHITECTURE.md) for the full technical spec.

## Cryptographic Alignment

All primitives match the OAS stack: BLAKE3 for hashing, Ed25519 keys as the encoded payload, base58btc for identifier encoding, HKDF-SHA256 reserved for derived glyphs.

In practice that means:

- glyph payloads should be generated from OAS-valid DIDs
- glyph scans should reconstruct OAS-valid DIDs
- any post-scan verification should use the core OAS libraries, not glyph-local trust shortcuts

## Contributing and Security

See [CONTRIBUTING.md](./CONTRIBUTING.md) for development and pull-request
guidance. Report vulnerabilities privately according to
[SECURITY.md](./SECURITY.md).

## License

Licensed under either the [Apache License, Version 2.0](./LICENSE-APACHE) or
the [MIT License](./LICENSE-MIT), at your option.
