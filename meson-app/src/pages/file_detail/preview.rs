//! The Preview card: what the file page shows for each scan state, then for
//! each file type (image, text, too big, no preview).

use leptos::prelude::*;
use meson_leptos::MesonImg;
use uf_product::components::{
    Caption1, Card, CardContent, CardHeader, EmptyState, EmptyStateCallToAction, SpacingSize,
    Subtitle1,
};
use uf_product::primitives::{
    Button, ButtonAppearance, Code, Flex, FlexAlign, MessageBar, MessageBarActions, MessageBarBody,
    MessageBarIntent, MessageBarLayout, MessageBarTitle, Skeleton, SkeletonItem, Spinner,
};

use super::Scan;
use crate::format::{size_label, time_label, type_label};
use crate::server::{FilePreviewPayload, MyFileDetail, PreviewMode, PREVIEW_LIMIT_BYTES};

/// Preview card: the state for the file's scan, then its type.
#[component]
pub(super) fn PreviewCard(
    detail: MyFileDetail,
    scan: Scan,
    slow: bool,
    preview_res: Resource<Result<FilePreviewPayload, ServerFnError>>,
    on_download: Callback<leptos::ev::MouseEvent>,
) -> impl IntoView {
    let file = detail.file;
    let body = match scan {
        Scan::Scanning if slow => view! {
            <div data-testid="meson-preview-waiting">
                <EmptyState message="Preview opens after the virus check" icon=icondata::AiClockCircleOutlined />
            </div>
        }
        .into_any(),
        Scan::Scanning => {
            let uploaded = file.uploaded_at.clone();
            view! {
                <div data-testid="meson-preview-scanning">
                    <Flex vertical=true align=FlexAlign::Center gap=SpacingSize::Size80.flex_gap()>
                        <Spinner label="Checking for viruses" />
                        <Caption1>"Scanning since"</Caption1>
                        <Caption1><time datetime=uploaded.clone()>{time_label(&uploaded)}</time></Caption1>
                    </Flex>
                </div>
            }
            .into_any()
        }
        Scan::Quarantined => view! {
            <div data-testid="meson-preview-blocked">
                <EmptyState
                    message="Preview blocked"
                    description="Flagged files are never shown."
                    icon=icondata::AiLockOutlined
                />
            </div>
        }
        .into_any(),
        Scan::Clean if file.mime_type.starts_with("image/") => {
            view! { <ImagePreview file_id=file.id.clone() alt=file.file_name.clone() size_bytes=file.size_bytes on_download=on_download /> }
                .into_any()
        }
        Scan::Clean => view! {
            <Transition fallback=move || view! { <PreviewSkeleton /> }>
                {move || match preview_res.get() {
                    Some(Ok(preview)) => view! { <PreviewPane preview=preview on_download=on_download /> }.into_any(),
                    Some(Err(_)) => view! {
                        <PreviewFailed on_retry=Callback::new(move |()| preview_res.refetch()) />
                    }.into_any(),
                    None => view! { <PreviewSkeleton /> }.into_any(),
                }}
            </Transition>
        }
        .into_any(),
    };
    view! {
        <Card>
            <CardHeader><Subtitle1>"Preview"</Subtitle1></CardHeader>
            <CardContent>{body}</CardContent>
        </Card>
    }
}

#[component]
fn PreviewSkeleton() -> impl IntoView {
    view! {
        <div data-testid="meson-preview-loading">
            <Skeleton><SkeletonItem width="100%" height="128px" /></Skeleton>
        </div>
    }
}

