//! File status badge (Orbital Badge only — no layout CSS).

use leptos::prelude::*;
use uf_product::primitives::Badge;

use crate::status::badge_for_file_status;

/// Map File `file_status` to Pending / Available / Quarantined badge chrome.
#[component]
pub fn FileStatusBadge(
    /// Raw status from the File row (`available`, `pending_virus_scan`, …).
    #[prop(into)]
    status: String,
) -> impl IntoView {
    let (label, appearance, color) = badge_for_file_status(&status);

    view! {
        <span data-testid="meson-file-status-badge" data-status=status>
            <Badge appearance=appearance color=color>{label}</Badge>
        </span>
    }
}
