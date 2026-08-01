//! # Kitty Graphics Protocol Renderer
//!
//! Transmits a PNG image inline using the Kitty graphics protocol.
//! This gives pixel-perfect rendering at native resolution.
//!
//! Protocol: `\x1b_Gf=100,a=T,<key>=<value>;{base64_data}\x1b\\`
//!
//! Supported by: Kitty, WezTerm, Ghostty, Konsole (partial).

use crate::{RenderOptions, Result};
use base64::Engine;

/// Render a PNG image via the Kitty graphics protocol.
pub fn render(png_data: &[u8], _opts: &RenderOptions) -> Result<String> {
    let b64 = base64::engine::general_purpose::STANDARD.encode(png_data);

    // Kitty protocol supports chunked transmission for large images.
    // Each chunk is max 4096 bytes of base64 data.
    let chunk_size = 4096;
    let chunks: Vec<&str> = b64
        .as_bytes()
        .chunks(chunk_size)
        .map(|c| std::str::from_utf8(c).unwrap_or(""))
        .collect();

    let mut output = String::with_capacity(b64.len() + chunks.len() * 64);

    for (i, chunk) in chunks.iter().enumerate() {
        let is_first = i == 0;
        let is_last = i == chunks.len() - 1;
        let more = if is_last { 0 } else { 1 };

        if is_first {
            // First chunk includes the image metadata
            output.push_str(&format!("\x1b_Gf=100,a=T,t=d,m={};{}\x1b\\", more, chunk));
        } else {
            // Continuation chunks
            output.push_str(&format!("\x1b_Gm={};{}\x1b\\", more, chunk));
        }
    }

    output.push('\n');
    Ok(output)
}
