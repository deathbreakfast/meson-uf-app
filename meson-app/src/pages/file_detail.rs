//! File detail / preview page.

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use meson_leptos::MesonImg;
use uf_product::components::{
    Body1, Caption1, Card, CardContent, CardHeader, ContentContainer, EmptyState, SpacingSize,
    Subtitle1, Title3,
};
use uf_product::primitives::{
    Button, ButtonAppearance, Code, Flex, Link, MessageBar, MessageBarIntent,
};

use crate::components::FileStatusBadge;
use crate::server::{
    get_my_file, get_my_file_preview, subscribe_list_my_files, FilePreviewPayload, MyFileRow,
    PreviewMode,
};

/// Detail view: metadata + type-based preview.
#[component]
pub fn FileDetailPage() -> impl IntoView {
    let params = use_params_map();
    let id = move || {
        params
            .get()
            .get("id")
            .map(|s| s.replace("%3A", ":"))
            .unwrap_or_default()
    };

    // Same `meson.file.updated` topic as the list (one WS registration).
    let ws_trigger = subscribe_list_my_files(|| {});

    let meta_res = Resource::new(
        move || (id(), ws_trigger.get()),
        |(eid, _)| async move {
            if eid.is_empty() {
                return Err(ServerFnError::new("missing id"));
            }
            get_my_file(eid).await
        },
    );

    let preview_res = Resource::new(
        move || (id(), ws_trigger.get()),
        |(eid, _)| async move {
            if eid.is_empty() {
                return Err(ServerFnError::new("missing id"));
            }
            get_my_file_preview(eid).await
        },
    );

    view! {
        <ContentContainer data_testid="meson-file-detail">
            <Flex vertical=true gap=SpacingSize::Size240.flex_gap()>
                <Link href="/meson">
                    <Button appearance=ButtonAppearance::Subtle>"Back to My Files"</Button>
                </Link>

                <Suspense fallback=move || view! { <Card><CardContent>"Loading…"</CardContent></Card> }>
                    {move || match meta_res.get() {
                        Some(Ok(meta)) => {
                            let title = meta.file_name.clone();
                            let alt = title.clone();
                            let file_id = meta.id.clone();
                            let is_image = meta.mime_type.starts_with("image/");
                            view! {
                                <Title3>{title}</Title3>
                                <Flex gap=SpacingSize::Size240.flex_gap()>
                                    <Card>
                                        <CardHeader>
                                            <Subtitle1>"Preview"</Subtitle1>
                                        </CardHeader>
                                        <CardContent>
                                            {if is_image {
                                                view! {
                                                    <MesonImg
                                                        file_id=file_id
                                                        alt=alt
                                                        width="100%".to_string()
                                                        height="320px".to_string()
                                                    />
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <Suspense fallback=move || view! { <Body1>"Loading preview…"</Body1> }>
                                                        {move || match preview_res.get() {
                                                            Some(Ok(preview)) => view! { <PreviewPane preview=preview /> }.into_any(),
                                                            Some(Err(e)) => view! {
                                                                <MessageBar intent=MessageBarIntent::Error>{e.to_string()}</MessageBar>
                                                            }.into_any(),
                                                            None => view! { <Body1>"Loading preview…"</Body1> }.into_any(),
                                                        }}
                                                    </Suspense>
                                                }.into_any()
                                            }}
                                        </CardContent>
                                    </Card>
                                    <FileDetailsCard meta=meta />
                                </Flex>
                            }.into_any()
                        }
                        Some(Err(e)) => view! {
                            <MessageBar intent=MessageBarIntent::Warning>{e.to_string()}</MessageBar>
                        }.into_any(),
                        None => view! { <Card><CardContent>"Loading…"</CardContent></Card> }.into_any(),
                    }}
                </Suspense>
            </Flex>
        </ContentContainer>
    }
}

/// Metadata card: name, type, size, status badge, upload time, source table.
#[component]
fn FileDetailsCard(meta: MyFileRow) -> impl IntoView {
    let status = meta.file_status.clone();
    view! {
        <Card>
            <CardHeader>
                <Subtitle1>"Details"</Subtitle1>
            </CardHeader>
            <CardContent>
                <Flex vertical=true gap=SpacingSize::Size120.flex_gap()>
                    <Body1>{format!("Name: {}", meta.file_name)}</Body1>
                    <Caption1>{format!("Type: {}", meta.mime_type)}</Caption1>
                    <Caption1>{format!("Size: {} bytes", meta.size_bytes)}</Caption1>
                    <Flex gap=SpacingSize::Size80.flex_gap()>
                        <Caption1>"Status:"</Caption1>
                        <FileStatusBadge status=status />
                    </Flex>
                    <Caption1>{format!("Uploaded: {}", meta.uploaded_at)}</Caption1>
                    <Caption1>{format!("Source: {}", meta.source_table)}</Caption1>
                </Flex>
            </CardContent>
        </Card>
    }
}

/// Renders preview bytes per [`PreviewMode`] (text inline, image handled by the caller, unsupported as an empty state).
#[component]
pub fn PreviewPane(preview: FilePreviewPayload) -> impl IntoView {
    match preview.mode {
        PreviewMode::Image => {
            // Images use MesonImg on the parent; keep a safe fallback for unexpected mime.
            view! {
                <EmptyState message="Open this file from the image preview above" />
            }
            .into_any()
        }
        PreviewMode::Text => {
            let text = preview.content;
            view! { <Code text=text /> }.into_any()
        }
        PreviewMode::Unsupported => view! {
            <EmptyState message="No preview for this file type" />
            <Body1>"Open the file locally after downloading from the product that uploaded it."</Body1>
        }
        .into_any(),
    }
}
