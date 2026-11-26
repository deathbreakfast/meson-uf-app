//! Storage page at `/meson/storage`: how much space the signed-in member's
//! files use and roughly what that costs, plus the site-wide Everyone tab and
//! the cost settings for site owners.
//!
//! Every input to the estimate can be missing (no daily run yet, no copy
//! count, no price, no history). The page shows what it knows, says what's
//! missing in a banner, and never turns a missing input into an error.

mod banners;
mod everyone;
pub mod format;
pub mod run;
mod settings;

use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::{use_navigate, use_query_map};
use leptos_router::NavigateOptions;
use uf_product::components::{
    AutoGrid, Body1, Caption1, Caption1Strong, ContentContainer, EmptyState, FlexItem, SpacingSize,
    StatCard, StatCardVariant, Subtitle1, Title2,
};
use uf_product::primitives::{
    AreaChart, AxisDef, AxisPosition, Button, ButtonAppearance, Card, CardContent, CardHeader,
    CardHeaderAction, Flex, FlexWrap, Link, MessageBar, MessageBarActions, MessageBarBody,
    MessageBarIntent, MessageBarLayout, MessageBarTitle, ProgressBar, ScaleType, SeriesDef,
    SkeletonItem, Tab, TabList, Table, TableBody, TableCell, TableHeader, TableHeaderCell,
    TableRow,
};

use crate::components::ErrorBar;
use crate::server::{get_my_storage, get_site_storage, list_my_files, MyFileRow};
use crate::storage::view::{
    AppRow, CopiesSource, HistoryState, PriceSource, SizeSource, StorageView,
};

use banners::Banners;
use everyone::{EveryoneDenied, EveryoneTab};
use format::{
    biggest_files, change_over_history, cost_stat, file_href, files_text, format_bytes,
    format_change, format_cost, format_count, format_price, price_of, quarantined_files,
    with_live_sizes, worked_out,
};
pub use run::{get_storage_run, run_storage_now, StorageRunInfo, JOB_NAME};
#[cfg(feature = "ssr")]
pub use run::{install_storage_run_hook, StorageRunHook};
use settings::SettingsDialog;

/// Gauge permission name for the Everyone tab and the cost settings.
const STORAGE_ADMIN: &str = "StorageAdmin";

const TAB_YOURS: &str = "yours";
const TAB_EVERYONE: &str = "everyone";

/// Biggest files shown on Your storage.
const BIGGEST_FILES: usize = 4;

/// Actions the banners and cards can ask the page for.
#[derive(Clone, Copy)]
struct PageActions {
    /// Viewer holds `StorageAdmin`.
    owner: bool,
    /// Open the cost settings dialog.
    open_settings: Callback<()>,
    /// Switch to the Everyone tab.
    show_everyone: Callback<()>,
    /// Refetch every resource on the page.
    refresh: Callback<()>,
}

