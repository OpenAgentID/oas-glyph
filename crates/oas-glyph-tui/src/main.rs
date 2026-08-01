use clap::{Parser, Subcommand, ValueEnum};
use oas_glyph_tui::*;

#[derive(Parser)]
#[command(
    name = "oas-glyph",
    about = "Render and inspect OpenAgent Glyphs — visual identity for OAS entities",
    version,
    long_about = "OpenAgent Glyph renders scannable identity avatars for did:oas entities \
                  directly in your terminal. Auto-detects the best rendering backend \
                  (Kitty > iTerm2 > Sixel > half-block > Braille > ASCII)."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Render a glyph for a DID
    Render {
        /// The did:oas identifier
        did: String,

        /// Force a specific renderer
        #[arg(short, long, value_enum)]
        renderer: Option<RendererArg>,

        /// Render size in character columns
        #[arg(short, long, default_value = "0")]
        size: u16,

        /// Hide the DID label
        #[arg(long)]
        no_did: bool,

        /// Hide the entity kind badge
        #[arg(long)]
        no_kind: bool,

        /// Show the color palette strip
        #[arg(long)]
        palette: bool,

        /// Compact single-line rendering
        #[arg(short, long)]
        compact: bool,

        /// Border style
        #[arg(short, long, value_enum, default_value = "rounded")]
        border: BorderArg,
    },

    /// Show a gallery of glyphs for all entity kinds
    Gallery {
        /// Namespace to use
        #[arg(short, long, default_value = "l1fe")]
        namespace: String,

        /// Force a specific renderer
        #[arg(short, long, value_enum)]
        renderer: Option<RendererArg>,
    },

    /// Inspect encoding details for a DID
    Inspect {
        /// The did:oas identifier
        did: String,
    },

    /// Print terminal capability diagnostics
    Diag,

    /// Show the derived color palette for a DID
    Palette {
        /// The did:oas identifier
        did: String,
    },
}

#[derive(Clone, ValueEnum)]
enum RendererArg {
    Kitty,
    Iterm2,
    Sixel,
    HalfBlock,
    Braille,
    Ascii,
}

impl From<RendererArg> for GraphicsCapability {
    fn from(r: RendererArg) -> Self {
        match r {
            RendererArg::Kitty => Self::Kitty,
            RendererArg::Iterm2 => Self::Iterm2,
            RendererArg::Sixel => Self::Sixel,
            RendererArg::HalfBlock => Self::HalfBlock,
            RendererArg::Braille => Self::Braille,
            RendererArg::Ascii => Self::Ascii,
        }
    }
}

#[derive(Clone, ValueEnum)]
enum BorderArg {
    None,
    Single,
    Double,
    Rounded,
    Heavy,
}

