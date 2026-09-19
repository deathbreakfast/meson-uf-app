//! Orbital shell layout for Meson (`MesonLayout`).

use lepton_shell::AppBarUserMenu;
use leptos::prelude::*;
use leptos_router::components::Outlet;
use uf_product::components::{
    Navigation, NavigationBody, NavigationConfig, NavigationLink, NavigationMaterial,
};
use uf_product::routes::RequireAuthenticated;

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
                    </NavigationBody>
                </Navigation>
            </ShellLeftNav>
            <RequireAuthenticated>
                <Outlet />
            </RequireAuthenticated>
        </UnifiedFieldShellLayout>
        </div>
    }
}
