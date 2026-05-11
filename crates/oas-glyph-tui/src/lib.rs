//! # OpenAgent Glyph TUI
//!
//! Terminal rendering library for OpenAgent Glyphs. Renders OAS identity glyphs
//! directly in the terminal using multiple rendering backends:
//!
//! ## Rendering Tiers
//!
//! The library auto-detects terminal capabilities and selects the best renderer:
//!
//! ### Tier 1: Kitty Graphics Protocol
//! Full-color PNG image transmitted inline via the Kitty graphics protocol.
//! Pixel-perfect rendering at any size. Supported by: Kitty, WezTerm, Ghostty.
//!
//! ### Tier 2: iTerm2 Inline Images
//! Base64-encoded PNG via iTerm2's proprietary escape sequence.
//! Supported by: iTerm2, Hyper (with plugin), WezTerm.
//!
//! ### Tier 3: Sixel Graphics
//! Sixel bitmap protocol — the oldest terminal graphics standard (DEC VT340+).
//! Supported by: mlterm, XTerm (with +sixel), foot, contour, some tmux configs.
//!
//! ### Tier 4: Unicode Half-Block
//! Pure text rendering using Unicode half-block characters (▀▄█░).
//! Each character cell represents two vertical pixels via foreground/background
//! color tricks. Works in ALL modern terminals with true-color support.
//!
//! ### Tier 5: Braille Dot Patterns
//! Unicode Braille characters (⠀-⣿) encode a 2×4 dot grid per character cell,
//! giving 2x the horizontal and 4x the vertical resolution of normal text.
//! Monochrome but extremely detailed. Works everywhere Unicode works.
//!
//! ### Tier 6: ASCII Block Art
//! Pure ASCII fallback using block characters and ANSI 256-color palette.
//! Maximum compatibility — works in virtually any terminal emulator.
//!
//! ## Ratatui Widget
//!
//! The library also provides a `GlyphWidget` that implements ratatui's `Widget`
//! trait, so glyphs can be embedded in any ratatui-based TUI application as a
//! first-class UI component alongside tables, charts, and forms.

pub mod render;
pub mod detect;
pub mod widget;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum TuiError {
    #[error("glyph error: {0}")]
    Glyph(#[from] oas_glyph::GlyphError),

    #[error("terminal error: {0}")]
    Terminal(String),

    #[error("render error: {0}")]
    Render(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, TuiError>;

/// Terminal graphics capability level, ordered by fidelity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GraphicsCapability {
    /// ASCII block art with ANSI 256 colors
    Ascii,
    /// Unicode Braille dot patterns (monochrome, high resolution)
    Braille,
    /// Unicode half-block characters with true-color fg/bg
    HalfBlock,
    /// Sixel bitmap graphics
    Sixel,
    /// iTerm2 inline image protocol
    Iterm2,
    /// Kitty graphics protocol (highest fidelity)
    Kitty,
}

impl GraphicsCapability {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Ascii => "ASCII 256-color",
            Self::Braille => "Braille dot patterns",
            Self::HalfBlock => "Unicode half-block",
            Self::Sixel => "Sixel graphics",
            Self::Iterm2 => "iTerm2 inline images",
            Self::Kitty => "Kitty graphics protocol",
        }
    }
}

/// Render size specification.
#[derive(Debug, Clone, Copy, Default)]
pub enum GlyphSize {
    /// Size in terminal character cells (width x height).
    /// Height is typically ~2x width for square glyphs due to cell aspect ratio.
    Cells(u16, u16),
    /// Size in pixels (for graphics protocols).
    Pixels(u32, u32),
    /// Auto-detect based on terminal size. Fills available space or a sensible default.
    #[default]
    Auto,
    /// Compact inline rendering (fits in a single line of output context).
    Compact,
}

