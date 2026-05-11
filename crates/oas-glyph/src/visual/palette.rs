//! # Identity-Derived Color Palette
//!
//! Every OAS entity gets a unique, deterministic color palette derived from
//! the BLAKE3 hash of their DID. The palette consists of:
//!
//! - A primary hue (derived from hash bytes 0-3)
//! - A secondary hue (derived from hash bytes 4-7)
//! - Saturation range (hash bytes 8-9)
//! - Lightness range (hash bytes 10-11)
//! - Four hue-shifted variants for the data cell encoding

/// An RGBA color.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Convert to CSS hex string.
    pub fn to_hex(&self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }

    /// Create from HSL values.
    pub fn from_hsl(h: f64, s: f64, l: f64) -> Self {
        let (r, g, b) = hsl_to_rgb(h, s, l);
        Self::rgb(
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8,
        )
    }

    /// Lerp between two colors.
    pub fn lerp(a: &Color, b: &Color, t: f64) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            r: (a.r as f64 + (b.r as f64 - a.r as f64) * t) as u8,
            g: (a.g as f64 + (b.g as f64 - a.g as f64) * t) as u8,
            b: (a.b as f64 + (b.b as f64 - a.b as f64) * t) as u8,
            a: (a.a as f64 + (b.a as f64 - a.a as f64) * t) as u8,
        }
    }
}

/// A complete glyph color palette derived from identity.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GlyphPalette {
    /// Background color
    pub background: Color,
    /// Primary identity color
    pub primary: Color,
    /// Secondary identity color
    pub secondary: Color,
    /// Four hue-shifted data colors (mapped to hue_shift 0-3)
    pub data_colors: [Color; 4],
    /// Finder pattern colors (high contrast)
    pub finder_dark: Color,
    pub finder_light: Color,
    /// Kind region accent
    pub kind_accent: Color,
}

impl GlyphPalette {
    /// Derive a palette from a DID string.
    pub fn from_did(did: &str) -> Self {
        let hash = blake3::hash(did.as_bytes());
        let bytes = hash.as_bytes();

        // Primary hue: first 4 bytes → 0-360°
        let primary_hue = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64
            / u32::MAX as f64
            * 360.0;

        // Secondary hue: next 4 bytes, offset by 60-180° from primary
        let secondary_offset = 60.0
            + (u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as f64
                / u32::MAX as f64
                * 120.0);
        let secondary_hue = (primary_hue + secondary_offset) % 360.0;

        // Saturation: 50-90%
        let saturation = 0.50 + (bytes[8] as f64 / 255.0) * 0.40;

        // Lightness base: 35-55%
        let lightness = 0.35 + (bytes[9] as f64 / 255.0) * 0.20;

        let primary = Color::from_hsl(primary_hue, saturation, lightness);
        let secondary = Color::from_hsl(secondary_hue, saturation, lightness);

        // Data colors: four hue shifts around primary
        let data_colors = [
            Color::from_hsl(primary_hue, saturation, lightness + 0.15),
            Color::from_hsl((primary_hue + 30.0) % 360.0, saturation, lightness),
            Color::from_hsl((primary_hue + 60.0) % 360.0, saturation, lightness - 0.10),
            Color::from_hsl(secondary_hue, saturation * 0.8, lightness + 0.05),
        ];

        // Background: very dark desaturated version of primary
        let background = Color::from_hsl(primary_hue, saturation * 0.15, 0.08);

        // Finder patterns: always high contrast (near white/dark)
        let finder_dark = Color::from_hsl(primary_hue, saturation * 0.3, 0.12);
        let finder_light = Color::from_hsl(primary_hue, saturation * 0.2, 0.85);

        // Kind accent: bright saturated version
        let kind_accent = Color::from_hsl(secondary_hue, 0.9, 0.6);

        Self {
            background,
            primary,
            secondary,
            data_colors,
            finder_dark,
            finder_light,
            kind_accent,
        }
    }
}

/// Convert HSL to RGB (all values 0.0-1.0, hue 0-360).
fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    if s == 0.0 {
        return (l, l, l);
    }

    let h = h / 360.0;
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;

    let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h);
    let b = hue_to_rgb(p, q, h - 1.0 / 3.0);

    (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
}