/// The Storage page.
#[component]
pub fn StoragePage() -> impl IntoView {
    // `?view=everyone` only picks the first tab; later switches don't track it.
    let initial = use_query_map()
        .get_untracked()
        .get("view")
        .filter(|v| v == TAB_EVERYONE)
        .map_or(TAB_YOURS, |_| TAB_EVERYONE);
    let tab = RwSignal::new(initial.to_string());
    let is_owner = RwSignal::new(None::<bool>);
    let refresh = RwSignal::new(0_u64);
    let settings_open = RwSignal::new(false);

    // Browser-only, like `FilesReadGate`: the server render has no backend.
    Effect::new(move |_| {
        spawn_local(async move {
            let ok = matches!(
                uf_product::permissions::check_permission_by_name(STORAGE_ADMIN.to_string()).await,
                Ok(true)
            );
            is_owner.set(Some(ok));
        });
    });

    // Keep the URL in step with the tab without navigating away.
    let navigate = use_navigate();
    Effect::new(move |prev: Option<String>| {
        let current = tab.get();
        if prev.is_some_and(|p| p != current) {
            let href = if current == TAB_EVERYONE {
                "/meson/storage?view=everyone"
            } else {
                "/meson/storage"
            };
            navigate(
                href,
                NavigateOptions {
                    replace: true,
                    scroll: false,
                    ..Default::default()
                },
            );
        }
        current
    });

    let my_res = Resource::new(move || refresh.get(), |_| get_my_storage());
    let files_res = Resource::new(move || refresh.get(), |_| list_my_files());
    let site_res = Resource::new(
        move || (refresh.get(), is_owner.get() == Some(true)),
        |(_, owner)| async move {
            if owner {
                get_site_storage().await.map(Some)
            } else {
                Ok(None)
            }
        },
    );
    let run_res = Resource::new(
        move || (refresh.get(), is_owner.get() == Some(true)),
        |(_, owner)| async move {
            if owner {
                get_storage_run().await.ok().flatten()
            } else {
                None
            }
        },
    );

    let refresh_all = Callback::new(move |()| refresh.update(|n| *n += 1));
    let actions = move || PageActions {
        owner: is_owner.get() == Some(true),
        open_settings: Callback::new(move |()| settings_open.set(true)),
        show_everyone: Callback::new(move |()| tab.set(TAB_EVERYONE.to_string())),
        refresh: refresh_all,
    };

    let intro = move || {
        if tab.get() == TAB_EVERYONE && is_owner.get() == Some(true) {
            "How much space every file on the site uses, and roughly what it costs."
        } else {
            "How much space your files use, and roughly what that costs the site."
        }
    };

    view! {
        <ContentContainer data_testid="meson-storage">
            <Flex vertical=true gap=SpacingSize::Size240.flex_gap()>
                <Flex vertical=true gap=SpacingSize::Size80.flex_gap()>
                    <Title2>"Storage"</Title2>
                    <Body1>{intro}</Body1>
                </Flex>
                <div data-testid="storage-tabs">
                    <TabList selected_value=tab>
                        <Tab value=TAB_YOURS>"Your storage"</Tab>
                        {move || {
                            (is_owner.get() == Some(true))
                                .then(|| view! { <Tab value=TAB_EVERYONE>"Everyone"</Tab> })
                        }}
                    </TabList>
                </div>
                {move || {
                    let on_everyone = tab.get() == TAB_EVERYONE;
                    match (on_everyone, is_owner.get()) {
                        (false, _) => {
                            view! { <YourStorage my_res=my_res files_res=files_res actions=Signal::derive(actions) /> }
                                .into_any()
                        }
                        (true, Some(true)) => {
                            view! {
                                <EveryoneTab
                                    site_res=site_res
                                    run_res=run_res
                                    actions=Signal::derive(actions)
                                />
                            }
                                .into_any()
                        }
                        (true, Some(false)) => {
                            view! {
                                <EveryoneDenied on_back=Callback::new(move |()| {
                                    tab.set(TAB_YOURS.to_string());
                                }) />
                            }
                                .into_any()
                        }
                        (true, None) => ().into_any(),
                    }
                }}
            </Flex>
            <SettingsDialog open=settings_open site_res=site_res on_saved=refresh_all />
        </ContentContainer>
    }
}

/// The member's own storage.
#[component]
fn YourStorage(
    my_res: Resource<Result<StorageView, ServerFnError>>,
    files_res: Resource<Result<Vec<MyFileRow>, ServerFnError>>,
    actions: Signal<PageActions>,
) -> impl IntoView {
    view! {
        <Transition fallback=StorageSkeleton>
            {move || {
                let files = files_res.get();
                match my_res.get() {
                    Some(Ok(view)) => {
                        let files = files.and_then(Result::ok).unwrap_or_default();
                        let view = with_live_sizes(view, &files);
                        your_storage_view(view, &files, actions.get()).into_any()
                    }
                    Some(Err(e)) => view! {
                        <ErrorBar
                            error=e
                            what="your storage"
                            on_retry=Callback::new(move |()| actions.get_untracked().refresh.run(()))
                        />
                    }
                    .into_any(),
                    None => view! { <StorageSkeleton /> }.into_any(),
                }
            }}
        </Transition>
    }
}

