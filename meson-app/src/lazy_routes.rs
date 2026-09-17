//! Lazy-loaded route views for WASM code-splitting.

#![allow(clippy::used_underscore_binding)]

use leptos::prelude::*;
use leptos_router::{lazy_route, LazyRoute};

use crate::{FileDetailPage, MesonLayout, MyFilesPage};

/// Eager layout shell for `/meson/*`.
#[component]
pub fn MesonLayoutRouteView() -> impl IntoView {
    view! { <MesonLayout /> }
}

/// Lazy `/meson` My Files index.
#[derive(Clone, Copy, Debug, Default)]
pub struct MyFilesIndexRoute;

#[lazy_route]
impl LazyRoute for MyFilesIndexRoute {
    fn data() -> Self {
        Self
    }

    fn view(_this: Self) -> AnyView {
        view! { <MyFilesPage /> }.into_any()
    }
}

/// Lazy `/meson/files/:id` detail/preview.
#[derive(Clone, Copy, Debug, Default)]
pub struct FileDetailRoute;

#[lazy_route]
impl LazyRoute for FileDetailRoute {
    fn data() -> Self {
        Self
    }

    fn view(_this: Self) -> AnyView {
        view! { <FileDetailPage /> }.into_any()
    }
}
