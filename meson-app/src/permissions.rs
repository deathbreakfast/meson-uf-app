//! Permission manifest for the Meson My Files app.

use uf_product_macros::UfPermissionManifest;

/// Meson app permission domain for the platform catalog.
///
/// My Files list/preview is gated by authenticated session (not Gauge). This
/// manifest still registers the domain for host inventory / future grants.
#[allow(clippy::expl_impl_clone_on_copy)]
#[derive(UfPermissionManifest)]
#[permission_manifest(
    domain_key = "meson",
    domain_name = "Meson",
    domain_description = "My Files browser across File-trait uploads"
)]
pub enum MesonPermission {
    /// Reserved for future shared-file browse grants.
    #[permission(description = "List and preview own Meson uploads")]
    FilesRead,
    /// Site owners: storage use and estimated cost for everyone, and the cost settings.
    #[permission(
        description = "See how much storage everyone on the site uses and what it costs, and change the cost settings"
    )]
    StorageAdmin,
}
