//! The Everyone tab: site totals, by app, by person and the daily run, for
//! viewers holding `StorageAdmin`.

use std::collections::HashMap;
use std::sync::Arc;

use leptos::prelude::*;
use leptos::task::spawn_local;
use uf_product::components::{
    AutoGrid, Body1, Caption1, Code, EmptyState, EmptyStateCallToAction, Persona, PersonaConfig,
    PersonaSecondaryText, SpacingSize, StatCard, Subtitle1,
};
use uf_product::primitives::{
    Badge, BadgeAppearance, BadgeColor, BadgeSize, Button, ButtonAppearance, Card, CardContent,
    CardHeader, CardHeaderAction, CardHeaderDescription, DataRecord, DataTable, DataTableColumnDef,
    DataTableLocale, DataTableRowModel, DataTableToolbarConfig, DataValue, Flex, Link,
    MessageBarIntent, PagingMode,
};

use super::banners::{Banner, Banners};
use super::cards::{
    ByAppCard, HowCard, MainColumn, SideColumn, StorageSkeleton, StorageTime, TwoColumns,
};
use super::format::{
    copies_text, cost_stat, format_bytes, format_cost, format_count, is_stale, run_status,
    RunStatus,
};
use super::run::{run_storage_now, StorageRunInfo, JOB_NAME};
use super::PageActions;
use crate::components::ErrorBar;
use crate::storage::view::{PersonRow, PriceSource, SiteStorageView, SizeSource};

/// Everyone tab body.
#[component]
pub(super) fn EveryoneTab(
    site_res: Resource<Result<Option<SiteStorageView>, ServerFnError>>,
    run_res: Resource<Option<StorageRunInfo>>,
    actions: Signal<PageActions>,
) -> impl IntoView {
    view! {
        <div data-testid="storage-everyone">
            <Transition fallback=StorageSkeleton>
                {move || {
                    let run = run_res.get().flatten();
                    match site_res.get() {
                        Some(Ok(Some(site))) => everyone_view(site, run, actions.get()),
                        Some(Err(e)) => view! {
                            <ErrorBar
                                error=e
                                what="everyone's storage"
                                on_retry=Callback::new(move |()| actions.get_untracked().refresh.run(()))
                            />
                        }
                        .into_any(),
                        Some(Ok(None)) | None => view! { <StorageSkeleton /> }.into_any(),
                    }
                }}
            </Transition>
        </div>
    }
}

fn everyone_view(
    site: SiteStorageView,
    run: Option<StorageRunInfo>,
    actions: PageActions,
) -> AnyView {
    let view = site.view.clone();
    let copies = view.copies.count();
    let stored_label = format!("Stored with {}", copies_text(copies));
    let cost = cost_stat(&view).unwrap_or_else(|| "Set a price".to_string());
    let show_cost = view.price != PriceSource::Unset;
    let failing = match view.size {
        SizeSource::Stale { measured_at, .. } => Some(measured_at),
        _ => None,
    };
    let failing_bar = failing.map(|measured_at| {
        let days = super::format::days_old(measured_at, chrono::Utc::now());
        let title = format!(
            "The daily storage run has failed {} running",
            super::format::days_text(days)
        );
        let chronon = run.as_ref().map(|r| r.chronon_href.clone());
        view! {
            <Banner
                test_id="storage-banner-run-failing"
                intent=MessageBarIntent::Error
                title=title
            >
                "Everyone sees the numbers from " <StorageTime at=measured_at /> " until a run works."
                {chronon.map(|href| view! {
                    " " <Link href=href>"Open the run in Chronon"</Link>
                })}
            </Banner>
        }
    });

    let size_text = format_bytes(view.totals.bytes);
    let stored_text = format_bytes(view.totals.bytes_with_copies);
    let people_text = format_count(i64::try_from(site.people.len()).unwrap_or(i64::MAX));
    let by_app = view.by_app.clone();
    let people = site.people.clone();
    let banner_view = view.clone();

    view! {
        <Flex vertical=true gap=SpacingSize::Size240.flex_gap()>
            {failing_bar}
            <Banners view=banner_view actions=actions site=true />
            <AutoGrid min="200px">
                <div data-testid="storage-site-stat-size">
                    <StatCard label="Files on the site" value=size_text />
                </div>
                <div data-testid="storage-site-stat-stored">
                    <StatCard label=stored_label value=stored_text />
                </div>
                <div data-testid="storage-site-stat-cost">
                    <StatCard label="Estimated cost a month" value=cost />
                </div>
                <div data-testid="storage-site-stat-people">
                    <StatCard label="People with files" value=people_text />
                </div>
            </AutoGrid>
            <TwoColumns>
                <MainColumn slot>
                    <ByAppCard rows=by_app show_cost=show_cost />
                    <ByPersonCard people=people show_cost=show_cost />
                </MainColumn>
                <SideColumn slot>
                    <RunCard site=site run=run />
                    <HowCard view=view actions=actions site=true />
                </SideColumn>
            </TwoColumns>
        </Flex>
    }
    .into_any()
}

