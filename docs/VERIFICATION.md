# Meson uf-app verification

Re-run after code or doc changes.

## Environment

CI runs every job on the `nightly` Rust toolchain (`dtolnay/rust-toolchain@nightly`),
with `wasm32-unknown-unknown` added for the check job.

```bash
export CARGO_BUILD_JOBS=1
export RUSTFLAGS="-D warnings"
```

## Gates

From the repository root, matching `.github/workflows/ci.yml` exactly:

```bash
cargo fmt -p meson-app -- --check
cargo clippy -p meson-app --features ssr --all-targets -- -D warnings -A clippy::pedantic -A clippy::nursery
cargo check -p meson-app --features ssr
cargo check -p meson-app --features hydrate --target wasm32-unknown-unknown
cargo test -p meson-app --features ssr -- --test-threads=1
```

## Monorepo / UF hosts

These notes assume a Unified Field workspace checkout. Skip them from a
standalone `meson-uf-app` clone.

Embedded host e2e: `/meson` and `/meson/files/:id` route contracts in
`unified-field-embedded/end2end` (`meson-my-files-list-happy`,
`meson-my-files-anonymous-gate-sad`, `meson-virus-scan-upload-available-happy`,
`meson-virus-scan-upload-quarantine-sad`, `meson-virus-scan-photon-push-happy`,
`meson-file-detail-image-preview-happy`, `meson-file-detail-not-found-sad`,
`meson-file-detail-peer-id-sad`, `meson-file-detail-anonymous-gate-sad`).

Dedicated lab host e2e (fast, harness-auth, isolated from the embedded
platform): `cargo leptos end-to-end --project meson-uf-app-e2e`
(`pw-meson-my-files-populated-happy`, `pw-meson-file-detail-image-happy`,
`pw-meson-file-detail-text-happy`, `pw-meson-file-detail-peer-id-sad`,
`pw-meson-unauth-gated-sad`, `pw-meson-my-files-error-sad`) — see
`meson-uf-app-e2e/README.md`.

Remote fleet: `MESON_BLOB_BACKEND=rustfs` + `MESON_RUSTFS_*` (see meson
[`docs/VERIFICATION.md`](https://github.com/unified-field-dev/meson/blob/main/docs/VERIFICATION.md);
lab credentials are local-only).
