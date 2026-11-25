//! The `meson_storage_daily` Spectra table and the row contract its writers follow.
//!
//! A daily Chronon job (one for the embedded host, one for the remote fleet)
//! measures how much space Meson files use and writes one snapshot per run to
//! this table. The Storage page reads the newest snapshot through
//! [`super::snapshot::latest_snapshot`] and the last 30 days through
//! [`super::snapshot::history`]. Nothing in this crate writes the table.
//!
//! # Row contract
//!
//! Every row of one run carries the same `ds`, `measured_at` and `run_id`.
//!
//! | Field | Type | Meaning |
//! |-------|------|---------|
//! | `ds` | `String` | The UTC day the snapshot covers, `YYYY-MM-DD`. The partition key. |
//! | `measured_at` | `String` | When the run measured, RFC 3339 in UTC (`2026-11-24T03:00:12Z`). |
//! | `run_id` | `String` | Unique per run. A rerun for the same `ds` writes a new `run_id`. |
//! | `owner_kind` | `String` | `site` for site-wide rows, else the Valence owner table (`user`, `service_user`, `team`, ...). |
//! | `owner_id` | `String` | `all` for site rows, else the owner's record key without the table prefix (`abc123`, not `user:abc123`). PII. |
//! | `app_table` | `String` | The File table the files live in (`receipt_scan`), or `*` for all apps. |
//! | `file_status` | `String` | `available`, `pending`, `quarantined`, or `*` for all. |
//! | `files` | `i64` | Number of File rows counted. |
//! | `bytes` | `i64` | Sum of their recorded sizes, before copies. |
//! | `missing_size_files` | `i64` | Files with no recorded size, counted as 0 bytes. |
//! | `copies_reported` | `i64` | How many copies the blob store says it keeps. Set on `site` rows only, and only when the store can say; write `0` everywhere else. |
//!
//! Spectra event fields are `String`, `i64`, `f64` or `bool`, so the time is an
//! RFC 3339 string and "not reported" is `0` rather than a null. Write each
//! row with [`MesonStorageDailyLogger::log_at`] and pass the run's
//! `measured_at` as the timestamp.
//!
//! Each run writes these rows:
//!
//! | Row | `owner_kind` | `owner_id` | `app_table` | `file_status` |
//! |-----|--------------|------------|-------------|---------------|
//! | Site total (exactly one) | `site` | `all` | `*` | `*` |
//! | Site by app (one per File table) | `site` | `all` | table | `*` |
//! | Site by status (one per status) | `site` | `all` | `*` | status |
//! | Owner total (one per owner with files) | kind | key | `*` | `*` |
//! | Owner by app (one per owner and File table) | kind | key | table | `*` |
//!
//! An owner with no files gets no rows; readers treat a missing owner as zero.
//! A run that finds no files at all still writes the site total with zeros, so
//! readers can tell "measured, empty" from "never measured".
//!
//! Readers take the newest `ds` and, within it, the `run_id` of the newest
//! row, so a rerun replaces the earlier run for that day without a delete.

// The generated logger takes one argument per field.
#[allow(clippy::too_many_arguments)]
#[path = "../../schemas/meson_storage_daily_spectra_schema.rs"]
mod meson_storage_daily;

pub use meson_storage_daily::*;

/// Spectra table name for the daily snapshot.
pub const TABLE: &str = "meson_storage_daily";

/// `owner_kind` of site-wide rows.
pub const SITE_KIND: &str = "site";

/// `owner_id` of site-wide rows.
pub const SITE_ID: &str = "all";

/// `app_table` / `file_status` value meaning "all".
pub const ALL: &str = "*";
