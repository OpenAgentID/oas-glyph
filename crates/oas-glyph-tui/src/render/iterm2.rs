//! # iTerm2 Inline Image Renderer
//!
//! Uses iTerm2's proprietary inline image protocol:
//! `\x1b]1337;File=inline=1;size={size};width=auto;height=auto:{base64}\x07`
//!
//! Supported by: iTerm2, WezTerm (via iTerm2 compatibility), Hyper.

use crate::{RenderOptions, Result};
use base64::Engine;

/// Render a PNG image via the iTerm2 inline image protocol.
pub fn render(png_data: &[u8], _opts: &RenderOptions) -> Result<String> {
    let b64 = base64::engine::general_purpose::STANDARD.encode(png_data);
    let size = png_data.len();

    Ok(format!(
        "\x1b]1337;File=inline=1;size={};width=auto;height=auto;preserveAspectRatio=1:{}\x07\n",
        size, b64
    ))
}
