//! Shared Valence + blob-store boot helpers for meson-app integration tests.

#![allow(dead_code)]
#![allow(clippy::expect_used, clippy::unwrap_used)]

pub mod ui_render;

use chrono::Utc;
use meson::generated::{E2eMesonFile, FileFileStatus};
use meson::touch_schema_inventory;
use std::sync::{Arc, OnceLock};
use valence::{
    register_backend_logical_names, Actor, DatabaseBackend, DatabaseRouter, Model, RecordId,
    RegisterBackendLogicalNamesOptions, SqliteBackend, Valence, SQLITE_ENGINE_ID,
};

pub const OWNER_USER_ID: &str = "meson-app-owner";
pub const PEER_USER_ID: &str = "meson-app-peer";

/// Serialize tests that mutate the process-wide [`meson::install_blob_store`].
pub async fn test_lock() -> tokio::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

pub fn owner_rid() -> RecordId {
    RecordId::new("user", OWNER_USER_ID)
}

pub fn peer_rid() -> RecordId {
    RecordId::new("user", PEER_USER_ID)
}

pub async fn setup_valence() -> Valence {
    valence::deletion::register_noop_deletion_dispatcher_for_tests();
    valence::clear_for_test();
    touch_schema_inventory();

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
            operation: "meson_app_test".to_string(),
        })
        .build()
        .expect("build valence");
    valence
        .sync_typed_tables_from_registry()
        .await
        .expect("sync_typed_tables_from_registry");
    valence
}

pub fn as_user(base: &Valence, user_id: &str) -> Valence {
    base.with_actor(Actor::User {
        user_id: user_id.to_string(),
    })
}

/// Seed one `E2eMesonFile` row owned by `uploader`.
///
/// `bare_id` is the record id **without** the `e2e_meson_file:` table prefix
/// (matching `upsert`'s own convention) — returns the full `table:id`
/// string the row actually landed under, for use with [`get_file_for_user`]-
/// style lookups that expect the full form.
pub async fn seed_file(
    valence: &Valence,
    bare_id: &str,
    uploader: RecordId,
    file_name: &str,
    mime: &str,
    storage_path: &str,
    size_bytes: i64,
) -> String {
    let extension = file_name.rsplit('.').next().unwrap_or("bin").to_string();
    let row = E2eMesonFile::new(
        file_name.to_string(),
        extension,
        mime.to_string(),
        size_bytes,
        storage_path.to_string(),
        FileFileStatus::Available,
        uploader,
        Utc::now(),
    )
    .expect("E2eMesonFile::new");
    let created = E2eMesonFile::upsert(bare_id, row, valence, valence::use_!(r"**Test:** Fixture **E2e Meson File** save for `meson-app tests/common` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."))
        .await
        .expect("upsert e2e_meson_file");
    created
        .id()
        .map_or_else(|| bare_id.to_string(), std::string::ToString::to_string)
}
