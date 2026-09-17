//! Fetch a single My Files row by id (ownership checked).

use leptos::prelude::*;

use super::list::MyFileRow;

/// Read one owned File metadata row.
///
/// Pair with [`super::list::subscribe_list_my_files`] so detail refetches when
/// Photon publishes `meson.file.updated` (same topic / WS as the list).
#[uf_product_macros::server]
pub async fn get_my_file(id: String) -> Result<MyFileRow, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use super::helpers::{session_user_record_id, session_valence_from_ctx, MesonAppError};
        use super::service::get_file_for_user;

        let ctx = higgs::Higgs::from_request().await?;
        let v = session_valence_from_ctx(&ctx).map_err(MesonAppError::into_server_fn)?;
        let user_rid = session_user_record_id(&ctx).map_err(MesonAppError::into_server_fn)?;
        get_file_for_user(&v, user_rid, &id)
            .await
            .map_err(MesonAppError::into_server_fn)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("ssr required"))
    }
}
