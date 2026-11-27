//! Storage page at `/meson/storage`: how much space the signed-in member's
//! files use and roughly what that costs, plus the site-wide Everyone tab and
//! the cost settings for site owners.
//!
//! Every input to the estimate can be missing (no daily run yet, no copy
//! count, no price, no history). The page shows what it knows, says what's
//! missing in a banner, and never turns a missing input into an error.

mod banners;
mod cards;
mod everyone;
pub mod format;
pub mod run;
mod settings;

use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::{use_navigate, use_query_map};
use leptos_router::NavigateOptions;
use uf_product::components::{
    AutoGrid, Body1, ContentContainer, EmptyState, SpacingSize, StatCard, StatCardVariant, Title2,
};
use uf_product::primitives::{Flex, Tab, TabList};

use crate::components::ErrorBar;
use crate::server::{get_my_storage, get_site_storage, list_my_files, MyFileRow};
use crate::storage::view::{HistoryState, PriceSource, SizeSource, StorageView};

use banners::Banners;
pub use cards::StorageTime;
use cards::{
    BiggestFilesCard, ByAppCard, HistoryCard, HowCard, MainColumn, SideColumn, StorageSkeleton,
    TwoColumns,
};
use everyone::{EveryoneDenied, EveryoneTab};
use format::{
    biggest_files, change_over_history, cost_stat, format_bytes, format_change, format_count,
    quarantined_files, with_live_sizes,
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
