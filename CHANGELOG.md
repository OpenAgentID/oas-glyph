# Changelog

All notable changes to `oas-glyph` and `oas-glyph-tui` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

The workspace is at version 0.1.0 and has not been published to crates.io.

### Added

- `oas-glyph`: deterministic visual identity for `did:oas` addresses. A DID is
  encoded into a compact payload, protected with Reed-Solomon error correction,
  laid out on a 12×12 multi-channel cell grid with BLAKE3-derived identity
  colours, and rendered to SVG or PNG; the scanner decodes an image back to the
  DID. An axum HTTP API serves rendering and decoding.
- `oas-glyph-tui`: a terminal viewer for glyphs.
- Community health files, CI (format, clippy and tests on every change),
  issue forms, a support policy, a code of conduct and Dependabot updates for
  Cargo and GitHub Actions.

### Changed

- The licence files are `LICENSE-APACHE` and `LICENSE-MIT` only. The separate
  `LICENSE` summary is removed so GitHub detects the dual licence; the README's
  License section states the choice.