fn your_storage_view(view: StorageView, files: &[MyFileRow], actions: PageActions) -> AnyView {
    if view.size != SizeSource::Unavailable && view.totals.files == 0 {
        return view! {
            <div data-testid="storage-empty">
                <EmptyState
                    message="You aren't using any storage"
                    description="Files you upload in any app on this site count here."
                    icon=icondata::AiCloudOutlined
                />
            </div>
        }
        .into_any();
    }

    let used = if view.size == SizeSource::Live {
        format!("About {}", format_bytes(view.totals.bytes))
    } else {
        format_bytes(view.totals.bytes)
    };
    let (cost_label, cost_value) = cost_stat(&view).map_or_else(
        || {
            (
                "Stored with copies",
                format_bytes(view.totals.bytes_with_copies),
            )
        },
        |c| ("Estimated cost a month", c),
    );
    let change = change_over_history(&view.history);
    let (change_value, change_variant) = match (&view.history, change) {
        (_, Some(delta)) => (
            format_change(delta),
            (delta > 0).then_some(StatCardVariant::Warning),
        ),
        (HistoryState::Down, None) => ("Not available".to_string(), None),
        (_, None) => ("No history yet".to_string(), None),
    };
    let show_cost = view.price != PriceSource::Unset;
    let biggest = biggest_files(files, BIGGEST_FILES);
    let quarantined = quarantined_files(files);
    let files_count = format_count(view.totals.files);
    let history = view.history.clone();
    let by_app = view.by_app.clone();
    let banner_view = view.clone();

    view! {
        <Flex vertical=true gap=SpacingSize::Size240.flex_gap()>
            <Banners view=banner_view actions=actions site=false />
            <AutoGrid min="200px">
                <div data-testid="storage-stat-used">
                    <StatCard label="Used" value=used />
                </div>
                <div data-testid="storage-stat-cost">
                    <StatCard label=cost_label value=cost_value />
                </div>
                <div data-testid="storage-stat-files">
                    <StatCard label="Files" value=files_count />
                </div>
                <div data-testid="storage-stat-change">
                    {match change_variant {
                        Some(variant) => view! {
                            <StatCard label="Change in 30 days" value=change_value variant=variant />
                        }
                        .into_any(),
                        None => view! { <StatCard label="Change in 30 days" value=change_value /> }
                            .into_any(),
                    }}
                </div>
            </AutoGrid>
            <TwoColumns>
                <MainColumn slot>
                    <HistoryCard history=history change=change on_retry=actions.refresh />
                    <ByAppCard rows=by_app show_cost=show_cost />
                    <BiggestFilesCard biggest=biggest quarantined=quarantined />
                </MainColumn>
                <SideColumn slot>
                    <HowCard view=view actions=actions site=false />
                </SideColumn>
            </TwoColumns>
        </Flex>
    }
    .into_any()
}

/// Main column slot of [`TwoColumns`].
#[slot]
struct MainColumn {
    children: Children,
}

/// Side column slot of [`TwoColumns`].
#[slot]
struct SideColumn {
    children: Children,
}

/// Two columns about 2:1 that stack into one below about 800px.
#[component]
fn TwoColumns(main_column: MainColumn, side_column: SideColumn) -> impl IntoView {
    view! {
        <Flex wrap=FlexWrap::Wrap gap=SpacingSize::Size160.flex_gap() align=uf_product::primitives::FlexAlign::Start>
            <FlexItem grow=2.0 basis="480px" min_width="0">
                <Flex vertical=true gap=SpacingSize::Size160.flex_gap()>
                    {(main_column.children)()}
                </Flex>
            </FlexItem>
            <FlexItem grow=1.0 basis="280px" min_width="0">
                <Flex vertical=true gap=SpacingSize::Size160.flex_gap()>
                    {(side_column.children)()}
                </Flex>
            </FlexItem>
        </Flex>
    }
}

/// Skeletons of the stats, the chart and the by-app table.
#[component]
fn StorageSkeleton() -> impl IntoView {
    let stat = |label: &'static str| {
        view! {
            <Card>
                <CardContent>
                    <Flex vertical=true gap=SpacingSize::Size80.flex_gap()>
                        <Caption1>{label}</Caption1>
                        <SkeletonItem width="60%" height="28px" />
                    </Flex>
                </CardContent>
            </Card>
        }
    };
    view! {
        <div data-testid="storage-loading">
            <Flex vertical=true gap=SpacingSize::Size240.flex_gap()>
                <AutoGrid min="200px">
                    {stat("Used")}
                    {stat("Estimated cost a month")}
                    {stat("Files")}
                    {stat("Change in 30 days")}
                </AutoGrid>
                <Card>
                    <CardHeader>
                        <Subtitle1>"Last 30 days"</Subtitle1>
                    </CardHeader>
                    <CardContent>
                        <SkeletonItem width="100%" height="128px" />
                    </CardContent>
                </Card>
                <Card>
                    <CardHeader>
                        <Subtitle1>"By app"</Subtitle1>
                    </CardHeader>
                    <CardContent>
                        <Table>
                            <TableHeader>
                                <TableRow>
                                    <TableHeaderCell>"App"</TableHeaderCell>
                                    <TableHeaderCell>"Files"</TableHeaderCell>
                                    <TableHeaderCell>"Size"</TableHeaderCell>
                                    <TableHeaderCell>"Share"</TableHeaderCell>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                {(0..3)
                                    .map(|_| view! {
                                        <TableRow>
                                            <TableCell><SkeletonItem /></TableCell>
                                            <TableCell><SkeletonItem /></TableCell>
                                            <TableCell><SkeletonItem /></TableCell>
                                            <TableCell><SkeletonItem /></TableCell>
                                        </TableRow>
                                    })
                                    .collect_view()}
                            </TableBody>
                        </Table>
                    </CardContent>
                </Card>
            </Flex>
        </div>
    }
}

