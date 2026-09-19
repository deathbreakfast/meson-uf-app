# Meson UF App

[![CI](https://github.com/unified-field-dev/meson-uf-app/actions/workflows/ci.yml/badge.svg)](https://github.com/unified-field-dev/meson-uf-app/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

[GitHub](https://github.com/unified-field-dev/meson-uf-app) ·
`cargo doc -p meson-app --features ssr --open`

## About

Meson UF App is the Unified Field **My Files** UI under `/meson`. It lists the
signed-in user’s uploads across Valence schemas that opt into Meson’s `File`
trait, with image/text preview.

Domain File metadata, `FileQueryAll`, and `FileByteBackend` live in sibling
[meson](https://github.com/unified-field-dev/meson). This repo mounts the Orbital
pages and Higgs `#[server]` wrappers hosts use.

- **UI (`meson-app`)** — pages, SSR list/get/preview, `MesonRoutes`, `uf_app!`
  registration at app id `meson` / path `/meson`

Hosts supply Valence + auth, enable `ssr` / hydrate to match the host, and call
`meson::install_blob_store` once at boot (same store as `/api/files/*`). Crate-root
rustdoc owns the Features index and mount guide.

## Where things live

| Concern | Location |
|---------|----------|
| Mount routes, Features guide | `meson-app` crate root (`MesonRoutes`) |
| Orbital pages | `meson-app/src/pages/` |
| SSR list / get / preview | `meson-app/src/server/` |
| Shell / app bar | `meson-app/src/shell/`, `layout.rs` |
| File trait + blob store | sibling [meson](https://github.com/unified-field-dev/meson) |
| Local + CI gates | [`docs/VERIFICATION.md`](docs/VERIFICATION.md) |

## Getting started

```toml
[dependencies]
# Pin a release tag or commit SHA — do not use branch = "main".
meson-app = { git = "https://github.com/unified-field-dev/meson-uf-app", package = "meson-app", rev = "<tag-or-sha>", default-features = false }
meson = { git = "https://github.com/unified-field-dev/meson", package = "meson", rev = "<tag-or-sha>", default-features = false, features = ["db-sqlite", "backend-local"] }
```

```rust,ignore
use meson::{blob_store_from_env, install_blob_store};
use meson_app::MesonRoutes;
use leptos::prelude::*;
use leptos_router::components::Routes;

// Host boot (once):
install_blob_store(blob_store_from_env()?)?;

view! {
    <Routes fallback=|| "not found">
        <MesonRoutes />
    </Routes>
}
```

Wire Higgs session + Valence, link File-bearing schemas so they register with
`FileQueryAll`, then mount the routes above. Prefer
`meson::blob_store_from_env` when the host selects LocalDisk or RustFS from
`MESON_*`.

```bash
export CARGO_BUILD_JOBS=1
cargo check -p meson-app --features ssr
```

## In the stack

| Consumer | Role |
|----------|------|
| [unified-field-embedded](https://github.com/unified-field-dev/unified-field-embedded) / [unified-field-remote-fleet](https://github.com/deathbreakfast/unified-field-remote-fleet) | Mount `<MesonRoutes />` at `/meson` |
| [lepton](https://github.com/unified-field-dev/lepton) `lepton-host-adapter` | Upload API uses the same Meson blob store the UI previews |
| Finance (planned) | File-trait uploads surface in My Files once wired |

## Security

My Files list and preview use the signed-in session Valence and owner-scoped
`FileQueryAll`. Foreign or missing file ids resolve as not found. Report
vulnerabilities privately — see [`SECURITY.md`](SECURITY.md). Do not open a
public issue for security-sensitive reports.

## Verify

Full fmt / clippy / check / test gates live in
[`docs/VERIFICATION.md`](docs/VERIFICATION.md). GitHub Actions runs the same
gates, on the `nightly` toolchain, on every PR and push to `main`.

```bash
export CARGO_BUILD_JOBS=1
export RUSTFLAGS="-D warnings"
cargo fmt -p meson-app -- --check
cargo clippy -p meson-app --features ssr --all-targets -- -D warnings -A clippy::pedantic -A clippy::nursery
cargo test -p meson-app --features ssr -- --test-threads=1
```

## FAQ

**Is this a standalone server?** No. `meson-app` mounts under a host `<Routes>`
tree. File metadata and blob storage live in `meson`; hosts supply Valence,
session chrome, and the blob store.

**Do I need this crate for uploads?** No. Upload routes use `meson` (via
`lepton-host-adapter` or host wiring). Depend on `meson-app` when users need
`/meson` My Files.

## License

MIT. See [LICENSE](LICENSE), [CONTRIBUTING.md](CONTRIBUTING.md),
[SECURITY.md](SECURITY.md), and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
