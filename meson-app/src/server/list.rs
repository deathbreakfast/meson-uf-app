//! List My Files via `FileQueryAll`.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// One row in the My Files table (trait projection + source table).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MyFileRow {
    /// Full `table:id` record id.
    pub id: String,
    /// Source Valence table (e.g. `profile_photo`).
    pub source_table: String,
    /// Original upload name.
    pub file_name: String,
    /// Mime type.
    pub mime_type: String,
    /// Size in bytes.
    pub size_bytes: i64,
    /// File status enum as string.
    pub file_status: String,
    /// Uploaded-at ISO timestamp.
    pub uploaded_at: String,
}

/// Photon live list — refetches when `meson.file.updated` fires for the session user.
///
/// Detail pages reuse [`subscribe_list_my_files`] (parameterized [`super::get_my_file`]
/// cannot carry `#[synced]` hydrate hooks). Hosts must mount Photon WS at
/// `/ws/meson-files` (inventory from this attribute).
#[cfg_attr(
    any(feature = "ssr", feature = "hydrate"),
    photon_leptos::synced(
        topic = "meson.file.updated",
        ws = "/ws/meson-files",
        strategy = "refetch",
        auth = "user"
    )
)]
#[uf_product_macros::server]
pub async fn list_my_files() -> Result<Vec<MyFileRow>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use super::helpers::{session_user_record_id, session_valence_from_ctx, MesonAppError};
        use super::service::list_files_for_user;

        let ctx = higgs::Higgs::from_request().await?;
        let v = session_valence_from_ctx(&ctx).map_err(MesonAppError::into_server_fn)?;
        let user_rid = session_user_record_id(&ctx).map_err(MesonAppError::into_server_fn)?;
        list_files_for_user(&v, user_rid)
            .await
            .map_err(MesonAppError::into_server_fn)
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::new("ssr required"))
    }
}

/// Fallback when neither `ssr` nor `hydrate` is enabled (no WebSocket).
#[cfg(not(any(feature = "ssr", feature = "hydrate")))]
pub fn subscribe_list_my_files(_on_event: impl Fn() + Send + Sync + 'static) -> RwSignal<u64> {
    RwSignal::new(0u64)
}
