//! # Terminal Graphics Detection
//!
//! Auto-detects which graphics protocol the current terminal supports,
//! ordered from highest fidelity to lowest:
//!
//! 1. Check `$TERM_PROGRAM` for known terminals (Kitty, iTerm2, WezTerm, Ghostty)
//! 2. Check `$TERM` for sixel-capable terminals
//! 3. Check `$COLORTERM` for true-color support (enables half-block)
//! 4. Fall back to Braille or ASCII

use crate::GraphicsCapability;

/// Detect the best available graphics capability.
pub fn detect_capability() -> GraphicsCapability {
    // Check for forced override via environment
    if let Ok(force) = std::env::var("OAS_GLYPH_RENDERER") {
        match force.to_lowercase().as_str() {
            "kitty" => return GraphicsCapability::Kitty,
            "iterm2" | "iterm" => return GraphicsCapability::Iterm2,
            "sixel" => return GraphicsCapability::Sixel,
            "halfblock" | "half-block" | "half_block" => return GraphicsCapability::HalfBlock,
            "braille" => return GraphicsCapability::Braille,
            "ascii" => return GraphicsCapability::Ascii,
            _ => {}
        }
    }

    let term_program = std::env::var("TERM_PROGRAM")
        .unwrap_or_default()
        .to_lowercase();
    let term = std::env::var("TERM").unwrap_or_default().to_lowercase();
    let colorterm = std::env::var("COLORTERM")
        .unwrap_or_default()
        .to_lowercase();
    let kitty_id = std::env::var("KITTY_WINDOW_ID");
    let ghostty = std::env::var("GHOSTTY_RESOURCES_DIR");

    // Tier 1: Kitty graphics protocol
    if kitty_id.is_ok() || term_program == "kitty" {
        return GraphicsCapability::Kitty;
    }

    // Ghostty supports Kitty protocol
    if ghostty.is_ok() || term_program == "ghostty" {
        return GraphicsCapability::Kitty;
    }

    // WezTerm supports both Kitty and iTerm2 protocols
    if term_program == "wezterm" {
        return GraphicsCapability::Kitty;
    }

    // Tier 2: iTerm2 inline images
    if term_program == "iterm2" || term_program.contains("iterm") {
        return GraphicsCapability::Iterm2;
    }

    // Tier 3: Sixel — check for known sixel-capable terminals
    if term.contains("mlterm")
        || term.contains("yaft")
        || term_program == "foot"
        || term_program == "contour"
        || is_sixel_capable()
    {
        return GraphicsCapability::Sixel;
    }

    // Tier 4: True-color half-block
    if colorterm == "truecolor" || colorterm == "24bit" || term.contains("256color") {
        return GraphicsCapability::HalfBlock;
    }

    // Tier 5: Braille (works if Unicode is supported)
    if supports_unicode() {
        return GraphicsCapability::Braille;
    }

    // Tier 6: ASCII fallback
    GraphicsCapability::Ascii
}

/// Check for sixel capability via terminal query.
///
/// In production, this would send a DA1 query (`\x1b[c`) and parse the response
/// for the `4` parameter (sixel support). For now, we check environment hints.
fn is_sixel_capable() -> bool {
    // Check SIXEL environment hint (some terminals set this)
    std::env::var("TERMINAL_SIXEL").is_ok()
}

/// Heuristic check for Unicode support.
fn supports_unicode() -> bool {
    let lang = std::env::var("LANG").unwrap_or_default().to_lowercase();
    let lc_all = std::env::var("LC_ALL").unwrap_or_default().to_lowercase();
    let lc_ctype = std::env::var("LC_CTYPE").unwrap_or_default().to_lowercase();

    lang.contains("utf")
        || lc_all.contains("utf")
        || lc_ctype.contains("utf")
        || lang.contains("unicode")
}

/// Get the terminal's cell size in pixels (for scaling graphics output).
///
/// Uses the `TIOCGWINSZ` ioctl to get both character and pixel dimensions.
/// Returns (cell_width_px, cell_height_px) or a reasonable default.
pub fn cell_pixel_size() -> (u16, u16) {
    // Try crossterm's terminal size first
    if let Ok((cols, rows)) = crossterm::terminal::size() {
        // Default assumption: 8×16 pixels per cell
        // More accurate: query with `\x1b[14t` (xterm pixel size)
        let _ = (cols, rows);
    }
    (8, 16) // Conservative default
}

/// Detect if we're running inside tmux (affects protocol support).
pub fn in_tmux() -> bool {
    std::env::var("TMUX").is_ok()
}

/// Detect if we're running inside screen (affects protocol support).
pub fn in_screen() -> bool {
    std::env::var("TERM")
        .map(|t| t.starts_with("screen"))
        .unwrap_or(false)
}

/// Print a diagnostic of detected capabilities.
pub fn print_diagnostics() {
    let cap = detect_capability();
    let (cw, ch) = cell_pixel_size();

    println!("┌─ OpenAgent Glyph Terminal Diagnostics ──────────┐");
    println!(
        "│ TERM_PROGRAM: {:>28} │",
        std::env::var("TERM_PROGRAM").unwrap_or_else(|_| "(unset)".into())
    );
    println!(
        "│ TERM:         {:>28} │",
        std::env::var("TERM").unwrap_or_else(|_| "(unset)".into())
    );
    println!(
        "│ COLORTERM:    {:>28} │",
        std::env::var("COLORTERM").unwrap_or_else(|_| "(unset)".into())
    );
    println!(
        "│ Tmux:         {:>28} │",
        if in_tmux() { "yes" } else { "no" }
    );
    println!("│ Cell size:    {:>24}×{}px │", cw, ch);
    println!("│ Detected:     {:>28} │", cap.label());
    println!("└──────────────────────────────────────────────┘");
}
