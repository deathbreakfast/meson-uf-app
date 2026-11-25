//! SSR server functions for My Files.

mod get;
mod helpers;
mod list;
mod preview;
#[cfg(feature = "ssr")]
mod service;
mod storage;
#[cfg(feature = "ssr")]
mod storage_service;

pub use get::get_my_file;
pub use list::{list_my_files, subscribe_list_my_files, MyFileRow};
pub use preview::{get_my_file_preview, FilePreviewPayload, PreviewMode};
pub use storage::{get_my_storage, get_site_storage, get_storage_settings, save_storage_settings};

#[cfg(feature = "ssr")]
pub use helpers::MesonAppError;
#[cfg(feature = "ssr")]
pub use service::{get_file_for_user, list_files_for_user, preview_file_for_user};
#[cfg(feature = "ssr")]
pub use storage_service::{my_storage, save_settings, site_storage, storage_settings};
