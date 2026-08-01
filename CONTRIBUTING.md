# Contributing to OpenAgent Glyph

OpenAgent Glyph is the visual transport layer for `did:oas` identities. Changes
must preserve deterministic encoding, round-trip decoding, and compatibility
with the Open Agent Specification.

## Before You Start

- Search existing issues and pull requests before opening a new one.
- Use a bug report for incorrect behavior and a feature request for proposed
  capabilities.
- Report vulnerabilities privately according to [SECURITY.md](SECURITY.md).

For substantial protocol, payload, or compatibility changes, open an issue
before implementation so maintainers can confirm the intended contract.

## Development

Install the stable Rust toolchain with `rustfmt` and `clippy`, then run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace --no-fail-fast
```

Tests should cover valid round trips, malformed input, and boundary conditions.
Changes to the encoded payload or grid layout must document compatibility
impact in `ARCHITECTURE.md`.

## Pull Requests

- Keep each pull request focused and explain why the change is needed.
- Add or update tests for behavior changes.
- Update user-facing documentation when commands, formats, or APIs change.
- Ensure all verification commands pass without warnings.
- Use conventional commit messages such as `feat:`, `fix:`, `docs:`, or
  `chore:`.

## License

The project is licensed under Apache-2.0 OR MIT. By contributing, you agree
that your contribution is licensed under both terms.
