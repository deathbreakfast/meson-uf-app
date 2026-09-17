//! Meson My Files app — browse and preview uploads across File-trait schemas.
//!
//! Meson-app mounts authenticated Orbital routes at `/meson` so a signed-in user
//! can list and preview their File-trait uploads under the session Valence
//! (Higgs session actor).
//!
//! # Features
//!
//! - **My Files routes** — Authenticated Orbital UI that lists the session
//!   user's File-trait uploads and opens owned image/text previews at
//!   [`paths::ROOT`] (`/meson`) and `/meson/files/:id`. [Get started](#mount-mesonroutes)
//!
//! # Mount `MesonRoutes`
//!
//! `MesonRoutes` provides the Orbital My Files UI. Mount it during host startup
//! under the Leptos router. Preview loads bytes through
//! [`meson::FileBytes::get_file_bytes`], so the host must
//! [`meson::install_blob_store`] once at boot (same store as upload routes).
//! The list binds Meson `FileQueryAll` to the session `uploaded_by`; preview
//! loads only owned rows (a foreign or missing id is treated as not found).
//!
//! Prerequisites: Higgs session; `ssr` / `hydrate` features aligned to the host;
//! Meson linked so File-bearing schemas register with `FileQueryAll`.
//!
//! 1. Depend on `meson-app` with matching features.
//! 2. Mount `<MesonRoutes />` under host `<Routes>`.
//! 3. Call [`meson::install_blob_store`] once at host boot (often
//!    [`meson::blob_store_from_env`]).
//!
//! ```rust,ignore
//! use meson::{blob_store_from_env, install_blob_store};
//! use meson_app::{paths, MesonRoutes};
//! use leptos::prelude::*;
//! use leptos_router::components::Routes;
//!
//! // Host boot (once):
//! install_blob_store(blob_store_from_env()?)?;
//!
//! view! {
//!     <Routes fallback=|| "not found">
//!         <MesonRoutes />
//!     </Routes>
//! }
//!
//! // After hydrate, signed-in users reach My Files at paths::ROOT.
//! assert_eq!(paths::ROOT, "/meson");
//! ```
//!
//! On success `/meson` shows My Files; `/meson/files/:id` opens preview for owned
//! rows. Anonymous visitors hit the host auth gate.
//!
//! **Next:** use the same installed store for host `/api/files/*` upload routes
//! so metadata keys match stored bytes. Prefer
//! [`meson::blob_store_from_env`] when the host selects `LocalDisk` or `RustFS` from
//! `MESON_*` (see meson [Select blob store from env](../meson/index.html#select-blob-store-from-env)).
//!
//! # Feature flags
//!
//! | Flag | Enables |
//! |------|---------|
//! | `ssr` | Server-side rendering: SSR-only server fns, Higgs/Valence session wiring, `meson` blob-store access. Enable on the host binary. |
//! | `hydrate` | Client-side hydration for the Orbital UI. Enable on the host's wasm/client lib target. |
//!
//! Hosts enable exactly one of `ssr` / `hydrate` per compiled artifact (SSR
//! binary vs. client wasm), matching the leptos-axum split.

#![allow(missing_docs)] // Generated-code exception: `uf_app!`, `#[orbital_routes_extract]`,
                        // `#[uf_product_macros::server]`, and `#[derive(UfPermissionManifest)]` each emit
                        // sibling items (constants, modules, server-fn structs, manifest methods) that don't
                        // inherit doc attributes from the annotated item. Item-level `#[allow(missing_docs)]`
                        // on the macro call site does not suppress these (verified: removing this crate-level
                        // allow still errors on the macro-emitted siblings, not the authored items).

use leptos::prelude::*;
use leptos_router::{
    components::{ParentRoute, Route},
    path, Lazy,
};
use uf_product_macros::uf_app;

pub mod components;
mod layout;
mod lazy_routes;
pub mod pages;
pub mod permissions;
pub mod server;
pub mod shell;
pub mod status;

pub use layout::MesonLayout;
pub use pages::{FileDetailPage, MyFilesPage, MyFilesTable, PreviewPane};

use lazy_routes::{FileDetailRoute, MesonLayoutRouteView, MyFilesIndexRoute};

uf_app! {
    name: "Meson",
    id: "meson",
    description: "My Files — browse uploads across apps",
    icon: "📁",
    version: "0.1.0",
    routes: MesonRoutes,
    route_path: "/meson",
    permission_manifest: permissions::MesonPermission,
}

/// Route tree for Meson: My Files index and file detail/preview.
#[allow(missing_docs)]
#[orbital_macros::orbital_routes_extract]
#[component(transparent)]
pub fn MesonRoutes() -> impl leptos_router::MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("meson") view=MesonLayoutRouteView>
            <Route path=path!("") view={Lazy::<MyFilesIndexRoute>::new()} />
            <Route path=path!("files/:id") view={Lazy::<FileDetailRoute>::new()} />
        </ParentRoute>
    }
    .into_inner()
}
