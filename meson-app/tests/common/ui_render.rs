//! Shared SSR HTML render helper for UI e2e tests.

#![allow(dead_code)]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use leptos::prelude::*;
use leptos_router::components::Router;
use leptos_router::location::RequestUrl;

/// Build the view **inside** an Owner so sandboxed arenas are active.
///
/// Wraps `build()` in a `<Router>` (with a `RequestUrl` context, same as a
/// real SSR request) since `leptos_router`'s `<A>` component panics outside
/// one — any real page render always has this context in production.
pub fn render_html(build: impl FnOnce() -> AnyView + Send + 'static) -> String {
    Owner::new().with(|| {
        provide_context(RequestUrl::new("/"));
        view! { <Router>{build()}</Router> }.to_html()
    })
}
