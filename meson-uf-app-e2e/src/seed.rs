//! Harness-only seed endpoint for Playwright.

use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::e2e_valence::{e2e_fixtures, seed_fresh_files};
use crate::gate_demos::{write_e2e_auth_kind, E2eAuthKind};

#[derive(Debug, Deserialize)]
pub struct SeedRequest {
    /// `anonymous` | `owner` | `peer` | `unverified` | `broken_session` | `no_files_read`
    #[serde(default = "default_auth")]
    pub auth: String,
    /// New deletable rows to create for this spec: `plain` | `quarantined` | `keep`.
    #[serde(default)]
    pub fresh: Vec<String>,
}

fn default_auth() -> String {
    E2eAuthKind::Anonymous.as_str().to_string()
}

pub async fn seed_data(
    session: tower_sessions::Session,
    Json(body): Json<SeedRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let kind = E2eAuthKind::parse(&body.auth);
    write_e2e_auth_kind(&session, kind)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let fixtures = e2e_fixtures();
    let fresh: serde_json::Map<String, serde_json::Value> = seed_fresh_files(&body.fresh)
        .await
        .into_iter()
        .map(|(kind, id)| (kind, serde_json::Value::String(id)))
        .collect();

    Ok(Json(serde_json::json!({
        "ok": true,
        "auth": kind.as_str(),
        "fixtures": {
            "owner_image_file_id": fixtures.owner_image_file_id,
            "owner_text_file_id": fixtures.owner_text_file_id,
            "peer_file_id": fixtures.peer_file_id,
            "owner_scanning_file_id": fixtures.owner_scanning_file_id,
            "owner_slow_scan_file_id": fixtures.owner_slow_scan_file_id,
            "owner_quarantined_file_id": fixtures.owner_quarantined_file_id,
            "owner_big_file_id": fixtures.owner_big_file_id,
            "owner_pdf_file_id": fixtures.owner_pdf_file_id,
        },
        "fresh": fresh,
    })))
}
