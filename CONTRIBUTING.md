# Contributing to Meson UF App

Thank you for improving this project.

## Development setup

1. Clone [unified-field-dev/meson-uf-app](https://github.com/unified-field-dev/meson-uf-app)
2. Install Rust `nightly` (matches CI) with the `wasm32-unknown-unknown` target
   for hydrate checks
3. From the repository root:

```bash
export CARGO_BUILD_JOBS=1
export RUSTFLAGS="-D warnings"
cargo fmt -p meson-app -- --check
cargo clippy -p meson-app --features ssr --all-targets -- -D warnings -A clippy::pedantic -A clippy::nursery
cargo check -p meson-app --features ssr
cargo test -p meson-app --features ssr -- --test-threads=1
```

Full gates: [`docs/VERIFICATION.md`](docs/VERIFICATION.md).

## Code of conduct

Participation is governed by [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md). Security
reports: [`SECURITY.md`](SECURITY.md).

## Pull requests

- Prefer small, focused PRs.
- Update [`README.md`](README.md) when user-facing flows or host mounting steps change.
