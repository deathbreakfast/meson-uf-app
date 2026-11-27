//! Cards and layout shared by the Your storage and Everyone tabs.

use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use leptos::prelude::*;
use uf_product::components::{
    AutoGrid, Body1, Caption1, Caption1Strong, EmptyState, FlexItem, SpacingSize, Subtitle1,
};
use uf_product::primitives::{
    AreaChart, AxisDef, AxisPosition, Button, ButtonAppearance, Card, CardContent, CardHeader,
    CardHeaderAction, Flex, FlexWrap, Link, MessageBar, MessageBarActions, MessageBarBody,
    MessageBarIntent, MessageBarLayout, MessageBarTitle, ProgressBar, ScaleType, SeriesDef,
    SkeletonItem, Table, TableBody, TableCell, TableHeader, TableHeaderCell, TableRow,
};

use super::format::{
    self, file_href, files_text, format_bytes, format_cost, format_count, format_price, price_of,
    worked_out,
};
use super::PageActions;
use crate::server::MyFileRow;
use crate::storage::view::{AppRow, CopiesSource, HistoryState, SizeSource, StorageView};

/// Main column slot of [`TwoColumns`].
#[slot]
pub(super) struct MainColumn {
    children: Children,
}

/// Side column slot of [`TwoColumns`].
#[slot]
pub(super) struct SideColumn {
    children: Children,
}

/// Two columns about 2:1 that stack into one below about 800px.
#[component]
pub(super) fn TwoColumns(main_column: MainColumn, side_column: SideColumn) -> impl IntoView {
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
pub(super) fn StorageSkeleton() -> impl IntoView {
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
pub(super) fn HistoryCard(
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
pub(super) fn ByAppCard(rows: Vec<AppRow>, show_cost: bool) -> impl IntoView {
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
pub(super) fn BiggestFilesCard(
    biggest: Vec<MyFileRow>,
    quarantined: Vec<MyFileRow>,
) -> impl IntoView {
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
pub(super) fn HowCard(view: StorageView, actions: PageActions, site: bool) -> impl IntoView {
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
