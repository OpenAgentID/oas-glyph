# oas-glyph-tui

Terminal rendering library for [OpenAgent Glyphs](../oas-glyph/) — OAS visual identity avatars rendered directly in your CLI.

## Quick Start

```bash
# Render a glyph (auto-detects best renderer)
oas-glyph render did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK

# Force half-block rendering at 32 columns
oas-glyph render -r half-block -s 32 did:oas:l1fe:hmr:z6MkiTBz1ymuep...

# Compact inline (for scripts/status bars)
oas-glyph render --compact did:oas:l1fe:agent:z6Mkh...

# Show all entity kinds
oas-glyph gallery --namespace l1fe

# Inspect encoding details
oas-glyph inspect did:oas:l1fe:agent:z6Mkh...

# Check terminal capabilities
oas-glyph diag

# Show identity-derived palette
oas-glyph palette did:oas:l1fe:agent:z6Mkh...
```

## Rendering Tiers

Auto-detected from highest to lowest fidelity:

| Tier | Renderer | Terminals | Quality |
|------|----------|-----------|---------|
| 1 | Kitty graphics | Kitty, WezTerm, Ghostty | Pixel-perfect PNG |
| 2 | iTerm2 inline | iTerm2, Hyper | Pixel-perfect PNG |
| 3 | Sixel bitmap | mlterm, foot, contour | Full-color bitmap |
| 4 | Half-block (▀▄) | Any true-color terminal | True-color, 2:1 vertical res |
| 5 | Braille (⠿⣿) | Any Unicode terminal | High-res monochrome |
| 6 | ASCII blocks | Everything else | 256-color density map |

Override detection with `-r <renderer>` or `OAS_GLYPH_RENDERER=<name>`.

## Library Usage

### Standalone rendering

```rust
use oas_glyph_tui::{render_to_stdout, RenderOptions, GlyphSize};

// Auto-detect and render
render_to_stdout(
    "did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
    &RenderOptions::default(),
)?;

// Custom options
let opts = RenderOptions {
    size: GlyphSize::Cells(24, 24),
    show_palette: true,
    ..Default::default()
};
render_to_stdout("did:oas:l1fe:hmr:z6Mki...", &opts)?;
```

### Ratatui widget

Drop glyphs into any ratatui TUI as a first-class widget:

```rust
use oas_glyph_tui::widget::{GlyphWidget, CompactGlyphWidget};
use ratatui::Frame;
use ratatui::layout::{Layout, Constraint, Direction};

fn ui(frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(30), Constraint::Min(40)])
        .split(frame.area());

    // Full glyph in sidebar
    let glyph = GlyphWidget::new(
        "did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK"
    ).unwrap();
    frame.render_widget(glyph, chunks[0]);

    // Compact inline glyph (for table cells, status bars)
    let mini = CompactGlyphWidget::new("did:oas:l1fe:tool:z6Mkn...").unwrap();
    frame.render_widget(mini, chunks[1]);
}
```

### Integration with oas-cli / sigil-node

```rust
// In your node's peer display:
for peer in connected_peers {
    let glyph = render_to_string(
        &peer.did,
        &RenderOptions { compact: true, ..Default::default() },
        detect::detect_capability(),
    )?;
    println!("{} {} ({})", glyph, peer.did, peer.kind);
}
```

## Environment Variables

| Variable | Effect |
|----------|--------|
| `OAS_GLYPH_RENDERER` | Force renderer: `kitty`, `iterm2`, `sixel`, `halfblock`, `braille`, `ascii` |
| `COLORTERM` | Standard true-color detection (`truecolor` / `24bit`) |
| `TERM_PROGRAM` | Terminal identification (auto-detected) |
| `NO_COLOR` | Respected — disables color, falls back to Braille/ASCII |

## Architecture

```
oas-glyph-tui/
├── src/
│   ├── lib.rs           # Public API, RenderOptions, render_to_stdout()
│   ├── main.rs          # CLI binary (clap)
│   ├── detect/          # Terminal capability detection
│   ├── render/          # Rendering backends
│   │   ├── halfblock.rs # Unicode ▀▄ (Tier 4)
│   │   ├── braille.rs   # Unicode ⠿⣿ (Tier 5)
│   │   ├── ascii.rs     # ASCII ░▒▓█ (Tier 6)
│   │   ├── kitty.rs     # Kitty protocol (Tier 1)
│   │   ├── iterm2.rs    # iTerm2 protocol (Tier 2)
│   │   ├── sixel.rs     # Sixel bitmap (Tier 3)
│   │   └── border.rs    # Box-drawing borders
│   └── widget/          # Ratatui Widget implementations
│       └── mod.rs       # GlyphWidget + CompactGlyphWidget
└── Cargo.toml
```

## Depends on

- `oas-glyph` — core encoding, grid layout, palette derivation, PNG/SVG rendering
- `crossterm` — terminal I/O and capability detection
- `ratatui` — TUI widget framework
- `clap` — CLI argument parsing