/// A time in the viewer's local zone, with UTC on hover.
///
/// Stands in for an Orbital `Timestamp`, which Orbital doesn't have yet.
#[component]
pub fn StorageTime(at: DateTime<Utc>) -> impl IntoView {
    let iso = at.to_rfc3339_opts(SecondsFormat::Secs, true);
    let local = at
        .with_timezone(&chrono::Local)
        .format("%b %-d, %Y, %-I:%M %p")
        .to_string();
    let utc = at.format("%b %-d, %Y, %H:%M UTC").to_string();
    view! {
        <Caption1>
            <time datetime=iso title=utc>{local}</time>
        </Caption1>
    }
}

/// The 30-day chart, or why there isn't one.
#[component]
fn HistoryCard(
    history: HistoryState,
    change: Option<i64>,
    on_retry: Callback<()>,
) -> impl IntoView {
    let body = match history {
        HistoryState::Points(points) => {
            let caption = change.map(|d| match d {
                d if d > 0 => format!("Up {} in 30 days.", format_bytes(d)),
                d if d < 0 => format!("Down {} in 30 days.", format_bytes(-d)),
                _ => "No change in 30 days.".to_string(),
            });
            view! {
                <Flex vertical=true gap=SpacingSize::Size120.flex_gap()>
                    {history_chart(&points)}
                    {caption.map(|c| view! { <Caption1>{c}</Caption1> })}
                </Flex>
            }
            .into_any()
        }
        HistoryState::NoneYet => view! {
            <div data-testid="storage-history-none-yet">
                <EmptyState
                    message="History starts after the first daily run"
                    description="Meson measures storage once a day. The first point shows up tomorrow morning."
                    icon=icondata::AiBarChartOutlined
                />
            </div>
        }
        .into_any(),
        HistoryState::Down => view! {
            <div data-testid="storage-history-down">
                <MessageBar intent=MessageBarIntent::Warning layout=MessageBarLayout::Multiline>
                    <MessageBarBody>
                        <MessageBarTitle>"History isn't available right now"</MessageBarTitle>
                        "The totals above are a live estimate. The daily history comes back when the metrics store answers again."
                    </MessageBarBody>
                    <MessageBarActions>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            on_click=Callback::new(move |_| on_retry.run(()))
                        >
                            "Try again"
                        </Button>
                    </MessageBarActions>
                </MessageBar>
            </div>
        }
        .into_any(),
    };
    view! {
        <Card>
            <CardHeader>
                <Subtitle1>"Last 30 days"</Subtitle1>
            </CardHeader>
            <CardContent>{body}</CardContent>
        </Card>
    }
}

#[allow(clippy::cast_precision_loss)]
fn history_chart(points: &[(NaiveDate, i64)]) -> impl IntoView {
    let categories: Vec<String> = points
        .iter()
        .map(|(d, _)| d.format("%Y-%m-%d").to_string())
        .collect();
    let ticks: Vec<String> = points
        .iter()
        .map(|(d, _)| d.format("%b %-d").to_string())
        .collect();
    let gigabytes: Vec<f64> = points.iter().map(|(_, b)| *b as f64 / 1e9).collect();
    let x_axis = vec![AxisDef {
        id: "day".to_string(),
        scale_type: ScaleType::Band,
        data: Some(categories),
        tick_labels: Some(ticks),
        position: AxisPosition::Bottom,
        ..Default::default()
    }];
    let series = vec![SeriesDef {
        id: "total".to_string(),
        label: Some("Total".to_string()),
        data: Some(gigabytes),
        area: Some(true),
        value_formatter: Some(Callback::new(|(gb,): (f64,)| format!("{gb:.1} GB"))),
        ..Default::default()
    }];
    view! {
        <div data-testid="storage-history-chart" data-points=points.len()>
            <AreaChart
                x_axis=x_axis
                series=series
                responsive=true
                height=200.0
            />
        </div>
    }
}

