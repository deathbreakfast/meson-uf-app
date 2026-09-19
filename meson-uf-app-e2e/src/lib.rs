//! Meson `MesonRoutes` Playwright host.
#![allow(missing_docs)]
// Lab-only harness: boot/seed failures should panic immediately rather than
// propagate `Result`s through test setup that has no caller to recover.
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod app;
#[cfg(feature = "ssr")]
mod e2e_valence;
mod gate_demos;
mod harness_auth_menu;
mod meson_routes_eager;
#[cfg(feature = "ssr")]
pub mod seed;

pub use app::{shell, App};
#[cfg(feature = "ssr")]
pub use e2e_valence::{e2e_higgs_config, e2e_router, init_e2e_valence};
#[cfg(feature = "ssr")]
pub use gate_demos::inject_e2e_session_snapshot;