fn hue_to_rgb(p: f64, q: f64, mut t: f64) -> f64 {
    if t < 0.0 { t += 1.0; }
    if t > 1.0 { t -= 1.0; }
    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 1.0 / 2.0 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_deterministic() {
        let did = "did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK";
        let p1 = GlyphPalette::from_did(did);
        let p2 = GlyphPalette::from_did(did);
        assert_eq!(p1.primary.r, p2.primary.r);
        assert_eq!(p1.primary.g, p2.primary.g);
        assert_eq!(p1.primary.b, p2.primary.b);
    }

    #[test]
    fn different_dids_different_palettes() {
        let p1 = GlyphPalette::from_did("did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK");
        let p2 = GlyphPalette::from_did("did:oas:l1fe:hmr:z6MkiTBz1ymuepAQ4HEHYSF1H8quG5GLVVQR3djdX3mDooWp");
        // Very unlikely to be identical
        assert!(
            p1.primary.r != p2.primary.r
                || p1.primary.g != p2.primary.g
                || p1.primary.b != p2.primary.b
        );
    }

    /// Compute Euclidean distance in HSL-approximated space using RGB.
    fn color_distance(a: &Color, b: &Color) -> f64 {
        let dr = a.r as f64 - b.r as f64;
        let dg = a.g as f64 - b.g as f64;
        let db = a.b as f64 - b.b as f64;
        (dr * dr + dg * dg + db * db).sqrt()
    }

    #[test]
    fn distinct_palettes_for_each_kind() {
        let kinds = [
            "hmr", "mhr", "ao", "agent", "tool",
            "skill", "workflow", "model", "dataset", "service",
        ];
        let palettes: Vec<GlyphPalette> = kinds
            .iter()
            .map(|k| {
                let did = format!(
                    "did:oas:l1fe:{}:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
                    k
                );
                GlyphPalette::from_did(&did)
            })
            .collect();

        // Check that no two palettes have identical primary colors
        for i in 0..palettes.len() {
            for j in (i + 1)..palettes.len() {
                let dist = color_distance(&palettes[i].primary, &palettes[j].primary);
                assert!(
                    dist > 1.0,
                    "palettes for {} and {} are too similar (dist={})",
                    kinds[i],
                    kinds[j],
                    dist
                );
            }
        }
    }

    #[test]
    fn palette_has_four_data_colors() {
        let p = GlyphPalette::from_did("did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK");
        assert_eq!(p.data_colors.len(), 4);
    }

    #[test]
    fn data_colors_visually_distinct() {
        let p = GlyphPalette::from_did("did:oas:l1fe:agent:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK");
        // At least some of the 4 data colors should differ
        let mut all_same = true;
        for i in 1..4 {
            if color_distance(&p.data_colors[0], &p.data_colors[i]) > 5.0 {
                all_same = false;
                break;
            }
        }
        assert!(!all_same, "data colors should not all be the same");
    }

    #[test]
    fn color_hex_format_opaque() {
        let c = Color::rgb(0xAB, 0xCD, 0xEF);
        let hex = c.to_hex();
        assert_eq!(hex, "#abcdef");
        assert_eq!(hex.len(), 7); // # + 6 hex digits
    }

    #[test]
    fn color_hex_format_transparent() {
        let c = Color::rgba(0x11, 0x22, 0x33, 0x80);
        let hex = c.to_hex();
        assert_eq!(hex, "#11223380");
    }

    #[test]
    fn color_lerp_midpoint() {
        let a = Color::rgb(0, 0, 0);
        let b = Color::rgb(100, 200, 50);
        let mid = Color::lerp(&a, &b, 0.5);
        assert_eq!(mid.r, 50);
        assert_eq!(mid.g, 100);
        assert_eq!(mid.b, 25);
    }

    #[test]
    fn color_lerp_clamp() {
        let a = Color::rgb(100, 100, 100);
        let b = Color::rgb(200, 200, 200);
        let clamped = Color::lerp(&a, &b, 1.5); // t > 1.0 should clamp
        assert_eq!(clamped.r, 200);
    }

    #[test]
    fn hsl_to_rgb_achromatic() {
        // s=0 should yield grey
        let (r, g, b) = hsl_to_rgb(180.0, 0.0, 0.5);
        assert!((r - 0.5).abs() < 0.01);
        assert!((g - 0.5).abs() < 0.01);
        assert!((b - 0.5).abs() < 0.01);
    }

    #[test]
    fn hsl_red() {
        let c = Color::from_hsl(0.0, 1.0, 0.5);
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 0);
        assert_eq!(c.b, 0);
    }
}
