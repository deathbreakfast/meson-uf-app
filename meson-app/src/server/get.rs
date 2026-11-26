//! Fetch a single My Files row by id (ownership checked), plus what the file
//! page shows beside it: the app that stored it and its storage footprint.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use super::list::MyFileRow;
use crate::storage::settings::StorageSettings;
use crate::storage::view::{cost_per_month, CopiesSource};

/// One file as its page shows it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MyFileDetail {
    /// The File row.
    pub file: MyFileRow,
    /// Name of the app that stored it ("Receipt scanner"), or its table name
    /// when the app registered none.
    pub app_name: String,
    /// Where "Open in <app>" goes. `None` until apps can register one, which
    /// leaves the button disabled.
    pub open_url: Option<String>,
    /// Copies and cost; `None` when the site's storage settings can't be read.
    pub storage: Option<FileStorage>,
}

/// What one file takes up in storage.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FileStorage {
    /// Copies kept (the site owner's override, else 1).
    pub copies: i64,
    /// Size times copies.
    pub stored_bytes: i64,
    /// Estimated cost each month, when a price is set.
    pub cost_per_month: Option<f64>,
    /// Currency of the cost.
    pub currency: String,
}

/// Storage footprint of a file of `size_bytes` under the site's settings.
#[must_use]
pub fn file_storage(size_bytes: i64, settings: &StorageSettings) -> FileStorage {
    let copies = CopiesSource::resolve(settings.copies_override, None).count();
    FileStorage {
        copies,
        stored_bytes: size_bytes.saturating_mul(copies),
        cost_per_month: cost_per_month(size_bytes, copies, settings.price_per_gb_month),
        currency: settings.currency.clone(),
    }
}

/// Read one owned File metadata row.
///
/// Pair with [`super::list::subscribe_list_my_files`] so detail refetches when
/// Photon publishes `meson.file.updated` (same topic / WS as the list).
#[uf_product_macros::server(permission = "FilesRead")]
pub async fn get_my_file(id: String) -> Result<MyFileDetail, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use super::helpers::{session_user_record_id, session_valence_from_ctx, MesonAppError};
        use super::service::get_file_for_user;
        use super::storage_service::storage_settings;

        let ctx = higgs::Higgs::from_request().await?;
        let v = session_valence_from_ctx(&ctx).map_err(MesonAppError::into_server_fn)?;
        let user_rid = session_user_record_id(&ctx).map_err(MesonAppError::into_server_fn)?;
        let file = get_file_for_user(&v, user_rid, &id)
            .await
            .map_err(MesonAppError::into_server_fn)?;
        let storage = match storage_settings(&v).await {
            Ok(settings) => Some(file_storage(file.size_bytes, &settings)),
            Err(e) => {
                tracing::warn!(error = %e, outcome = "storage_row_omitted", "storage settings read failed");
                None
            }
        };
        Ok(MyFileDetail {
            app_name: meson::app_name_or_table(&file.source_table).to_string(),
            open_url: None,
            storage,
            file,
        })
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("ssr required"))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn storage_multiplies_by_the_copies_override() {
        let settings = StorageSettings {
            copies_override: Some(3),
            price_per_gb_month: Some(0.02),
            ..StorageSettings::default()
        };
        let s = file_storage(1_000_000_000, &settings);
        assert_eq!(s.copies, 3);
        assert_eq!(s.stored_bytes, 3_000_000_000);
        let cost = s.cost_per_month.expect("price set");
        assert!((cost - 0.06).abs() < 1e-9, "{cost}");
        assert_eq!(s.currency, "USD");
    }

    #[test]
    fn storage_without_settings_is_one_copy_and_no_cost() {
        let s = file_storage(248_311, &StorageSettings::default());
        assert_eq!(s.copies, 1);
        assert_eq!(s.stored_bytes, 248_311);
        assert_eq!(s.cost_per_month, None);
    }
}
