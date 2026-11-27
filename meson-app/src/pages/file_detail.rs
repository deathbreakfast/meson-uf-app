//! A file's page: breadcrumb, title with its scan badge, Open / Download /
//! Delete, a preview that says what's going on while the file is scanned,
//! flagged or can't be shown, and a Details card.

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::{use_navigate, use_params_map};
use uf_product::components::{
    Body1, Caption1, Caption1Strong, Card, CardContent, CardHeader, ContentContainer, SpacingSize,
    Subtitle1, Title2,
};
use uf_product::primitives::{
    use_breakpoint_down, Badge, BadgeAppearance, BadgeColor, BadgeSize, Breadcrumb,
    BreadcrumbButton, BreadcrumbDivider, BreadcrumbItem, Breakpoint, Button, ButtonAppearance,
    Flex, FlexAlign, FlexJustify, FlexWrap, Grid, GridConfig, GridItem, GridItemConfig, Link,
    MessageBar, MessageBarActions, MessageBarBody, MessageBarIntent, MessageBarLayout,
    MessageBarTitle, Skeleton, SkeletonItem,
};

use crate::components::{
    classify, DeleteDialog, DeleteTarget, ErrorBar, ErrorKind, NotInYourFiles,
};
use crate::format::{exact_bytes, hours_since, size_label, storage_label, time_label, type_label};
use crate::server::{
    download_my_file, get_my_file, get_my_file_preview, subscribe_list_my_files, DownloadPayload,
    FilePreviewPayload, MyFileDetail,
};

mod preview;

use preview::PreviewCard;
pub use preview::PreviewPane;

/// A pending scan older than this gets the "taking longer than usual" warning.
const SLOW_SCAN_HOURS: i64 = 2;

/// Where a file is in its virus scan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scan {
    Clean,
    Scanning,
    Quarantined,
}

impl Scan {
    fn from_status(status: &str) -> Self {
        match status {
            "available" => Self::Clean,
            "quarantined" => Self::Quarantined,
            _ => Self::Scanning,
        }
    }

    fn badge(self) -> (&'static str, BadgeColor) {
        match self {
            Self::Clean => ("Clean", BadgeColor::Success),
            Self::Scanning => ("Scanning", BadgeColor::Informative),
            Self::Quarantined => ("Quarantined", BadgeColor::Danger),
        }
    }
}

/// Detail view: one row, its actions and a type-based preview.
#[component]
pub fn FileDetailPage() -> impl IntoView {
    let params = use_params_map();
    let id = Memo::new(move |_| {
        params
            .read()
            .get("id")
            .map(|s| s.replace("%3A", ":"))
            .unwrap_or_default()
    });

    // Same `meson.file.updated` topic as the list (one WS registration). A
    // scan finishing refetches both resources in place; `Transition` keeps
    // the page on screen while they reload.
    let ws_trigger = subscribe_list_my_files(|| {});

    let meta_res = Resource::new(
        move || (id.get(), ws_trigger.get()),
        |(eid, _)| async move {
            if eid.is_empty() {
                return Err(ServerFnError::new("not_found: missing id"));
            }
            get_my_file(eid).await
        },
    );

    let preview_res = Resource::new(
        move || (id.get(), ws_trigger.get()),
        |(eid, _)| async move {
            if eid.is_empty() {
                return Err(ServerFnError::new("not_found: missing id"));
            }
            get_my_file_preview(eid).await
        },
    );

    view! {
        <ContentContainer data_testid="meson-file-detail">
            <Flex vertical=true gap=SpacingSize::Size240.flex_gap()>
                <Transition fallback=move || view! { <LoadingPage /> }>
                    {move || match meta_res.get() {
                        Some(Ok(detail)) => view! {
                            <FileView detail=detail preview_res=preview_res />
                        }.into_any(),
                        Some(Err(e)) if classify(&e) == ErrorKind::NotFound => view! {
                            <FileBreadcrumb name="File".to_string() />
                            <NotInYourFiles />
                        }.into_any(),
                        Some(Err(e)) => view! {
                            <FileBreadcrumb name="File".to_string() />
                            <ErrorBar
                                error=e
                                what="this file"
                                on_retry=Callback::new(move |()| meta_res.refetch())
                            />
                        }.into_any(),
                        None => view! { <LoadingPage /> }.into_any(),
                    }}
                </Transition>
            </Flex>
        </ContentContainer>
    }
}

