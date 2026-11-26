//! Preview / download bytes for an owned File row.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// Largest file the page previews (matches lepton's upload cap). Bigger files
/// get [`PreviewMode::TooBig`] and a Download button.
pub const PREVIEW_LIMIT_BYTES: usize = 5 * 1024 * 1024;

/// How the client should render preview bytes.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PreviewMode {
    /// Image bytes as base64.
    Image,
    /// UTF-8 text.
    Text,
    /// No in-app preview; metadata only (bytes omitted).
    Unsupported,
    /// Over [`PREVIEW_LIMIT_BYTES`]; bytes never fetched.
    TooBig,
}

/// Preview payload returned to the UI.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FilePreviewPayload {
    /// Render mode.
    pub mode: PreviewMode,
    /// Mime type from the File row.
    pub mime_type: String,
    /// Base64 (image) or UTF-8 (text); empty when unsupported or too big.
    pub content: String,
    /// Byte length of the stored object.
    pub size_bytes: i64,
}

#[uf_product_macros::server(permission = "FilesRead")]
pub async fn get_my_file_preview(id: String) -> Result<FilePreviewPayload, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use super::helpers::{session_user_record_id, session_valence_from_ctx, MesonAppError};
        use super::service::preview_file_for_user;

        let ctx = higgs::Higgs::from_request().await?;
        let v = session_valence_from_ctx(&ctx).map_err(MesonAppError::into_server_fn)?;
        let user_rid = session_user_record_id(&ctx).map_err(MesonAppError::into_server_fn)?;
        preview_file_for_user(&v, user_rid, &id)
            .await
            .map_err(MesonAppError::into_server_fn)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("ssr required"))
    }
}
