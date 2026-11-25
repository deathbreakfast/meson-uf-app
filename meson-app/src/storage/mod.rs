//! Data layer for the Storage page: the daily snapshot table, its reader, the
//! site cost settings and the view the page renders.
//!
//! - [`schema`]: the `meson_storage_daily` Spectra table and the row contract
//!   the daily Chronon jobs write against.
//! - [`policy`]: the Gauge `StorageAdmin` write rule on the settings row.
//! - [`snapshot`]: [`snapshot::latest_snapshot`] and [`snapshot::history`].
//! - [`settings`]: the `meson_storage_settings` row (copies override, price).
//! - [`view`]: [`view::StorageView`], with a source for every input so the
//!   page can say what's measured, assumed or missing.
//!
//! The server fns in [`crate::server`] tie these together.

#[cfg(feature = "ssr")]
pub mod policy;
#[cfg(feature = "ssr")]
pub mod schema;
pub mod settings;
#[cfg(feature = "ssr")]
pub mod snapshot;
pub mod view;

#[cfg(all(test, feature = "ssr"))]
pub(crate) mod test_support;

/// Logical database the settings schema lives in (embedded SQLite `default`,
/// same as Meson's File tables).
#[cfg(feature = "ssr")]
pub const DEFAULT_STORAGE: valence::DatabaseFromEngine =
    valence::Database::from_engine("default", valence::SQLITE_ENGINE_ID);