impl From<BorderArg> for BorderStyle {
    fn from(b: BorderArg) -> Self {
        match b {
            BorderArg::None => Self::None,
            BorderArg::Single => Self::Single,
            BorderArg::Double => Self::Double,
            BorderArg::Rounded => Self::Rounded,
            BorderArg::Heavy => Self::Heavy,
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Render {
            did,
            renderer,
            size,
            no_did,
            no_kind,
            palette,
            compact,
            border,
        } => {
            let opts = RenderOptions {
                size: if compact {
                    GlyphSize::Compact
                } else if size > 0 {
                    GlyphSize::Cells(size, size)
                } else {
                    GlyphSize::Auto
                },
                force_renderer: renderer.map(GraphicsCapability::from),
                show_did: !no_did,
                show_kind: !no_kind,
                show_palette: palette,
                border: border.into(),
                compact,
            };

            if let Err(e) = render_to_stdout(&did, &opts) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }

        Commands::Gallery {
            namespace,
            renderer,
        } => {
            let kinds = [
                "hmr", "mhr", "ao", "agent", "tool", "skill", "workflow", "model", "dataset",
                "service",
            ];

            let cap = renderer
                .map(GraphicsCapability::from)
                .unwrap_or_else(detect::detect_capability);

            println!("\n  OpenAgent Glyph Gallery — namespace: {}\n", namespace);
            println!("  Renderer: {}\n", cap.label());

            for kind in &kinds {
                let did = format!(
                    "did:oas:{}:{}:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
                    namespace, kind
                );

                let opts = RenderOptions {
                    size: GlyphSize::Cells(24, 24),
                    force_renderer: Some(cap),
                    show_did: false,
                    show_kind: true,
                    show_palette: false,
                    border: BorderStyle::Rounded,
                    compact: false,
                };

                match render_to_string(&did, &opts, cap) {
                    Ok(output) => print!("{}", output),
                    Err(e) => eprintln!("  Error rendering {}: {}", kind, e),
                }
                println!();
            }
        }

        Commands::Inspect { did } => match inspect_did(&did) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        },

        Commands::Diag => {
            detect::print_diagnostics();
        }

        Commands::Palette { did } => {
            let palette = oas_glyph::visual::GlyphPalette::from_did(&did);
            println!();
            println!("  Palette for: {}", did);
            println!();
            print_color_swatch("  Background ", &palette.background);
            print_color_swatch("  Primary    ", &palette.primary);
            print_color_swatch("  Secondary  ", &palette.secondary);
            for (i, c) in palette.data_colors.iter().enumerate() {
                print_color_swatch(&format!("  Data {}     ", i), c);
            }
            print_color_swatch("  Finder dk  ", &palette.finder_dark);
            print_color_swatch("  Finder lt  ", &palette.finder_light);
            print_color_swatch("  Kind accent", &palette.kind_accent);
            println!();
        }
    }
}

fn inspect_did(did: &str) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let parsed = oas_glyph::ParsedDid::parse(did)?;
    let payload = oas_glyph::encoding::encode_did_payload(&parsed)?;
    let ecc_info = oas_glyph::encoding::ecc_info(payload.len());
    let codeword = oas_glyph::encoding::encode_with_ecc(&payload)?;
    let grid = oas_glyph::encoding::GlyphGrid::from_codeword(&codeword, parsed.kind)?;

    println!();
    println!("  ╭─ Glyph Encoding Inspector ──────────────╮");
    println!("  │                                          │");
    println!("  │  DID:        {}…{}", &did[..20], &did[did.len() - 8..]);
    println!("  │  Namespace:  {:>28} │", parsed.namespace);
    println!("  │  Kind:       {:>28} │", parsed.kind.as_str());
    println!(
        "  │  Identifier: {}…{}",
        &parsed.identifier[..8],
        &parsed.identifier[parsed.identifier.len() - 6..]
    );
    println!("  │                                          │");
    println!("  │  Grid:       {:>22}×{} cells │", 12, 12);
    println!("  │  Data cells: {:>28} │", grid.data_cell_count);
    println!("  │  Capacity:   {:>24} bits │", grid.capacity_bits());
    println!("  │                                          │");
    println!("  │  Payload:    {:>24} bytes │", ecc_info.data_bytes);
    println!("  │  ECC parity: {:>24} bytes │", ecc_info.parity_bytes);
    println!("  │  Codeword:   {:>24} bytes │", ecc_info.total_bytes);
    println!(
        "  │  Correction: {:>25.0}% │",
        ecc_info.correction_capacity_percent
    );
    println!("  │                                          │");
    println!("  │  Payload hex:                            │");

    // Print payload in hex rows
    for chunk in payload.chunks(16) {
        let hex: String = chunk.iter().map(|b| format!("{:02x} ", b)).collect();
        println!("  │    {:<40} │", hex.trim());
    }

    println!("  │                                          │");
    println!("  ╰──────────────────────────────────────────╯");
    println!();

    Ok(())
}

fn print_color_swatch(label: &str, c: &oas_glyph::visual::palette::Color) {
    println!(
        "{} \x1b[48;2;{};{};{}m      \x1b[0m  {}",
        label,
        c.r,
        c.g,
        c.b,
        c.to_hex()
    );
}