/// Options for terminal glyph rendering.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Desired render size.
    pub size: GlyphSize,
    /// Force a specific renderer (overrides auto-detection).
    pub force_renderer: Option<GraphicsCapability>,
    /// Whether to print the DID below the glyph.
    pub show_did: bool,
    /// Whether to print the entity kind label.
    pub show_kind: bool,
    /// Whether to show the color palette strip below the glyph.
    pub show_palette: bool,
    /// Border style around the glyph.
    pub border: BorderStyle,
    /// Whether to use the compact single-line representation.
    pub compact: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            size: GlyphSize::Auto,
            force_renderer: None,
            show_did: true,
            show_kind: true,
            show_palette: false,
            border: BorderStyle::Rounded,
            compact: false,
        }
    }
}

/// Border style for glyph display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    None,
    Single,
    Double,
    Rounded,
    Heavy,
}

/// Render a glyph to stdout.
///
/// This is the primary entry point. It auto-detects terminal capabilities
/// and renders the glyph at the best available fidelity.
///
/// ```rust
/// use oas_glyph_tui::{render_to_stdout, RenderOptions};
///
/// render_to_stdout(
///     "did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
///     &RenderOptions::default(),
/// ).unwrap();
/// ```
pub fn render_to_stdout(did: &str, opts: &RenderOptions) -> Result<()> {
    let cap = opts.force_renderer.unwrap_or_else(detect::detect_capability);
    let output = render_to_string(did, opts, cap)?;
    print!("{}", output);
    Ok(())
}

/// Render a glyph to a String (ANSI escape sequences included).
pub fn render_to_string(
    did: &str,
    opts: &RenderOptions,
    cap: GraphicsCapability,
) -> Result<String> {
    let parsed = oas_glyph::ParsedDid::parse(did)?;
    let payload = oas_glyph::encoding::encode_did_payload(&parsed)?;
    let codeword = oas_glyph::encoding::encode_with_ecc(&payload)?;
    let grid = oas_glyph::encoding::GlyphGrid::from_codeword(&codeword, parsed.kind)?;
    let palette = oas_glyph::visual::GlyphPalette::from_did(did);

    let mut output = String::new();

    // Border top
    if opts.border != BorderStyle::None {
        output.push_str(&render::border::top_border(opts, &parsed));
    }

    // Main glyph body
    match cap {
        GraphicsCapability::Kitty => {
            let png = oas_glyph::visual::did_to_png(did, pixel_size(opts))?;
            output.push_str(&render::kitty::render(&png, opts)?);
        }
        GraphicsCapability::Iterm2 => {
            let png = oas_glyph::visual::did_to_png(did, pixel_size(opts))?;
            output.push_str(&render::iterm2::render(&png, opts)?);
        }
        GraphicsCapability::Sixel => {
            let png = oas_glyph::visual::did_to_png(did, pixel_size(opts))?;
            output.push_str(&render::sixel::render(&png, opts)?);
        }
        GraphicsCapability::HalfBlock => {
            output.push_str(&render::halfblock::render(&grid, &palette, opts)?);
        }
        GraphicsCapability::Braille => {
            output.push_str(&render::braille::render(&grid, &palette, opts)?);
        }
        GraphicsCapability::Ascii => {
            output.push_str(&render::ascii::render(&grid, &palette, opts)?);
        }
    }

    // Palette strip
    if opts.show_palette {
        output.push_str(&render::palette_strip(&palette));
    }

    // DID label
    if opts.show_did {
        output.push_str(&render::did_label(did));
    }

    // Kind badge
    if opts.show_kind {
        output.push_str(&render::kind_badge(&parsed));
    }

    // Border bottom
    if opts.border != BorderStyle::None {
        output.push_str(&render::border::bottom_border(opts));
    }

    Ok(output)
}

fn pixel_size(opts: &RenderOptions) -> u32 {
    match opts.size {
        GlyphSize::Pixels(w, _) => w,
        GlyphSize::Cells(w, _) => (w as u32) * 8, // ~8px per cell
        GlyphSize::Compact => 64,
        GlyphSize::Auto => 256,
    }
}
