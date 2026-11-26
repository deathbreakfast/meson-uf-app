//! Unit-test helpers: a mem Spectra router, row builders that follow the row
//! contract, and an in-memory Valence with the crate's schemas.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::Arc;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde_json::json;
use spectra::spectra_core::{EventStorageBackend, SharedEventBackend, SharedMetricsBackend};
use spectra::{MemEventsBackend, MemMetricsBackend, SpectraRouter};
use valence::{
    register_backend_logical_names, Actor, DatabaseBackend, DatabaseRouter,
    RegisterBackendLogicalNamesOptions, SqliteBackend, Valence, SQLITE_ENGINE_ID,
};

use super::schema::{ALL, SITE_ID, SITE_KIND, TABLE};

/// A router whose default event backend is a fresh [`MemEventsBackend`].
///
/// Built with `with_defaults`, not `Spectra::builder()`, so tests never touch
/// the process-wide router.
pub fn mem_router() -> (SpectraRouter, Arc<MemEventsBackend>) {
    let events = Arc::new(MemEventsBackend::new());
    let router = SpectraRouter::with_defaults(
        Arc::new(MemMetricsBackend::new()) as SharedMetricsBackend,
        Arc::clone(&events) as SharedEventBackend,
    );
    (router, events)
}

/// One row to write, defaulting to an owner total.
pub struct RowSpec {
    pub ds: String,
    pub run_id: String,
    pub owner_kind: String,
    pub owner_id: String,
    pub app_table: String,
    pub file_status: String,
    pub files: i64,
    pub bytes: i64,
    pub missing_size_files: i64,
    pub copies_reported: i64,
    /// Measured at `ds` 00:00 UTC plus this many hours.
    pub hours_after_midnight: i64,
}

impl RowSpec {
    fn new(
        ds: &str,
        run_id: &str,
        kind: &str,
        id: &str,
        app: &str,
        status: &str,
        bytes: i64,
    ) -> Self {
        Self {
            ds: ds.to_string(),
            run_id: run_id.to_string(),
            owner_kind: kind.to_string(),
            owner_id: id.to_string(),
            app_table: app.to_string(),
            file_status: status.to_string(),
            files: 1,
            bytes,
            missing_size_files: 0,
            copies_reported: 0,
            hours_after_midnight: 3,
        }
    }

    pub fn site_total(ds: &str, run_id: &str, bytes: i64) -> Self {
        Self::new(ds, run_id, SITE_KIND, SITE_ID, ALL, ALL, bytes)
    }

    pub fn site_by_app(ds: &str, run_id: &str, app: &str, bytes: i64) -> Self {
        Self::new(ds, run_id, SITE_KIND, SITE_ID, app, ALL, bytes)
    }

    pub fn site_by_status(ds: &str, run_id: &str, status: &str, bytes: i64) -> Self {
        Self::new(ds, run_id, SITE_KIND, SITE_ID, ALL, status, bytes)
    }

    pub fn owner_total(ds: &str, run_id: &str, user_id: &str, bytes: i64) -> Self {
        Self::new(ds, run_id, "user", user_id, ALL, ALL, bytes)
    }

    pub fn owner_by_app(ds: &str, run_id: &str, user_id: &str, app: &str, bytes: i64) -> Self {
        Self::new(ds, run_id, "user", user_id, app, ALL, bytes)
    }

    fn measured_at(&self) -> DateTime<Utc> {
        let day = NaiveDate::parse_from_str(&self.ds, "%Y-%m-%d")
            .unwrap_or_else(|_| NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        day.and_hms_opt(0, 0, 0).unwrap().and_utc() + Duration::hours(self.hours_after_midnight)
    }
}

/// Append `row` the way a Chronon job would (`ts` = `measured_at`).
pub async fn write_row(events: &MemEventsBackend, row: RowSpec) {
    let measured_at = row.measured_at();
    let fields = json!({
        "ds": row.ds,
        "measured_at": measured_at.to_rfc3339(),
        "run_id": row.run_id,
        "owner_kind": row.owner_kind,
        "owner_id": row.owner_id,
        "app_table": row.app_table,
        "file_status": row.file_status,
        "files": row.files,
        "bytes": row.bytes,
        "missing_size_files": row.missing_size_files,
        "copies_reported": row.copies_reported,
    });
    events
        .append_row(TABLE, &fields, measured_at, None)
        .await
        .expect("append_row");
}

/// In-memory SQLite Valence (System actor) with every linked schema synced.
pub async fn setup_valence() -> Valence {
    valence::deletion::register_noop_deletion_dispatcher_for_tests();
    valence::clear_for_test();
    meson::touch_schema_inventory();
    let _ = std::any::type_name::<crate::generated::MesonStorageSettings>();

    if std::env::var_os("VALENCE_OWNERSHIP_UNIFIED_FETCH").is_none() {
        // SAFETY: test harness only; OnceLock reads this before first ownership get.
        unsafe {
            std::env::set_var("VALENCE_OWNERSHIP_UNIFIED_FETCH", "0");
        }
    }

    let backend: Arc<dyn DatabaseBackend> = Arc::new(
        SqliteBackend::connect_memory()
            .await
            .expect("SqliteBackend::connect_memory"),
    );
    let mut router = DatabaseRouter::new();
    register_backend_logical_names(
        &mut router,
        backend,
        &["default"],
        RegisterBackendLogicalNamesOptions::default(),
    );
    let valence = Valence::builder()
        .database_router(Arc::new(router))
        .default_backend_key(valence::router_key("default", SQLITE_ENGINE_ID))
        .with_actor(Actor::System {
            operation: "meson_app_storage_test".to_string(),
        })
        .build()
        .expect("build valence");
    valence
        .sync_typed_tables_from_registry()
        .await
        .expect("sync_typed_tables_from_registry");
    valence
}

/// Make `user_id` a Gauge Super User, which `StorageAdmin` checks accept.
///
/// `system` must be the System-actor Valence from [`setup_valence`].
pub async fn grant_storage_admin(system: &Valence, user_id: &str) {
    use valence::Model;
    let ensured = gauge::super_user::ensure_super_user_group(system)
        .await
        .expect("ensure super user group");
    let principal = gauge::generated::PermissionUserPrincipal::upsert(
        &format!("user:{user_id}"),
        gauge::generated::PermissionUserPrincipal::new(
            valence::RecordId::new("user", user_id),
            user_id.to_string(),
        )
        .expect("new user principal"),
        system,
        valence::use_!(r"**Test:** Fixture **Permission User Principal** save so the storage settings tests can act as a site owner. CI and developers running the suite only."),
    )
    .await
    .expect("upsert user principal");
    ensured
        .relate_to_member_record(
            principal.id().expect("principal id"),
            system,
            valence::use_!(r"**Test:** Fixture member-edge relate so the storage settings tests can act as a site owner. CI and developers running the suite only."),
        )
        .await
        .expect("relate member");
}
