//! Testable My Files service logic (session Valence + injected blob store).
//!
//! Get/preview resolve ownership via owner-scoped [`FileQueryAll`] then
//! [`find_file_in_owned_rows`]. The full-owner scan keeps foreign and missing
//! ids on the same not-found path across File implementors; scaling (get-by-id
//! on the File union) is deferred to Valence/Meson trait work.

use meson::generated::{FileFields, FileQueryAll};
use meson::{find_file_in_owned_rows, preview_kind, PreviewKind};
use valence::{RecordId, RecordPredicate, Valence};

use super::helpers::{
    io_error, not_found_error, reject_oversized_preview_metadata, MesonAppError, MAX_PREVIEW_BYTES,
};
use super::list::MyFileRow;
use super::preview::{FilePreviewPayload, PreviewMode};

fn row_from_file(row: &meson::generated::FileModel) -> Result<MyFileRow, MesonAppError> {
    let rid = row
        .id
        .clone()
        .ok_or_else(|| not_found_error("File not found"))?;
    Ok(MyFileRow {
        id: rid.to_string(),
        source_table: rid.table().to_string(),
        file_name: row.file_name().clone(),
        mime_type: row.mime_type().clone(),
        size_bytes: *row.size_bytes(),
        file_status: row.file_status().to_string(),
        uploaded_at: row.uploaded_at().to_rfc3339(),
    })
}

/// List File rows for `user_rid` (owner filter).
pub async fn list_files_for_user(
    v: &Valence,
    user_rid: RecordId,
) -> Result<Vec<MyFileRow>, MesonAppError> {
    let rows = FileQueryAll::query_used(v, valence::use_!(r"In **Meson file storage**, we **list File Query All** so the product can show or process the matching set for this workflow. Callers allowed for **Meson file storage** use the list; it is not a public dump of every field to anonymous visitors."))
        .where_uploaded_by(RecordPredicate::Equals(user_rid))
        .await
        .map_err(|e| MesonAppError::io_source("list files failed", e))?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            row_from_file(&row)
                .inspect_err(|_| {
                    tracing::warn!(outcome = "skip_row", "FileQueryAll row missing id; skipped");
                })
                .ok()
        })
        .collect())
}

/// Get one owned file metadata row; foreign ids collapse to `not_found`.
pub async fn get_file_for_user(
    v: &Valence,
    user_rid: RecordId,
    id: &str,
) -> Result<MyFileRow, MesonAppError> {
    let want = RecordId::parse(id).ok_or_else(|| not_found_error("Invalid file id"))?;
    let rows = FileQueryAll::query_used(v, valence::use_!(r"In **Meson file storage**, we **list File Query All** so the product can show or process the matching set for this workflow. Callers allowed for **Meson file storage** use the list; it is not a public dump of every field to anonymous visitors."))
        .where_uploaded_by(RecordPredicate::Equals(user_rid))
        .await
        .map_err(|e| MesonAppError::io_source("get file list failed", e))?;
    let row = find_file_in_owned_rows(rows, &want).ok_or_else(|| {
        tracing::debug!(
            target: "security",
            file_id = %want,
            outcome = "idor_collapse",
            "owned File row missing; returning not_found"
        );
        not_found_error("File not found")
    })?;
    row_from_file(&row)
}

/// Preview owned file bytes via the installed blob store ([`meson::FileBytes`]).
pub async fn preview_file_for_user(
    v: &Valence,
    user_rid: RecordId,
    id: &str,
) -> Result<FilePreviewPayload, MesonAppError> {
    let want = RecordId::parse(id).ok_or_else(|| not_found_error("Invalid file id"))?;
    let rows = FileQueryAll::query_used(v, valence::use_!(r"In **Meson file storage**, we **list File Query All** so the product can show or process the matching set for this workflow. Callers allowed for **Meson file storage** use the list; it is not a public dump of every field to anonymous visitors."))
        .where_uploaded_by(RecordPredicate::Equals(user_rid))
        .await
        .map_err(|e| MesonAppError::io_source("preview file list failed", e))?;
    let row = find_file_in_owned_rows(rows, &want).ok_or_else(|| {
        tracing::debug!(
            target: "security",
            file_id = %want,
            outcome = "idor_collapse",
            "owned File row missing; returning not_found"
        );
        not_found_error("File not found")
    })?;

    let mime = row.mime_type().clone();
    let size_bytes = *row.size_bytes();
    let kind = preview_kind(&mime);

    if kind == PreviewKind::Unsupported {
        return Ok(FilePreviewPayload {
            mode: PreviewMode::Unsupported,
            mime_type: mime,
            content: String::new(),
            size_bytes,
        });
    }

    // Bound allocation before fetching blob bytes (defense in depth after fetch too).
    reject_oversized_preview_metadata(size_bytes)?;

    let bytes = meson::FileBytes::get_file_bytes(&row)
        .await
        .map_err(|e| MesonAppError::io_source("blob get failed", e))?;

    preview_payload_from_bytes(&mime, &bytes, size_bytes)
}

