//! # Terminal Borders
//!
//! Draws Unicode box-drawing borders around glyphs with entity metadata.

use super::{bg, bold, dim, reset};
use crate::{BorderStyle, RenderOptions};
use oas_glyph::visual::palette::GlyphPalette;
use oas_glyph::ParsedDid;

/// Border character sets.
struct BoxChars {
    tl: char, // top-left
    tr: char, // top-right
    bl: char, // bottom-left
    br: char, // bottom-right
    h: char,  // horizontal
    v: char,  // vertical
}

fn chars_for(style: BorderStyle) -> BoxChars {
    match style {
        BorderStyle::Single => BoxChars {
            tl: '┌',
            tr: '┐',
            bl: '└',
            br: '┘',
            h: '─',
            v: '│',
        },
        BorderStyle::Double => BoxChars {
            tl: '╔',
            tr: '╗',
            bl: '╚',
            br: '╝',
            h: '═',
            v: '║',
        },
        BorderStyle::Rounded => BoxChars {
            tl: '╭',
            tr: '╮',
            bl: '╰',
            br: '╯',
            h: '─',
            v: '│',
        },
        BorderStyle::Heavy => BoxChars {
            tl: '┏',
            tr: '┓',
            bl: '┗',
            br: '┛',
            h: '━',
            v: '┃',
        },
        BorderStyle::None => BoxChars {
            tl: ' ',
            tr: ' ',
            bl: ' ',
            br: ' ',
            h: ' ',
            v: ' ',
        },
    }
}

/// Render the top border with entity info embedded.
pub fn top_border(opts: &RenderOptions, parsed: &ParsedDid) -> String {
    let bc = chars_for(opts.border);
    let palette = GlyphPalette::from_did(&parsed.did);

    // Kind badge text
    let kind_label = format!(" {} ", parsed.kind.as_str().to_uppercase());
    let ns_label = format!(" {} ", parsed.namespace);

    let width: usize = 52; // Fixed border width

    let mut out = String::new();

    // Top line: ╭─── HMR ──── l1fe ────╮
    out.push_str(dim());
    out.push(bc.tl);

    // Left segment
    for _ in 0..2 {
        out.push(bc.h);
    }

    // Kind badge (colored)
    out.push_str(reset());
    out.push_str(&bg(&palette.kind_accent));
    out.push_str(bold());
    out.push_str(&kind_label);
    out.push_str(reset());
    out.push_str(dim());

    // Middle segment
    for _ in 0..3 {
        out.push(bc.h);
    }

    // Namespace badge (subtle)
    out.push_str(reset());
    out.push_str(dim());
    out.push_str(&ns_label);
    out.push_str(dim());

    // Right fill
    let used = 2 + kind_label.len() + 3 + ns_label.len();
    let remaining = width.saturating_sub(used + 1);
    for _ in 0..remaining {
        out.push(bc.h);
    }

    out.push(bc.tr);
    out.push_str(reset());
    out.push('\n');

    out
}

/// Render the bottom border.
pub fn bottom_border(opts: &RenderOptions) -> String {
    let bc = chars_for(opts.border);
    let width = 52;

    let mut out = String::new();
    out.push_str(dim());
    out.push(bc.bl);
    for _ in 0..width {
        out.push(bc.h);
    }
    out.push(bc.br);
    out.push_str(reset());
    out.push('\n');

    out
}

/// Render a separator line within the border.
pub fn separator(opts: &RenderOptions) -> String {
    let bc = chars_for(opts.border);
    let width = 52;
    let mut out = String::new();
    out.push_str(dim());
    out.push(bc.v);
    for _ in 0..width {
        out.push(bc.h);
    }
    out.push(bc.v);
    out.push_str(reset());
    out.push('\n');
    out
}