/// My Files › name.
#[component]
fn FileBreadcrumb(name: String) -> impl IntoView {
    view! {
        <nav data-testid="meson-file-breadcrumb" aria-label="Breadcrumb">
            <Breadcrumb>
                <BreadcrumbItem>
                    <BreadcrumbButton href="/meson">"My Files"</BreadcrumbButton>
                </BreadcrumbItem>
                <BreadcrumbDivider />
                <BreadcrumbItem>
                    <BreadcrumbButton current=true>{name}</BreadcrumbButton>
                </BreadcrumbItem>
            </Breadcrumb>
        </nav>
    }
}

/// Skeleton of the title and both cards while the row loads.
#[component]
fn LoadingPage() -> impl IntoView {
    view! {
        <div data-testid="meson-file-loading">
            <Flex vertical=true gap=SpacingSize::Size240.flex_gap()>
                <Skeleton>
                    <SkeletonItem width="280px" height="28px" />
                </Skeleton>
                <TwoThirds
                    main=|| view! {
                        <Card><CardContent>
                            <Skeleton><SkeletonItem width="100%" height="128px" /></Skeleton>
                        </CardContent></Card>
                    }
                    side=|| view! {
                        <Card><CardContent>
                            <Skeleton>
                                <Flex vertical=true gap=SpacingSize::Size120.flex_gap()>
                                    <SkeletonItem width="60%" />
                                    <SkeletonItem width="80%" />
                                    <SkeletonItem width="50%" />
                                    <SkeletonItem width="70%" />
                                    <SkeletonItem width="40%" />
                                    <SkeletonItem width="65%" />
                                </Flex>
                            </Skeleton>
                        </CardContent></Card>
                    }
                />
            </Flex>
        </div>
    }
}

/// Preview on two thirds and Details on one third; stacked below the Md
/// breakpoint. Only a breakpoint change rebuilds the columns.
#[component]
fn TwoThirds(#[prop(into)] main: ViewFn, #[prop(into)] side: ViewFn) -> impl IntoView {
    let narrow = use_breakpoint_down(Breakpoint::Md);
    view! {
        {move || {
            let narrow = narrow.get();
            let (cols, main_span) = if narrow { (1, 1) } else { (3, 2) };
            let (main, side) = (main.clone(), side.clone());
            view! {
                <div data-testid="meson-file-columns" data-narrow=narrow.to_string()>
                    <Grid config=GridConfig::with_gaps(cols, 16, 16)>
                        <GridItem config=GridItemConfig::span(main_span)>{main.run()}</GridItem>
                        <GridItem config=GridItemConfig::span(1)>{side.run()}</GridItem>
                    </Grid>
                </div>
            }
        }}
    }
}

