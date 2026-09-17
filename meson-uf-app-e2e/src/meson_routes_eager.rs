//! Eager `/meson` routes for the Playwright host.
//!
//! Production `meson_app::MesonRoutes` wraps leaf pages in `Lazy` for
//! wasm-split. Nested `Lazy` under `ParentRoute` still panics on hydrate in
//! this Leptos pin, so the lab host mounts the same page components without
//! `Lazy`.

use leptos::prelude::*;
use leptos_router::{
    components::{ParentRoute, Route},
    path,
};
use meson_app::{FileDetailPage, MesonLayout, MyFilesPage};

/// Same paths as [`meson_app::MesonRoutes`], without Lazy route views.
#[component(transparent)]
pub fn MesonRoutesEager() -> impl leptos_router::MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("meson") view=MesonLayout>
            <Route path=path!("") view=MyFilesPage />
            <Route path=path!("files/:id") view=FileDetailPage />
        </ParentRoute>
    }
    .into_inner()
}