/// An image through `MesonImg`; a failed load turns into a Try again bar.
#[component]
fn ImagePreview(
    file_id: String,
    alt: String,
    size_bytes: i64,
    on_download: Callback<leptos::ev::MouseEvent>,
) -> impl IntoView {
    let too_big = usize::try_from(size_bytes).map_or(true, |n| n > PREVIEW_LIMIT_BYTES);
    if too_big {
        return view! { <TooBig size_bytes=size_bytes on_download=Some(on_download) /> }.into_any();
    }
    let failed = RwSignal::new(false);
    let attempt = RwSignal::new(0u32);
    view! {
        {move || {
            attempt.track();
            if failed.get() {
                view! {
                    <PreviewFailed on_retry=Callback::new(move |()| {
                        failed.set(false);
                        attempt.update(|n| *n += 1);
                    }) />
                }
                .into_any()
            } else {
                view! {
                    <MesonImg
                        file_id=file_id.clone()
                        alt=alt.clone()
                        width="100%".to_string()
                        height="320px".to_string()
                        on_error=Callback::new(move |_msg: String| failed.set(true))
                    />
                }
                .into_any()
            }
        }}
    }
    .into_any()
}

#[component]
fn PreviewFailed(on_retry: Callback<()>) -> impl IntoView {
    view! {
        <div data-testid="meson-preview-failed">
            <MessageBar intent=MessageBarIntent::Error layout=MessageBarLayout::Multiline>
                <MessageBarTitle>"Couldn't load the preview"</MessageBarTitle>
                <MessageBarBody>
                    "The file is listed, but storage didn't send its contents. Nothing was changed. Try again, or download it."
                </MessageBarBody>
                <MessageBarActions>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        attr:data-testid="meson-preview-retry"
                        on_click=Callback::new(move |_| on_retry.run(()))
                    >
                        "Try again"
                    </Button>
                </MessageBarActions>
            </MessageBar>
        </div>
    }
}

#[component]
fn TooBig(size_bytes: i64, on_download: Option<Callback<leptos::ev::MouseEvent>>) -> impl IntoView {
    view! {
        <div data-testid="meson-preview-too-big">
            <EmptyState
                message="Too big to preview"
                description="Previews stop at 5 MB. Download it to read it."
                icon=icondata::AiFileOutlined
            >
                <EmptyStateCallToAction slot:call_to_action>
                    <DownloadButton on_download=on_download />
                </EmptyStateCallToAction>
            </EmptyState>
            <Caption1>{format!("This file is {}.", size_label(size_bytes))}</Caption1>
        </div>
    }
}

/// Renders preview bytes per [`PreviewMode`]: text inline, too big and
/// unsupported as empty states with Download.
#[component]
pub fn PreviewPane(
    preview: FilePreviewPayload,
    /// Download, offered when the file can't be shown here.
    #[prop(optional)]
    on_download: Option<Callback<leptos::ev::MouseEvent>>,
) -> impl IntoView {
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
            view! { <div data-testid="meson-preview-text"><Code text=text /></div> }.into_any()
        }
        PreviewMode::TooBig => {
            view! { <TooBig size_bytes=preview.size_bytes on_download=on_download /> }.into_any()
        }
        PreviewMode::Unsupported => {
            let kind = type_label(&preview.mime_type);
            view! {
                <div data-testid="meson-preview-unsupported">
                    <EmptyState
                        message="No preview for this file type"
                        description="Download it, or open it in the app that stored it."
                        icon=icondata::AiFileOutlined
                    >
                        <EmptyStateCallToAction slot:call_to_action>
                            <DownloadButton on_download=on_download />
                        </EmptyStateCallToAction>
                    </EmptyState>
                    <Caption1>{format!("{kind} files can't be shown here.")}</Caption1>
                </div>
            }
            .into_any()
        }
    }
}

/// Download, when the caller offers it; nothing otherwise.
#[component]
fn DownloadButton(on_download: Option<Callback<leptos::ev::MouseEvent>>) -> impl IntoView {
    on_download.map(|cb| {
        view! {
            <Button appearance=ButtonAppearance::Secondary icon=icondata::AiDownloadOutlined on_click=cb>
                "Download"
            </Button>
        }
    })
}
