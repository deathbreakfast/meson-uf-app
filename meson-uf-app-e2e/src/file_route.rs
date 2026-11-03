//! Lab-only `GET /api/files/{id}`: the route `MesonImg` points `<img src>` at.
//!
//! Serves bytes only for an Available File owned by the session user. Missing,
//! foreign and non-available ids all return 404 so ids cannot be probed.

use axum::extract::Path;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use meson::generated::{FileFields, FileQueryAll};
use meson::{find_file_in_owned_rows, FileBytes};
use valence::{RecordId, RecordPredicate};

use crate::e2e_valence::e2e_system_valence;
use crate::gate_demos::e2e_session_user_id;

pub async fn get_file(session: tower_sessions::Session, Path(id): Path<String>) -> Response {
    let Some(user) = e2e_session_user_id(&session).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let (Some(owner), Some(want)) = (
        RecordId::parse(user),
        RecordId::parse(&id).or_else(|| RecordId::parse(&format!("e2e_meson_file:{id}"))),
    ) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    // System Valence is the lab host's own fixture store; ownership is enforced
    // by the explicit `uploaded_by` filter plus `find_file_in_owned_rows`.
    let rows = FileQueryAll::query(
        &e2e_system_valence(),
        valence::use_!(r"**Test:** Lab host `/api/files/{id}` lists the session user's File rows so Playwright can load seeded image bytes. CI and developers running the suite only."),
    )
    .where_uploaded_by(RecordPredicate::Equals(owner))
    .await;
    let Ok(rows) = rows else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    let Some(row) = find_file_in_owned_rows(rows, &want) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match row.get_file_bytes().await {
        Ok(bytes) => ([(header::CONTENT_TYPE, row.mime_type().clone())], bytes).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