/// Per-app table with a share bar; the cost column goes when there's no price.
#[component]
fn ByAppCard(rows: Vec<AppRow>, show_cost: bool) -> impl IntoView {
    view! {
        <Card>
            <CardHeader>
                <Subtitle1>"By app"</Subtitle1>
            </CardHeader>
            <CardContent>
                <ByAppTable rows=rows show_cost=show_cost />
            </CardContent>
        </Card>
    }
}

/// The by-app table, shared by both tabs.
#[component]
fn ByAppTable(rows: Vec<AppRow>, show_cost: bool) -> impl IntoView {
    view! {
        <div data-testid="storage-by-app">
            <Table>
                <TableHeader>
                    <TableRow>
                        <TableHeaderCell>"App"</TableHeaderCell>
                        <TableHeaderCell>"Files"</TableHeaderCell>
                        <TableHeaderCell>"Size"</TableHeaderCell>
                        <TableHeaderCell>"Share"</TableHeaderCell>
                        {show_cost.then(|| view! { <TableHeaderCell>"Cost a month"</TableHeaderCell> })}
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {rows
                        .into_iter()
                        .map(|row| {
                            let pct = format!("{:.0}%", row.share * 100.0);
                            let cost = row.cost_per_month.map(|c| format_cost(c, ""));
                            view! {
                                <TableRow>
                                    <TableCell>{row.app_name}</TableCell>
                                    <TableCell>{format_count(row.files)}</TableCell>
                                    <TableCell>{format_bytes(row.bytes)}</TableCell>
                                    <TableCell>
                                        <Flex vertical=true gap=SpacingSize::Size40.flex_gap()>
                                            <Caption1>{pct}</Caption1>
                                            <ProgressBar value=row.share />
                                        </Flex>
                                    </TableCell>
                                    {show_cost.then(|| view! { <TableCell>{cost.unwrap_or_default()}</TableCell> })}
                                </TableRow>
                            }
                        })
                        .collect_view()}
                </TableBody>
            </Table>
        </div>
    }
}

/// The member's biggest files and any quarantined ones still taking space.
#[component]
fn BiggestFilesCard(biggest: Vec<MyFileRow>, quarantined: Vec<MyFileRow>) -> impl IntoView {
    let warning = quarantined_warning(&quarantined);
    view! {
        <Card>
            <CardHeader>
                <Subtitle1>"Biggest files"</Subtitle1>
                <CardHeaderAction slot>
                    <Link href="/meson">"See all in My Files"</Link>
                </CardHeaderAction>
            </CardHeader>
            <CardContent>
                <Flex vertical=true gap=SpacingSize::Size160.flex_gap()>
                    <div data-testid="storage-biggest-files">
                        <Table>
                            <TableHeader>
                                <TableRow>
                                    <TableHeaderCell>"File"</TableHeaderCell>
                                    <TableHeaderCell>"App"</TableHeaderCell>
                                    <TableHeaderCell>"Size"</TableHeaderCell>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                {biggest
                                    .into_iter()
                                    .map(|f| {
                                        let href = file_href(&f.id);
                                        view! {
                                            <TableRow>
                                                <TableCell>
                                                    <Link href=href>{f.file_name}</Link>
                                                </TableCell>
                                                <TableCell>{format::app_name(&f.source_table)}</TableCell>
                                                <TableCell>{format_bytes(f.size_bytes)}</TableCell>
                                            </TableRow>
                                        }
                                    })
                                    .collect_view()}
                            </TableBody>
                        </Table>
                    </div>
                    {warning}
                </Flex>
            </CardContent>
        </Card>
    }
}

