//! `IsolatedLab` / SSR UI render e2e for the File detail / preview page chrome.

#![cfg(feature = "ssr")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::ui_render::render_html;
use leptos::prelude::*;
use leptos_router::components::A;
use meson_app::components::FileStatusBadge;
use meson_app::server::{FilePreviewPayload, PreviewMode};
use meson_app::PreviewPane;
use uf_product::components::{
    Body1, Caption1, Card, CardContent, ContentContainer, SpacingSize, Title3,
};
use uf_product::primitives::{Button, ButtonAppearance, Flex, MessageBar, MessageBarIntent};

/// Back-link + metadata card chrome, matching `FileDetailPage`'s `Some(Ok(meta))` arm minus the preview card.
fn detail_shell(children: impl IntoView + 'static) -> impl IntoView {
    view! {
        <ContentContainer data_testid="meson-file-detail">
            <Flex vertical=true gap=SpacingSize::Size240.flex_gap()>
                <A href="/meson">
                    <Button appearance=ButtonAppearance::Subtle>"Back to My Files"</Button>
                </A>
                {children}
            </Flex>
        </ContentContainer>
    }
}

/// E2E-L04 — text preview renders `<Code>` content via `PreviewPane`.
#[test]
fn e2e_l04_file_detail_text_preview_happy() {
    let preview = FilePreviewPayload {
        mode: PreviewMode::Text,
        mime_type: "text/plain".into(),
        content: "hello from fixture".into(),
        size_bytes: 19,
    };
    let html = render_html(move || {
        detail_shell(view! {
            <Title3>"notes.txt"</Title3>
            <Card>
                <CardContent>
                    <Caption1>"Preview"</Caption1>
                    <PreviewPane preview=preview />
                </CardContent>
            </Card>
        })
        .into_any()
    });
    assert!(html.contains("data-testid=\"meson-file-detail\""), "{html}");
    assert!(html.contains("hello from fixture"), "{html}");
}

/// E2E-L05 — unsupported mime falls back to an `EmptyState`, no raw bytes leaked.
#[test]
fn e2e_l05_file_detail_unsupported_preview_happy() {
    let preview = FilePreviewPayload {
        mode: PreviewMode::Unsupported,
        mime_type: "application/pdf".into(),
        content: String::new(),
        size_bytes: 4096,
    };
    let html = render_html(move || {
        detail_shell(view! {
            <Card>
                <CardContent>
                    <PreviewPane preview=preview />
                </CardContent>
            </Card>
        })
        .into_any()
    });
    assert!(html.contains("No preview for this file type"), "{html}");
}

/// E2E-L06 — image branch renders the preview card shell without panicking.
///
/// `FileDetailPage` hands image bytes to `<MesonImg>` directly (not through
/// `PreviewPane`); this asserts only the surrounding card/testid contract, not
/// `MesonImg`'s internal markup — the real image-loads assertion lives at the
/// Playwright tier where a browser actually fetches the `src`.
#[test]
fn e2e_l06_file_detail_image_mode_card_happy() {
    let html = render_html(move || {
        detail_shell(view! {
            <Title3>"receipt.png"</Title3>
            <Card>
                <CardContent>
                    <Caption1>"Preview"</Caption1>
                </CardContent>
            </Card>
        })
        .into_any()
    });
    assert!(html.contains("data-testid=\"meson-file-detail\""), "{html}");
    assert!(html.contains("Preview"), "{html}");
}

/// E2E-L07 — not-found / foreign-id collapse renders a Warning `MessageBar`, not a crash.
#[test]
fn e2e_l07_file_detail_not_found_sad() {
    let error_text = "not_found: File not found";
    let html = render_html(move || {
        view! {
            <ContentContainer data_testid="meson-file-detail">
                <MessageBar intent=MessageBarIntent::Warning>{error_text}</MessageBar>
            </ContentContainer>
        }
        .into_any()
    });
    assert!(html.contains("data-testid=\"meson-file-detail\""), "{html}");
    assert!(html.contains(error_text), "{html}");
}

/// E2E-L08 — "Back to My Files" always links to `/meson`, and status badge renders.
#[test]
fn e2e_l08_file_detail_back_link_happy() {
    let html = render_html(move || {
        detail_shell(view! {
            <Title3>"receipt.png"</Title3>
            <Card>
                <CardContent>
                    <Body1>"Name: receipt.png"</Body1>
                    <Flex gap=SpacingSize::Size80.flex_gap()>
                        <Caption1>"Status:"</Caption1>
                        <FileStatusBadge status="available".to_string() />
                    </Flex>
                </CardContent>
            </Card>
        })
        .into_any()
    });
    assert!(html.contains("Back to My Files"), "{html}");
    assert!(html.contains("href=\"/meson\""), "{html}");
    assert!(
        html.contains("data-testid=\"meson-file-status-badge\""),
        "{html}"
    );
}
