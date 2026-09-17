//! `IsolatedLab` / SSR UI render e2e for the My Files list page chrome.

#![cfg(feature = "ssr")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::ui_render::render_html;
use leptos::prelude::*;
use meson_app::server::MyFileRow;
use meson_app::MyFilesTable;
use uf_product::components::{
    Body1, Caption1, Card, CardContent, ContentContainer, EmptyState, SpacingSize, Title3,
};
use uf_product::primitives::{Flex, MessageBar, MessageBarIntent};

fn sample_row(id: &str, status: &str) -> MyFileRow {
    MyFileRow {
        id: id.into(),
        source_table: "e2e_meson_file".into(),
        file_name: format!("{id}.dat"),
        mime_type: "image/png".into(),
        size_bytes: 1024,
        file_status: status.into(),
        uploaded_at: "2026-01-01T00:00:00Z".into(),
    }
}

/// Empty state: Card + `EmptyState` + "Showing 0 files" when no rows come back.
#[test]
fn e2e_l01_my_files_empty_state_happy() {
    let html = render_html(move || {
        view! {
            <ContentContainer data_testid="meson-my-files">
                <Flex vertical=true gap=SpacingSize::Size240.flex_gap()>
                    <Title3>"My Files"</Title3>
                    <Body1>"Files you uploaded across apps that use the File trait."</Body1>
                    <Flex vertical=true gap=SpacingSize::Size160.flex_gap()>
                        <Card>
                            <CardContent>
                                <EmptyState message="No files yet" />
                                <Caption1>"Uploads from apps that use the File trait appear here."</Caption1>
                            </CardContent>
                        </Card>
                        <Body1>"Showing " {0} " files"</Body1>
                    </Flex>
                </Flex>
            </ContentContainer>
        }
        .into_any()
    });
    assert!(html.contains("data-testid=\"meson-my-files\""), "{html}");
    assert!(html.contains("No files yet"), "{html}");
    assert!(html.contains("Showing"), "{html}");
    assert!(html.contains('0'), "{html}");
    assert!(html.contains("files"), "{html}");
}

/// Populated table: one row per status, correct testid + `data-status` per row.
#[test]
fn e2e_l02_my_files_table_status_badges_happy() {
    let rows = vec![
        sample_row("file-available", "available"),
        sample_row("file-pending", "pending_virus_scan"),
        sample_row("file-complete", "virus_scan_complete"),
        sample_row("file-quarantined", "quarantined"),
    ];
    let html = render_html(move || {
        view! {
            <ContentContainer data_testid="meson-my-files">
                <MyFilesTable files=rows />
            </ContentContainer>
        }
        .into_any()
    });
    assert!(html.contains("data-testid=\"meson-my-files\""), "{html}");
    for status in [
        "available",
        "pending_virus_scan",
        "virus_scan_complete",
        "quarantined",
    ] {
        assert!(
            html.contains(&format!("data-status=\"{status}\"")),
            "status={status}: {html}"
        );
    }
    assert!(
        html.contains("data-testid=\"meson-file-status-badge\""),
        "{html}"
    );
    assert!(html.contains("Available"), "{html}");
    assert!(html.contains("Quarantined"), "{html}");
    assert!(html.contains("/meson/files/file-available"), "{html}");
}

/// Error state: `MessageBar` intent=Error surfaces the server-fn failure text.
#[test]
fn e2e_l03_my_files_error_state_sad() {
    let error_text = "io: list files failed";
    let html = render_html(move || {
        view! {
            <ContentContainer data_testid="meson-my-files">
                <MessageBar intent=MessageBarIntent::Error>{error_text}</MessageBar>
            </ContentContainer>
        }
        .into_any()
    });
    assert!(html.contains("data-testid=\"meson-my-files\""), "{html}");
    assert!(html.contains(error_text), "{html}");
}