/// Map mime + bytes into a preview payload (size-limit + mode).
pub(crate) fn preview_payload_from_bytes(
    mime: &str,
    bytes: &[u8],
    size_bytes: i64,
) -> Result<FilePreviewPayload, MesonAppError> {
    let kind = preview_kind(mime);
    if kind == PreviewKind::Unsupported {
        return Ok(FilePreviewPayload {
            mode: PreviewMode::Unsupported,
            mime_type: mime.to_string(),
            content: String::new(),
            size_bytes,
        });
    }
    if bytes.len() > MAX_PREVIEW_BYTES {
        return Err(io_error("File exceeds preview size limit"));
    }
    match kind {
        PreviewKind::Image => Ok(FilePreviewPayload {
            mode: PreviewMode::Image,
            mime_type: mime.to_string(),
            content: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, bytes),
            size_bytes,
        }),
        PreviewKind::Text => Ok(FilePreviewPayload {
            mode: PreviewMode::Text,
            mime_type: mime.to_string(),
            content: String::from_utf8_lossy(bytes).into_owned(),
            size_bytes,
        }),
        PreviewKind::Unsupported => Ok(FilePreviewPayload {
            mode: PreviewMode::Unsupported,
            mime_type: mime.to_string(),
            content: String::new(),
            size_bytes,
        }),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::server::helpers::{
        auth_error, forbidden_error, io_error, not_found_error, reject_oversized_preview_metadata,
    };

    #[test]
    fn error_prefixes_stable() {
        assert!(auth_error("x").to_string().contains("auth:"));
        assert!(not_found_error("x").to_string().contains("not_found:"));
        assert!(forbidden_error("x").to_string().contains("forbidden:"));
        assert!(io_error("x").to_string().contains("io:"));
    }

    #[test]
    fn preview_image_happy() {
        let p = preview_payload_from_bytes("image/png", b"PNG..", 5).expect("ok");
        assert_eq!(p.mode, PreviewMode::Image);
        assert!(!p.content.is_empty());
    }

    #[test]
    fn preview_text_happy() {
        let p = preview_payload_from_bytes("text/plain", b"hello", 5).expect("ok");
        assert_eq!(p.mode, PreviewMode::Text);
        assert_eq!(p.content, "hello");
    }

    #[test]
    fn preview_unsupported_happy() {
        let p = preview_payload_from_bytes("application/pdf", b"%PDF", 4).expect("ok");
        assert_eq!(p.mode, PreviewMode::Unsupported);
        assert!(p.content.is_empty());
    }

    #[test]
    fn preview_size_limit_sad() {
        let oversized = vec![0u8; MAX_PREVIEW_BYTES + 1];
        let size = i64::try_from(oversized.len()).expect("size fits i64");
        let err = preview_payload_from_bytes("image/png", &oversized, size).unwrap_err();
        assert!(err.to_string().contains("io:"));
        assert!(err.to_string().contains("exceeds preview size limit"));
    }

    #[test]
    fn preview_metadata_size_guard_rejects_before_fetch() {
        let max = i64::try_from(MAX_PREVIEW_BYTES).expect("max fits i64");
        let err = reject_oversized_preview_metadata(max + 1).unwrap_err();
        assert!(err.to_string().contains("exceeds preview size limit"));
        assert!(reject_oversized_preview_metadata(max).is_ok());
    }
}
