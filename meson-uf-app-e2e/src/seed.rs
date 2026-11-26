//! Harness-only seed endpoint for Playwright.

use std::sync::{Arc, OnceLock};

use axum::http::StatusCode;
use axum::Json;
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::json;
use spectra::spectra_core::{EventStorageBackend, SharedEventBackend, SharedMetricsBackend};
use spectra::{MemEventsBackend, MemMetricsBackend, SpectraRouter};
use valence::{Model, RecordId};

use crate::e2e_valence::{e2e_fixtures, e2e_system_valence};
use crate::gate_demos::{write_e2e_auth_kind, E2eAuthKind};

#[derive(Debug, Deserialize)]
pub struct SeedRequest {
    /// `anonymous` | `owner` | `peer` | `unverified` | `broken_session` | `no_files_read` | `site_owner`
    #[serde(default = "default_auth")]
    pub auth: String,
    /// Replaces the `meson_storage_daily` rows and the cost settings when set.
    #[serde(default)]
    pub storage: Option<StorageSeed>,
}

fn default_auth() -> String {
    E2eAuthKind::Anonymous.as_str().to_string()
}

/// What the Storage specs put in Spectra and the settings row.
#[derive(Debug, Deserialize)]
pub struct StorageSeed {
    /// Daily runs to write; none leaves the table empty.
    #[serde(default)]
    pub partitions: Vec<PartitionSeed>,
    /// Cost settings; `None` saves a row with no copies and no price.
    #[serde(default)]
    pub settings: Option<SettingsSeed>,
}

/// One daily run: every row shares its `ds`, `measured_at` and `run_id`.
#[derive(Debug, Deserialize)]
pub struct PartitionSeed {
    /// How long ago the run measured; `ds` is that moment's UTC date.
    pub hours_old: i64,
    pub run_id: String,
    pub rows: Vec<RowSeed>,
}

/// One `meson_storage_daily` row (see meson-app `storage::schema`).
#[derive(Debug, Deserialize)]
pub struct RowSeed {
    pub owner_kind: String,
    pub owner_id: String,
    #[serde(default = "all")]
    pub app_table: String,
    #[serde(default = "all")]
    pub file_status: String,
    pub files: i64,
    pub bytes: i64,
    #[serde(default)]
    pub missing_size_files: i64,
    #[serde(default)]
    pub copies_reported: i64,
}

fn all() -> String {
    "*".to_string()
}

/// Site owner settings to save.
#[derive(Debug, Deserialize)]
pub struct SettingsSeed {
    pub copies_override: Option<i64>,
    pub price_per_gb_month: Option<f64>,
}

static STORAGE_EVENTS: OnceLock<Arc<MemEventsBackend>> = OnceLock::new();

const STORAGE_TABLE: &str = "meson_storage_daily";

/// Install the process-wide Spectra router (mem backend) the Storage page
/// reads, and make the site owner a Gauge Super User so the settings row's
/// `StorageAdmin` write rule lets them save.
pub async fn init_e2e_storage() {
    let events = Arc::new(MemEventsBackend::new());
    let router = SpectraRouter::with_defaults(
        Arc::new(MemMetricsBackend::new()) as SharedMetricsBackend,
        Arc::clone(&events) as SharedEventBackend,
    );
    SpectraRouter::set_global(Arc::new(router));
    let _ = STORAGE_EVENTS.set(events);

    let system = e2e_system_valence();
    let ensured = gauge::super_user::ensure_super_user_group(&system)
        .await
        .expect("ensure super user group");
    let principal = gauge::generated::PermissionUserPrincipal::upsert(
        "user:siteowner",
        gauge::generated::PermissionUserPrincipal::new(
            RecordId::new("user", "siteowner"),
            "siteowner".to_string(),
        )
        .expect("new user principal"),
        &system,
        valence::use_!(r"**Test:** Fixture **Permission User Principal** save so the Storage specs can act as a site owner. CI and developers running the suite only."),
    )
    .await
    .expect("upsert user principal");
    ensured
        .relate_to_member_record(
            principal.id().expect("principal id"),
            &system,
            valence::use_!(r"**Test:** Fixture member-edge relate so the Storage specs can act as a site owner. CI and developers running the suite only."),
        )
        .await
        .expect("relate member");
}

async fn seed_storage(seed: StorageSeed) -> anyhow::Result<()> {
    let events = STORAGE_EVENTS
        .get()
        .ok_or_else(|| anyhow::anyhow!("init_e2e_storage must run first"))?;
    let now = Utc::now();
    // Empty the table: drop every row older than a year from now.
    events
        .prune_before(STORAGE_TABLE, now + Duration::days(365), u32::MAX)
        .await?;
    for partition in seed.partitions {
        let measured_at = now - Duration::hours(partition.hours_old);
        let ds = measured_at.date_naive().format("%Y-%m-%d").to_string();
        for row in partition.rows {
            let fields = json!({
                "ds": ds,
                "measured_at": measured_at.to_rfc3339(),
                "run_id": partition.run_id,
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
                .append_row(STORAGE_TABLE, &fields, measured_at, None)
                .await?;
        }
    }

    let settings = seed.settings.unwrap_or(SettingsSeed {
        copies_override: None,
        price_per_gb_month: None,
    });
    let row = meson_app::generated::MesonStorageSettings::new(
        settings.copies_override,
        settings.price_per_gb_month,
        "USD".to_string(),
        None,
        now,
    )?;
    meson_app::generated::MesonStorageSettings::upsert(
        meson_app::storage::settings::SETTINGS_ID,
        row,
        &e2e_system_valence(),
        valence::use_!(r"**Test:** Fixture **Meson Storage Settings** save so the Storage specs start from known copies and price. CI and developers running the suite only."),
    )
    .await?;
    Ok(())
}

pub async fn seed_data(
    session: tower_sessions::Session,
    Json(body): Json<SeedRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let kind = E2eAuthKind::parse(&body.auth);
    write_e2e_auth_kind(&session, kind)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(storage) = body.storage {
        seed_storage(storage).await.map_err(|e| {
            log::error!("storage seed failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    let fixtures = e2e_fixtures();

    Ok(Json(serde_json::json!({
        "ok": true,
        "auth": kind.as_str(),
        "fixtures": {
            "owner_image_file_id": fixtures.owner_image_file_id,
            "owner_text_file_id": fixtures.owner_text_file_id,
            "peer_file_id": fixtures.peer_file_id,
        }
    })))
}
