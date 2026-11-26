//! Orbital shell layout for Meson (`MesonLayout`).

use lepton_shell::AppBarUserMenu;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::Outlet;
use uf_product::components::{
    Navigation, NavigationBody, NavigationConfig, NavigationLink, NavigationMaterial,
};
use uf_product::routes::RequireAuthenticated;

use crate::components::FilesReadDenied;
use crate::permissions::FILES_READ;
use crate::shell::{
    ShellAppBar, ShellAuthMenu, ShellLeftNav, UnifiedFieldAppBar, UnifiedFieldShellLayout,
};
use crate::AppMetadata;

/// Shell layout: app bar + left nav wrapping the routed page [`Outlet`].
///
/// Auth gating lives inside the shell so the app bar stays visible when sign-in
/// is required.
#[component]
pub fn MesonLayout() -> impl IntoView {
    let app_name = AppMetadata::name().to_string();
    let selected_value = RwSignal::new(None::<String>);
    let open_categories = RwSignal::new(Vec::<String>::new());

    view! {
        <div data-testid="meson-app-root">
        <UnifiedFieldShellLayout>
            <ShellAppBar slot>
                <UnifiedFieldAppBar
                    app_name=app_name
                    app_id=AppMetadata::id()
                    homepage_url="/".to_string()
                >
                    <ShellAuthMenu slot:auth_menu>
                        <AppBarUserMenu />
                    </ShellAuthMenu>
                </UnifiedFieldAppBar>
            </ShellAppBar>
            <ShellLeftNav slot>
                <Navigation config=NavigationConfig::new().with_selected_value(selected_value).with_open_categories(open_categories)>
                    <NavigationMaterial slot />
                    <NavigationBody slot>
                        <div id="meson-nav-my-files">
                            <NavigationLink path="/meson" value="/meson" icon=icondata::AiFolderOutlined exact=true test_id="nav-meson-files">"My Files"</NavigationLink>
                        </div>
                        <div id="meson-nav-storage">
                            <NavigationLink path="/meson/storage" value="/meson/storage" icon=icondata::AiCloudOutlined test_id="nav-meson-storage">"Storage"</NavigationLink>
                        </div>
                    </NavigationBody>
                </Navigation>
            </ShellLeftNav>
            <RequireAuthenticated>
                <FilesReadGate>
                    <Outlet />
                </FilesReadGate>
            </RequireAuthenticated>
        </UnifiedFieldShellLayout>
        </div>
    }
}

/// Renders `children` only for viewers holding `FilesRead`.
///
/// The check runs in the browser through uf-product's `check_permission_by_name`
/// server fn, a real request that carries the host's `PermissionBackend`
/// context. During the server render the backend isn't always in context, so
/// the gate would deny every viewer there; it renders nothing until the check
/// returns. Fails closed: only an explicit allow opens the routes; a denial, a
/// failed check or a host with no permission backend shows the denied state.
/// The server fns check the same permission, so this gate is for the page, not
/// the data.
#[component]
fn FilesReadGate(children: ChildrenFn) -> impl IntoView {
    let allowed = RwSignal::new(None::<bool>);
    // Effects don't run during the server render, so this is browser-only.
    Effect::new(move |_| {
        spawn_local(async move {
            let ok = matches!(
                uf_product::permissions::check_permission_by_name(FILES_READ.to_string()).await,
                Ok(true)
            );
            allowed.set(Some(ok));
        });
    });
    view! {
        {move || match allowed.get() {
            None => ().into_any(),
            Some(true) => children().into_any(),
            Some(false) => view! { <FilesReadDenied /> }.into_any(),
        }}
    }
}
