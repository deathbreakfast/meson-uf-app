//! The daily storage run as the Everyone tab shows it, and the hook a host
//! installs when it runs Chronon.
//!
//! Meson-app doesn't depend on Chronon. A host that schedules the
//! `meson.storage-daily` job installs a [`StorageRunHook`] at boot with
//! [`install_storage_run_hook`]; the run card then shows the schedule, Run now
//! and Open in Chronon. Without a hook the card shows what the snapshot alone
//! says.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// Chronon job name of the daily storage run.
pub const JOB_NAME: &str = "meson.storage-daily";

/// What the run card shows when a host installed a [`StorageRunHook`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageRunInfo {
    /// When the job runs, in words (`Every day at 3:00 AM`).
    pub schedule: String,
    /// Where the job lives in Chronon's UI.
    pub chronon_href: String,
}

/// Host-side link to the Chronon job that writes `meson_storage_daily`.
#[cfg(feature = "ssr")]
#[async_trait::async_trait]
pub trait StorageRunHook: Send + Sync {
    /// When the job runs, in words.
    fn schedule(&self) -> String;
    /// Link to the job in Chronon's UI.
    fn chronon_href(&self) -> String;
    /// Start a run now.
    ///
    /// # Errors
    ///
    /// A sentence saying why the run couldn't start.
    async fn run_now(&self) -> Result<(), String>;
}

#[cfg(feature = "ssr")]
static HOOK: std::sync::OnceLock<std::sync::Arc<dyn StorageRunHook>> = std::sync::OnceLock::new();

/// Install the host's run hook once at boot.
///
/// # Errors
///
/// Returns the hook back when one is already installed.
#[cfg(feature = "ssr")]
pub fn install_storage_run_hook(
    hook: std::sync::Arc<dyn StorageRunHook>,
) -> Result<(), std::sync::Arc<dyn StorageRunHook>> {
    HOOK.set(hook)
}

#[cfg(feature = "ssr")]
async fn require_storage_admin() -> Result<(), ServerFnError> {
    if uf_product::permissions::has_permission(
        crate::permissions::MesonPermission::StorageAdmin.as_str(),
    )
    .await
    {
        Ok(())
    } else {
        Err(
            crate::server::MesonAppError::forbidden("StorageAdmin permission required")
                .into_server_fn(),
        )
    }
}

/// The schedule and Chronon link, or `None` when this host runs no Chronon.
/// StorageAdmin only.
#[uf_product_macros::server]
pub async fn get_storage_run() -> Result<Option<StorageRunInfo>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        require_storage_admin().await?;
        Ok(HOOK.get().map(|hook| StorageRunInfo {
            schedule: hook.schedule(),
            chronon_href: hook.chronon_href(),
        }))
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::new("ssr required"))
    }
}

/// Start the daily storage run now. StorageAdmin only.
#[uf_product_macros::server]
pub async fn run_storage_now() -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        require_storage_admin().await?;
        let Some(hook) = HOOK.get() else {
            return Err(
                crate::server::MesonAppError::invalid("this site doesn't run Chronon")
                    .into_server_fn(),
            );
        };
        hook.run_now().await.map_err(|e| {
            crate::server::MesonAppError::io_msg(format!("storage run didn't start: {e}"))
                .into_server_fn()
        })
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::new("ssr required"))
    }
}
