//! Pages module.

mod file_detail;
mod my_files;
pub mod storage;

pub use file_detail::{FileDetailPage, PreviewPane};
pub use my_files::{MyFilesPage, MyFilesTable};
pub use storage::StoragePage;
