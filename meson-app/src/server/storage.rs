//! Storage page server fns: the member's own usage, the site view for site
//! owners, and the cost settings.
//!
//! Each fn runs as the session user. The site view and saving settings need
//! the Gauge permission [`MesonPermission::StorageAdmin`](crate::permissions::MesonPermission::StorageAdmin),
//! checked through the host's uf-product permission backend (fail closed when
//! none is installed). Snapshots come from the process-wide Spectra router the
//! host installs at boot; without one, sizes read as unavailable.

use leptos::prelude::*;

use crate::storage::settings::StorageSettings;
use crate::storage::view::{SiteStorageView, StorageView};

#[cfg(feature = "ssr")]
async fn is_storage_admin() -> bool {
    uf_product::permissions::has_permission(
        crate::permissions::MesonPermission::StorageAdmin.as_str(),
    )
    .await
}

/// The signed-in member's storage: their totals, per-app split, cost estimate
/// and 30-day history.
#[uf_product_macros::server]
pub async fn get_my_storage() -> Result<StorageView, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use super::helpers::{session_user_record_id, session_valence_from_ctx, MesonAppError};

        let ctx = higgs::Higgs::from_request().await?;
        let v = session_valence_from_ctx(&ctx).map_err(MesonAppError::into_server_fn)?;
        let user_rid = session_user_record_id(&ctx).map_err(MesonAppError::into_server_fn)?;
        let router = spectra::SpectraRouter::try_global();
        super::storage_service::my_storage(router.as_deref(), &v, &user_rid, chrono::Utc::now())
            .await
            .map_err(MesonAppError::into_server_fn)
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::new("ssr required"))
    }
}

/// Site-wide storage: totals, per-app and per-person rows, and the last run.
/// StorageAdmin only.
#[uf_product_macros::server]
pub async fn get_site_storage() -> Result<SiteStorageView, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use super::helpers::{session_valence_from_ctx, MesonAppError};

        let ctx = higgs::Higgs::from_request().await?;
        let v = session_valence_from_ctx(&ctx).map_err(MesonAppError::into_server_fn)?;
        let router = spectra::SpectraRouter::try_global();
        super::storage_service::site_storage(
            router.as_deref(),
            &v,
            is_storage_admin().await,
            chrono::Utc::now(),
        )
        .await
        .map_err(MesonAppError::into_server_fn)
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::new("ssr required"))
    }
}

/// The site's cost settings. Any signed-in member.
#[uf_product_macros::server]
pub async fn get_storage_settings() -> Result<StorageSettings, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use super::helpers::{session_valence_from_ctx, MesonAppError};

        let ctx = higgs::Higgs::from_request().await?;
        let v = session_valence_from_ctx(&ctx).map_err(MesonAppError::into_server_fn)?;
        super::storage_service::storage_settings(&v)
            .await
            .map_err(MesonAppError::into_server_fn)
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::new("ssr required"))
    }
}

/// Save the copies override (1 to 10, or `None` to follow storage) and the
/// price per GB-month (≥ 0, or `None` for sizes only). StorageAdmin only.
#[uf_product_macros::server]
pub async fn save_storage_settings(
    copies_override: Option<i64>,
    price_per_gb_month: Option<f64>,
) -> Result<StorageSettings, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use super::helpers::{session_user_record_id, session_valence_from_ctx, MesonAppError};

        let ctx = higgs::Higgs::from_request().await?;
        let v = session_valence_from_ctx(&ctx).map_err(MesonAppError::into_server_fn)?;
        let user_rid = session_user_record_id(&ctx).map_err(MesonAppError::into_server_fn)?;
        super::storage_service::save_settings(
            &v,
            is_storage_admin().await,
            user_rid,
            copies_override,
            price_per_gb_month,
            chrono::Utc::now(),
        )
        .await
        .map_err(MesonAppError::into_server_fn)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (copies_override, price_per_gb_month);
        Err(ServerFnError::new("ssr required"))
    }
}