/// `user` → `Person`, `service_user` → `Service user`.
fn owner_kind_label(kind: &str) -> String {
    match kind {
        "user" => "Person".to_string(),
        other => crate::storage::view::app_display_name(other),
    }
}

#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn number_display(f: fn(f64) -> String) -> Callback<(DataValue,), String> {
    Callback::new(move |(value,): (DataValue,)| match value {
        DataValue::Number(n) => f(n),
        _ => String::new(),
    })
}

#[allow(clippy::cast_precision_loss)]
fn person_rows(people: &[PersonRow]) -> Vec<DataTableRowModel> {
    people
        .iter()
        .map(|p| {
            let mut values = HashMap::new();
            values.insert("person".to_string(), DataValue::Text(p.owner_id.clone()));
            values.insert(
                "kind".to_string(),
                DataValue::Text(owner_kind_label(&p.owner_kind)),
            );
            values.insert("files".to_string(), DataValue::Number(p.files as f64));
            values.insert("size".to_string(), DataValue::Number(p.bytes as f64));
            values.insert(
                "cost".to_string(),
                DataValue::Number(p.cost_per_month.unwrap_or_default()),
            );
            values.insert("share".to_string(), DataValue::Number(p.share));
            DataTableRowModel::new(DataRecord::new(
                format!("{}:{}", p.owner_kind, p.owner_id),
                values,
            ))
        })
        .collect()
}

/// Everyone's totals, biggest first: sortable, searchable, paged, exportable.
#[component]
fn ByPersonCard(people: Vec<PersonRow>, show_cost: bool) -> impl IntoView {
    let items = RwSignal::new(person_rows(&people));
    let person_view = Arc::new(|record: DataRecord| {
        let name = record
            .get("person")
            .map_or_else(String::new, DataValue::display_string);
        let kind = record
            .get("kind")
            .map_or_else(String::new, DataValue::display_string);
        view! {
            <Persona config=PersonaConfig::named(name)>
                <PersonaSecondaryText slot>{kind}</PersonaSecondaryText>
            </Persona>
        }
        .into_any()
    });
    let mut columns = vec![
        DataTableColumnDef::new("person", "Person").with_cell_view(person_view),
        DataTableColumnDef::new("files", "Files")
            .with_col_type(uf_product::primitives::ColumnType::Number)
            .with_format_display(number_display(|n| format_count(n as i64))),
        DataTableColumnDef::new("size", "Size")
            .with_col_type(uf_product::primitives::ColumnType::Number)
            .with_format_display(number_display(|n| format_bytes(n as i64))),
    ];
    if show_cost {
        columns.push(
            DataTableColumnDef::new("cost", "Cost a month")
                .with_col_type(uf_product::primitives::ColumnType::Number)
                .with_format_display(number_display(|n| format_cost(n, ""))),
        );
    }
    columns.push(
        DataTableColumnDef::new("share", "Share")
            .with_col_type(uf_product::primitives::ColumnType::Number)
            .with_format_display(number_display(|n| format!("{:.0}%", n * 100.0))),
    );
    let locale = DataTableLocale {
        quick_search_placeholder: "Search people and service users".to_string(),
        ..Default::default()
    };
    view! {
        <Card>
            <CardHeader>
                <Subtitle1>"By person"</Subtitle1>
                <CardHeaderDescription slot>
                    "Only site owners see this. People see their own row on Your storage."
                </CardHeaderDescription>
            </CardHeader>
            <CardContent>
                <div data-testid="storage-by-person">
                    <DataTable
                        columns=columns
                        items=items
                        paging=PagingMode::Paged
                        sortable=true
                        locale=locale
                        toolbar_config=DataTableToolbarConfig {
                            quick_search: true,
                            filter_panel: false,
                            column_picker: false,
                            pivot: false,
                            export_menu: true,
                        }
                    />
                </div>
            </CardContent>
        </Card>
    }
}