fn quarantined_warning(quarantined: &[MyFileRow]) -> Option<AnyView> {
    let first = quarantined.first()?;
    let count = i64::try_from(quarantined.len()).unwrap_or(i64::MAX);
    let bytes: i64 = quarantined.iter().map(|f| f.size_bytes.max(0)).sum();
    let (title, body, href) = if count == 1 {
        (
            "1 quarantined file still takes space".to_string(),
            format!(
                "{} ({}) is locked by the virus scan. Nobody can open it, so deleting it loses nothing.",
                first.file_name,
                format_bytes(bytes)
            ),
            file_href(&first.id),
        )
    } else {
        (
            format!("{count} quarantined files still take space"),
            format!(
                "{} ({}) are locked by the virus scan. Nobody can open them, so deleting them loses nothing.",
                files_text(count),
                format_bytes(bytes)
            ),
            "/meson".to_string(),
        )
    };
    Some(
        view! {
            <div data-testid="storage-quarantined">
                <MessageBar intent=MessageBarIntent::Warning layout=MessageBarLayout::Multiline>
                    <MessageBarBody>
                        <MessageBarTitle>{title}</MessageBarTitle>
                        {body}
                    </MessageBarBody>
                    <MessageBarActions>
                        <Link href=href>
                            <Button appearance=ButtonAppearance::Secondary>"Review it"</Button>
                        </Link>
                    </MessageBarActions>
                </MessageBar>
            </div>
        }
        .into_any(),
    )
}

/// One labelled line of the "How this is worked out" card.
#[component]
fn HowRow(label: &'static str, children: Children) -> impl IntoView {
    view! {
        <Flex vertical=true gap=SpacingSize::Size20.flex_gap()>
            <Caption1Strong block=true>{label}</Caption1Strong>
            <Body1 block=true>{children()}</Body1>
        </Flex>
    }
}

/// Where each input came from, the arithmetic, and that it's an estimate.
#[component]
fn HowCard(view: StorageView, actions: PageActions, site: bool) -> impl IntoView {
    let size_line = match view.size {
        SizeSource::Measured { measured_at, .. } => view! {
            "Measured " <StorageTime at=measured_at /> " by the daily storage run."
        }
        .into_any(),
        SizeSource::Stale { measured_at, .. } => view! {
            "Last measured " <StorageTime at=measured_at /> ". The daily run has failed since then."
        }
        .into_any(),
        SizeSource::Live => {
            "Added up just now from the sizes recorded at upload. The daily storage run hasn't measured it yet."
                .into_any()
        }
        SizeSource::Unavailable => "Not available right now.".into_any(),
    };
    let copies_line = match view.copies {
        CopiesSource::Reported(n) => format!("{n}, as the storage service reports."),
        CopiesSource::Set(n) => format!("{n}, set by a site owner."),
        CopiesSource::Assumed => "Counted as 1. The storage service doesn't say how many copies it keeps, and no site owner has set it.".to_string(),
    };
    let price_line = match price_of(&view.price) {
        Some((per_gb, currency)) => format!(
            "{} per GB each month, set by a site owner.",
            format_price(per_gb, currency)
        ),
        None => "Not set. A site owner hasn't added a storage price, so Meson shows size only."
            .to_string(),
    };
    let arithmetic = worked_out(view.totals.bytes, view.copies.count(), &view.price);
    let header = if site && actions.owner {
        view! {
            <CardHeader>
                <Subtitle1>"How this is worked out"</Subtitle1>
                <CardHeaderAction slot>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        attr:data-testid="storage-change-settings"
                        on_click=Callback::new(move |_| actions.open_settings.run(()))
                    >
                        "Change settings"
                    </Button>
                </CardHeaderAction>
            </CardHeader>
        }
        .into_any()
    } else {
        view! {
            <CardHeader>
                <Subtitle1>"How this is worked out"</Subtitle1>
            </CardHeader>
        }
        .into_any()
    };
    view! {
        <Card>
            {header}
            <CardContent>
                <div data-testid="storage-how">
                    <Flex vertical=true gap=SpacingSize::Size160.flex_gap()>
                        <HowRow label="Size">{size_line}</HowRow>
                        <HowRow label="Copies kept">{copies_line}</HowRow>
                        <HowRow label="Price">{price_line}</HowRow>
                        <HowRow label="Worked out">
                            <span data-testid="storage-worked-out">{arithmetic}</span>
                        </HowRow>
                        <Caption1>
                            "An estimate, not a bill. A provider also charges for requests and data sent out, which this leaves out."
                        </Caption1>
                    </Flex>
                </div>
            </CardContent>
        </Card>
    }
}
