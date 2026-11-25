//! Shared Meson My Files UI pieces.

mod error_bar;
mod file_status_badge;
mod keeps_dialog;

pub use error_bar::{classify, ErrorBar, ErrorKind, FilesReadDenied, NotInYourFiles};
pub use file_status_badge::FileStatusBadge;
pub use keeps_dialog::KeepsDialog;