/// The `meson.storage-daily` job: status, schedule, last run, and Run now /
/// Open in Chronon when the host runs Chronon.
#[component]
fn RunCard(site: SiteStorageView, run: Option<StorageRunInfo>) -> impl IntoView {
    let status = run_status(
        site.measured_at,
        is_stale(&site.view.size),
        run.is_some(),
        chrono::Utc::now(),
    );
    let (label, color) = match status {
        RunStatus::Healthy => ("Healthy".to_string(), BadgeColor::Success),
        RunStatus::Failing(days) => (
            format!("Failed {} running", super::format::days_text(days)),
            BadgeColor::Danger,
        ),
        RunStatus::NotRunYet => ("Not run yet".to_string(), BadgeColor::Informative),
        RunStatus::NotScheduled => ("Not scheduled".to_string(), BadgeColor::Warning),
    };
    let apps = site.view.by_app.len();
    let last_run = match (status, site.measured_at) {
        (RunStatus::Healthy, Some(at)) => view! {
            <StorageTime at=at />
            {format!(
                " · {} in {}",
                super::format::files_text(site.view.totals.files),
                if apps == 1 { "1 app".to_string() } else { format!("{apps} apps") }
            )}
        }
        .into_any(),
        (RunStatus::Failing(_), Some(at)) => view! {
            "Last good run " <StorageTime at=at /> "."
        }
        .into_any(),
        (RunStatus::NotScheduled, _) => {
            "This site doesn't run Chronon, so Meson can't measure storage each day.".into_any()
        }
        _ => "It hasn't run yet.".into_any(),
    };
    let message = RwSignal::new(None::<&'static str>);
    let header = match run.clone() {
        Some(info) => view! {
            <CardHeader>
                <Subtitle1>"Daily storage run"</Subtitle1>
                <CardHeaderAction slot>
                    <Flex gap=SpacingSize::Size80.flex_gap() align=uf_product::primitives::FlexAlign::Center>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            attr:data-testid="storage-run-now"
                            on_click=Callback::new(move |_| {
                                spawn_local(async move {
                                    let text = if run_storage_now().await.is_ok() {
                                        "Started. The numbers update when the run finishes."
                                    } else {
                                        "The run didn't start. Try again, or open it in Chronon."
                                    };
                                    message.set(Some(text));
                                });
                            })
                        >
                            "Run now"
                        </Button>
                        <Link href=info.chronon_href>"Open in Chronon"</Link>
                    </Flex>
                </CardHeaderAction>
            </CardHeader>
        }
        .into_any(),
        None => view! {
            <CardHeader>
                <Subtitle1>"Daily storage run"</Subtitle1>
            </CardHeader>
        }
        .into_any(),
    };
    let scheduled = status != RunStatus::NotScheduled;
    let status_label = label.clone();
    view! {
        <Card>
            {header}
            <CardContent>
                <div data-testid="storage-run-card" data-status=status_label>
                    <Flex vertical=true gap=SpacingSize::Size160.flex_gap()>
                        <RunRow label="Job">
                            <Code text=JOB_NAME />
                        </RunRow>
                        <RunRow label="Status">
                            <Badge appearance=BadgeAppearance::Tint color=color size=BadgeSize::Small>
                                {label}
                            </Badge>
                        </RunRow>
                        {run.map(|info| view! { <RunRow label="Schedule">{info.schedule}</RunRow> })}
                        <RunRow label="Last run">{last_run}</RunRow>
                        {scheduled.then(|| view! {
                            <RunRow label="Stored in">
                                "Spectra table " <Code text="meson_storage_daily" /> ", kept 400 days"
                            </RunRow>
                        })}
                        {move || message.get().map(|m| view! { <Caption1>{m}</Caption1> })}
                    </Flex>
                </div>
            </CardContent>
        </Card>
    }
}

/// One labelled line of the run card.
#[component]
fn RunRow(label: &'static str, children: Children) -> impl IntoView {
    view! {
        <Flex vertical=true gap=SpacingSize::Size20.flex_gap()>
            <uf_product::components::Caption1Strong block=true>{label}</uf_product::components::Caption1Strong>
            <Body1 block=true>{children()}</Body1>
        </Flex>
    }
}

/// A pasted `?view=everyone` link opened without `StorageAdmin`.
#[component]
pub(super) fn EveryoneDenied(on_back: Callback<()>) -> impl IntoView {
    view! {
        <div data-testid="storage-everyone-denied">
            <EmptyState
                message="Only site owners can see everyone's storage"
                description="You can see your own on Your storage. Someone sent you a link to the Everyone view, which needs the Storage admin permission."
                icon=icondata::AiLockOutlined
            >
                <EmptyStateCallToAction slot:call_to_action>
                    <Button
                        appearance=ButtonAppearance::Primary
                        on_click=Callback::new(move |_| on_back.run(()))
                    >
                        "Go to Your storage"
                    </Button>
                </EmptyStateCallToAction>
            </EmptyState>
        </div>
    }
}