/// The page once the row is loaded.
#[component]
fn FileView(
    detail: MyFileDetail,
    preview_res: Resource<Result<FilePreviewPayload, ServerFnError>>,
) -> impl IntoView {
    let file = detail.file.clone();
    let scan = Scan::from_status(&file.file_status);
    let slow = scan == Scan::Scanning
        && hours_since(&file.uploaded_at, chrono::Utc::now()) >= SLOW_SCAN_HOURS;
    let app_name = detail.app_name.clone();
    let open_url = detail.open_url.clone();

    let delete_open = RwSignal::new(false);
    let target = DeleteTarget {
        id: file.id.clone(),
        file_name: file.file_name.clone(),
        app_name: app_name.clone(),
        size_bytes: file.size_bytes,
        quarantined: scan == Scan::Quarantined,
    };
    let navigate = use_navigate();
    let on_deleted = Callback::new(move |()| navigate("/meson", Default::default()));
    let ask_delete = Callback::new(move |_| delete_open.set(true));

    let downloading = RwSignal::new(false);
    let download_failed = RwSignal::new(false);
    let download_id = file.id.clone();
    let download = Callback::new(move |_| {
        if downloading.get_untracked() {
            return;
        }
        downloading.set(true);
        download_failed.set(false);
        let id = download_id.clone();
        spawn_local(async move {
            match download_my_file(id).await {
                Ok(payload) => {
                    if save_download(&payload).is_err() {
                        download_failed.set(true);
                    }
                }
                Err(_) => download_failed.set(true),
            }
            downloading.set(false);
        });
    });

    let (badge_label, badge_color) = scan.badge();
    let ready = scan == Scan::Clean;
    let open_label = format!("Open in {app_name}");
    let open_disabled = !ready || open_url.is_none();
    let delete_appearance = if scan == Scan::Quarantined {
        ButtonAppearance::Danger
    } else {
        ButtonAppearance::Secondary
    };

    let main_detail = detail.clone();
    let main = move || {
        view! {
            <PreviewCard
                detail=main_detail.clone()
                scan=scan
                slow=slow
                preview_res=preview_res
                on_download=download
            />
        }
    };
    let side = move || view! { <DetailsCard detail=detail.clone() scan=scan /> };

    view! {
        <FileBreadcrumb name=file.file_name.clone() />
        <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center wrap=FlexWrap::Wrap gap=SpacingSize::Size160.flex_gap()>
            <Flex align=FlexAlign::Center wrap=FlexWrap::Wrap gap=SpacingSize::Size80.flex_gap()>
                <span data-testid="meson-file-title"><Title2>{file.file_name.clone()}</Title2></span>
                <span data-testid="meson-file-scan-badge" data-scan=badge_label>
                    <Badge appearance=BadgeAppearance::Tint color=badge_color size=BadgeSize::Small>
                        {badge_label}
                    </Badge>
                </span>
            </Flex>
            <Flex wrap=FlexWrap::Wrap gap=SpacingSize::Size80.flex_gap()>
                <OpenButton label=open_label url=open_url disabled=open_disabled />
                <Button
                    appearance=ButtonAppearance::Secondary
                    icon=icondata::AiDownloadOutlined
                    disabled=!ready
                    loading=downloading
                    attr:data-testid="meson-file-download"
                    on_click=download
                >
                    "Download"
                </Button>
                <Button
                    appearance=delete_appearance
                    icon=icondata::AiDeleteOutlined
                    attr:data-testid="meson-file-delete"
                    attr:data-appearance=if scan == Scan::Quarantined { "danger" } else { "secondary" }
                    on_click=ask_delete
                >
                    "Delete"
                </Button>
            </Flex>
        </Flex>
        <Show when=move || download_failed.get()>
            <div data-testid="meson-download-failed">
                <MessageBar intent=MessageBarIntent::Error layout=MessageBarLayout::Multiline>
                    <MessageBarTitle>"Couldn't download this file"</MessageBarTitle>
                    <MessageBarBody>"Storage didn't send its contents. Nothing was changed. Try again."</MessageBarBody>
                </MessageBar>
            </div>
        </Show>
        <StatusBar scan=scan slow=slow app_name=app_name.clone() on_delete=ask_delete />
        <TwoThirds main=main side=side />
        <DeleteDialog
            open=delete_open
            files=Signal::derive(move || vec![target.clone()])
            on_deleted=on_deleted
        />
    }
}

/// "Open in <app>": a link when the app registered a URL, otherwise a
/// disabled button.
#[component]
fn OpenButton(label: String, url: Option<String>, disabled: bool) -> impl IntoView {
    match url {
        Some(href) if !disabled => view! {
            <span data-testid="meson-file-open">
                <Link href=href>
                    <Button appearance=ButtonAppearance::Primary>{label}</Button>
                </Link>
            </span>
        }
        .into_any(),
        _ => view! {
            <span data-testid="meson-file-open">
                <Button appearance=ButtonAppearance::Primary disabled=true>{label}</Button>
            </span>
        }
        .into_any(),
    }
}

/// The page-level message for a file that's scanning or flagged.
#[component]
fn StatusBar(
    scan: Scan,
    slow: bool,
    app_name: String,
    on_delete: Callback<leptos::ev::MouseEvent>,
) -> impl IntoView {
    match (scan, slow) {
        (Scan::Scanning, false) => view! {
            <div data-testid="meson-file-scanning">
                <MessageBar intent=MessageBarIntent::Info layout=MessageBarLayout::Multiline>
                    <MessageBarTitle>"Checking this file for viruses"</MessageBarTitle>
                    <MessageBarBody>
                        {format!("This usually takes under a minute, and the page updates by itself. Preview, download and Open in {app_name} turn on when the check is done.")}
                    </MessageBarBody>
                </MessageBar>
            </div>
        }
        .into_any(),
        (Scan::Scanning, true) => view! {
            <div data-testid="meson-file-scan-slow">
                <MessageBar intent=MessageBarIntent::Warning layout=MessageBarLayout::Multiline>
                    <MessageBarTitle>"The virus check is taking longer than usual"</MessageBarTitle>
                    <MessageBarBody>
                        "This file has waited over 2 hours. The scanner may be behind or stopped; a site owner can see why on the scanner's status page. You can wait, or delete the file and upload it again."
                    </MessageBarBody>
                    <MessageBarActions>
                        <Button appearance=ButtonAppearance::Secondary on_click=on_delete>
                            "Delete and upload again"
                        </Button>
                    </MessageBarActions>
                </MessageBar>
            </div>
        }
        .into_any(),
        (Scan::Quarantined, _) => view! {
            <div data-testid="meson-file-quarantined">
                <MessageBar intent=MessageBarIntent::Error layout=MessageBarLayout::Multiline>
                    <MessageBarTitle>"The virus scan flagged this file"</MessageBarTitle>
                    <MessageBarBody>
                        {format!("Meson keeps it locked, so it can't be previewed, downloaded or opened in {app_name}. Nobody else can open it either. Delete it, then upload a clean copy in {app_name}.")}
                    </MessageBarBody>
                    <MessageBarActions>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            attr:data-testid="meson-file-quarantined-delete"
                            on_click=on_delete
                        >
                            "Delete this file"
                        </Button>
                    </MessageBarActions>
                </MessageBar>
            </div>
        }
        .into_any(),
        (Scan::Clean, _) => ().into_any(),
    }
}

