# meson-uf-app-e2e

Leptos host that mounts meson-app's My Files pages for Playwright. Lab-only:
insecure session cookies, `POST /api/test/seed-data`, harness auth (no lepton sign-in).

## Run

```bash
export CARGO_BUILD_JOBS=1
cd meson-uf-app-e2e/end2end && npm ci && npx playwright install chromium && cd ../..
cargo leptos end-to-end --project meson-uf-app-e2e
```

Host listens on `127.0.0.1:3220`. Do not Ctrl-C; the run exits when Playwright finishes.

To drive it interactively instead (build the host, then step through scenarios
by hand in Playwright's UI mode):

```bash
cargo leptos serve --project meson-uf-app-e2e &
cd meson-uf-app-e2e/end2end && npx playwright test --ui
```

The lab host mounts the same page components as `MesonRoutes`, without `Lazy`
(wasm-split `Lazy` under `ParentRoute` panics on hydrate in the current Leptos
pin). Production hosts keep `MesonRoutes` for code-splitting.

Fixtures (two owner files, one peer file) are seeded once at process boot in
`e2e_valence.rs`, not through the upload flow — this host never mounts
`/api/files/upload`. That path is exercised by meson's own quarantine-pipeline
tests and by `unified-field-embedded/end2end`'s `meson-virus-scan.spec.ts`.

## Seed

`POST /api/test/seed-data` with JSON
`{ "auth": "owner" | "peer" | "unverified" | "anonymous" | "broken_session" }`.

Response includes the seeded fixture ids:
`{ "ok": true, "auth": "...", "fixtures": { "owner_image_file_id": "...", "owner_text_file_id": "...", "peer_file_id": "..." } }`.

## Scenarios

- `pw-meson-my-files-populated-happy`
- `pw-meson-file-detail-image-happy`
- `pw-meson-file-detail-text-happy`
- `pw-meson-file-detail-peer-id-sad`
- `pw-meson-unauth-gated-sad`
- `pw-meson-my-files-error-sad`
