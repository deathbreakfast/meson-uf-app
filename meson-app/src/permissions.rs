//! Permission manifest for the Meson My Files app.

use uf_product_macros::UfPermissionManifest;

/// Meson app permission domain for the platform catalog.
///
/// My Files list, get and preview server fns and the `/meson` routes check
/// [`MesonPermission::FilesRead`] through the host's `PermissionBackend`. The
/// check fails closed, so each embedding host grants it to its members group.
#[allow(clippy::expl_impl_clone_on_copy)]
#[derive(UfPermissionManifest)]
#[permission_manifest(
    domain_key = "meson",
    domain_name = "Meson",
    domain_description = "My Files browser across File-trait uploads"
)]
pub enum MesonPermission {
    /// Open My Files and read the signed-in user's own uploads.
    #[permission(description = "List and preview own Meson uploads (Files read)")]
    FilesRead,
    /// Site owners: storage use and estimated cost for everyone, and the cost settings.
    #[permission(
        description = "See how much storage everyone on the site uses and what it costs, and change the cost settings"
    )]
    StorageAdmin,
}

/// Permission name checked by the My Files server fns and routes.
pub const FILES_READ: &str = "FilesRead";