/// Details card: app, type, size, upload time, scan result and storage.
#[component]
fn DetailsCard(detail: MyFileDetail, scan: Scan) -> impl IntoView {
    let file = detail.file;
    let uploaded = file.uploaded_at.clone();
    let app = match detail.open_url {
        Some(href) => view! {
            {detail.app_name.clone()} " · "
            <Link href=href>{format!("Open in {}", detail.app_name)}</Link>
        }
        .into_any(),
        None => view! { {detail.app_name.clone()} }.into_any(),
    };
    let scan_text = match scan {
        Scan::Clean => "Clean",
        Scan::Scanning => "Scanning",
        Scan::Quarantined => "Flagged by the virus scan",
    };
    let storage = detail
        .storage
        .map(|s| storage_label(s.stored_bytes, s.copies, s.cost_per_month, &s.currency));
    view! {
        <Card>
            <CardHeader><Subtitle1>"Details"</Subtitle1></CardHeader>
            <CardContent>
                <div data-testid="meson-file-details">
                    <Flex vertical=true gap=SpacingSize::Size160.flex_gap()>
                        <DetailRow label="App" key="app">{app}</DetailRow>
                        <DetailRow label="Type" key="type">{type_label(&file.mime_type)}</DetailRow>
                        <DetailRow label="Size" key="size">
                            {size_label(file.size_bytes)} " "
                            <Caption1>{format!("({})", exact_bytes(file.size_bytes))}</Caption1>
                        </DetailRow>
                        <DetailRow label="Uploaded" key="uploaded">
                            <time datetime=uploaded.clone()>{time_label(&uploaded)}</time>
                        </DetailRow>
                        <DetailRow label="Virus scan" key="scan">{scan_text}</DetailRow>
                        {storage.map(|text| view! {
                            <DetailRow label="Storage" key="storage">{text}</DetailRow>
                        })}
                    </Flex>
                </div>
            </CardContent>
        </Card>
    }
}

#[component]
fn DetailRow(label: &'static str, key: &'static str, children: Children) -> impl IntoView {
    view! {
        <div data-testid=format!("meson-detail-{key}")>
            <Flex vertical=true>
                <Caption1Strong>{label}</Caption1Strong>
                <Body1>{children()}</Body1>
            </Flex>
        </div>
    }
}

/// Hand the bytes to the browser as a download under the file's name.
fn save_download(payload: &DownloadPayload) -> Result<(), wasm_bindgen::JsValue> {
    use wasm_bindgen::JsCast;

    let bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &payload.content_base64,
    )
    .map_err(|_| wasm_bindgen::JsValue::from_str("bad base64"))?;
    let array = js_sys::Uint8Array::from(bytes.as_slice());
    let parts = js_sys::Array::of1(&array);
    let options = web_sys::BlobPropertyBag::new();
    options.set_type(&payload.mime_type);
    let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)?;
    let anchor: web_sys::HtmlAnchorElement = document().create_element("a")?.unchecked_into();
    anchor.set_href(&url);
    anchor.set_download(&payload.file_name);
    anchor.click();
    web_sys::Url::revoke_object_url(&url)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_states_from_status() {
        assert_eq!(Scan::from_status("available"), Scan::Clean);
        assert_eq!(Scan::from_status("pending_virus_scan"), Scan::Scanning);
        assert_eq!(Scan::from_status("virus_scan_complete"), Scan::Scanning);
        assert_eq!(Scan::from_status("quarantined"), Scan::Quarantined);
        assert_eq!(Scan::Quarantined.badge().0, "Quarantined");
    }
}
