//! Delete and download one of the session user's files.
//!
//! Both load the row owner-scoped (a foreign or missing id is `not_found`) and
//! run under the session actor. Delete goes through [`meson::delete_file`], so
//! the app that stored the file can refuse and the table's `delete:` policy
//! decides; download refuses files that are still being scanned or were
//! flagged.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// What happened to a delete.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeleteResult {
    /// The row is gone. `blob_removed` is false when its bytes were left in
    /// storage (logged server-side).
    Deleted {
        /// Whether the stored bytes were removed too.
        blob_removed: bool,
    },
    /// The app that stored the file refused; nothing changed.
    Refused {
        /// The app's sentence for the person who asked.
        reason: String,
    },
}

/// A file's bytes for the browser to save.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DownloadPayload {
    /// Name to save it under.
    pub file_name: String,
    /// Mime type from the File row.
    pub mime_type: String,
    /// The bytes, base64.
    pub content_base64: String,
}

/// Delete one of the session user's files with its bytes.
#[uf_product_macros::server(permission = "FilesRead")]
pub async fn delete_my_file(id: String) -> Result<DeleteResult, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use super::helpers::{session_user_record_id, session_valence_from_ctx, MesonAppError};

        let ctx = higgs::Higgs::from_request().await?;
        let v = session_valence_from_ctx(&ctx).map_err(MesonAppError::into_server_fn)?;
        let user_rid = session_user_record_id(&ctx).map_err(MesonAppError::into_server_fn)?;
        ssr::delete_file_for_user(&v, &user_rid, &id)
            .await
            .map_err(MesonAppError::into_server_fn)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("ssr required"))
    }
}

/// The bytes of one of the session user's files, for Download.
#[uf_product_macros::server(permission = "FilesRead")]
pub async fn download_my_file(id: String) -> Result<DownloadPayload, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use super::helpers::{session_user_record_id, session_valence_from_ctx, MesonAppError};

        let ctx = higgs::Higgs::from_request().await?;
        let v = session_valence_from_ctx(&ctx).map_err(MesonAppError::into_server_fn)?;
        let user_rid = session_user_record_id(&ctx).map_err(MesonAppError::into_server_fn)?;
        ssr::download_file_for_user(&v, &user_rid, &id)
            .await
            .map_err(MesonAppError::into_server_fn)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("ssr required"))
    }
}

#[cfg(feature = "ssr")]
mod ssr {
    use meson::generated::{FileFields, FileFileStatus};
    use meson::{delete_file, get_by_id, DeleteError, DeleteOutcome, FileBytes};
    use valence::{RecordId, Valence};

    use super::{DeleteResult, DownloadPayload};
    use crate::server::helpers::{not_found_error, MesonAppError};

    fn parse_id(id: &str) -> Result<RecordId, MesonAppError> {
        RecordId::parse(id).ok_or_else(|| not_found_error("Invalid file id"))
    }

    /// Map [`meson::delete_file`]'s answer onto the wire result.
    pub(super) fn delete_result(
        outcome: Result<DeleteOutcome, DeleteError>,
    ) -> Result<DeleteResult, MesonAppError> {
        match outcome {
            Ok(DeleteOutcome::Deleted { blob_removed }) => {
                Ok(DeleteResult::Deleted { blob_removed })
            }
            Ok(DeleteOutcome::Refused(refusal)) => Ok(DeleteResult::Refused {
                reason: refusal.reason,
            }),
            Ok(DeleteOutcome::NotFound) => Err(not_found_error("File not found")),
            Err(e) => Err(MesonAppError::io_source("delete failed", e)),
        }
    }

    /// Why a file in `status` can't be downloaded, if it can't.
    pub(super) fn download_refusal(status: &FileFileStatus) -> Option<&'static str> {
        match status {
            FileFileStatus::Available => None,
            FileFileStatus::PendingVirusScan | FileFileStatus::VirusScanComplete => {
                Some("The file is still being checked for viruses")
            }
            FileFileStatus::Quarantined => Some("The virus scan flagged this file"),
        }
    }

    pub(super) async fn delete_file_for_user(
        v: &Valence,
        owner: &RecordId,
        id: &str,
    ) -> Result<DeleteResult, MesonAppError> {
        let want = parse_id(id)?;
        delete_result(delete_file(v, owner, &want).await)
    }

    pub(super) async fn download_file_for_user(
        v: &Valence,
        owner: &RecordId,
        id: &str,
    ) -> Result<DownloadPayload, MesonAppError> {
        let want = parse_id(id)?;
        let row = get_by_id(v, valence::use_!(r"When you press **Download** on a file in **My Files**, we load **that one file's details** (name, type and scan status) to check it is **yours** and has passed its virus check, then fetch its **contents** from storage and **hand them to your browser** to save under its name."), owner, &want)
            .await
            .map_err(|e| MesonAppError::io_source("download file get failed", e))?
            .ok_or_else(|| not_found_error("File not found"))?;
        if let Some(reason) = download_refusal(row.file_status()) {
            return Err(MesonAppError::invalid(reason));
        }
        let bytes = row
            .get_file_bytes()
            .await
            .map_err(|e| MesonAppError::io_source("blob get failed", e))?;
        Ok(DownloadPayload {
            file_name: row.file_name().clone(),
            mime_type: row.mime_type().clone(),
            content_base64: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                bytes,
            ),
        })
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use meson::generated::FileFileStatus;
    use meson::{DeleteOutcome, DeleteRefusal};

    use super::ssr::{delete_result, download_refusal};
    use super::DeleteResult;

    #[test]
    fn delete_outcomes_map_to_results() {
        assert_eq!(
            delete_result(Ok(DeleteOutcome::Deleted { blob_removed: true })).expect("ok"),
            DeleteResult::Deleted { blob_removed: true }
        );
        assert_eq!(
            delete_result(Ok(DeleteOutcome::Refused(DeleteRefusal::new(
                "It's attached to an open expense."
            ))))
            .expect("ok"),
            DeleteResult::Refused {
                reason: "It's attached to an open expense.".into()
            }
        );
        let err = delete_result(Ok(DeleteOutcome::NotFound)).expect_err("not found");
        assert!(err.to_string().starts_with("not_found:"), "{err}");
    }

    #[test]
    fn download_refused_until_the_scan_passes() {
        assert_eq!(download_refusal(&FileFileStatus::Available), None);
        assert!(download_refusal(&FileFileStatus::PendingVirusScan).is_some());
        assert!(download_refusal(&FileFileStatus::VirusScanComplete).is_some());
        assert!(download_refusal(&FileFileStatus::Quarantined).is_some());
    }
}
