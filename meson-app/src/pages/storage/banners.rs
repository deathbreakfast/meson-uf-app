//! One banner per missing or old input. Site owners get the action that fixes
//! it; members get the sentence only.

use leptos::prelude::*;
use uf_product::primitives::{
    Button, ButtonAppearance, Flex, MessageBar, MessageBarActions, MessageBarBody,
    MessageBarIntent, MessageBarLayout, MessageBarTitle,
};

use super::format::{days_old, days_text, files_text};
use super::{PageActions, StorageTime};
use crate::storage::view::{CopiesSource, HistoryState, PriceSource, SizeSource, StorageView};

/// The banners for `view`. `site` is true on the Everyone tab, which draws
/// its own banner for a failing run.
#[component]
pub(super) fn Banners(view: StorageView, actions: PageActions, site: bool) -> impl IntoView {
    let owner = actions.owner;
    let mut bars: Vec<AnyView> = Vec::new();

    match view.size {
        SizeSource::Live if view.history != HistoryState::Down => bars.push(
            view! {
                <Banner
                    test_id="storage-banner-live"
                    intent=MessageBarIntent::Info
                    title="Live estimate".to_string()
                >
                    "Meson hasn't run its daily storage measurement yet, so this adds up the sizes recorded when each file was uploaded. History starts after the first run."
                </Banner>
            }
            .into_any(),
        ),
        SizeSource::Stale { measured_at, .. } if !site => {
            let action = owner.then_some(("See the failed run", actions.show_everyone));
            let title = format!(
                "These numbers are {} old",
                days_text(days_old(measured_at, chrono::Utc::now()))
            );
            bars.push(
                view! {
                    <Banner
                        test_id="storage-banner-stale"
                        intent=MessageBarIntent::Warning
                        title=title
                        action=action
                    >
                        "These numbers are from " <StorageTime at=measured_at />
                        "; files added since aren't counted. Site owners have been told."
                    </Banner>
                }
                .into_any(),
            );
        }
        SizeSource::Unavailable if site => bars.push(
            view! {
                <Banner
                    test_id="storage-banner-unavailable"
                    intent=MessageBarIntent::Info
                    title="No site totals yet".to_string()
                >
                    "Meson hasn't run its daily storage measurement on this site yet, so there's no total for everyone. Your storage still shows your own files."
                </Banner>
            }
            .into_any(),
        ),
        SizeSource::Unavailable => bars.push(
            view! {
                <Banner
                    test_id="storage-banner-unavailable"
                    intent=MessageBarIntent::Warning
                    title="Sizes aren't available right now".to_string()
                    action=Some(("Try again", actions.refresh))
                >
                    "Meson couldn't read the daily storage numbers or add up your files. Nothing was changed."
                </Banner>
            }
            .into_any(),
        ),
        _ => {}
    }

    let price_set = view.price != PriceSource::Unset;
    if view.copies == CopiesSource::Assumed && price_set {
        let action = owner.then_some(("Set copies kept", actions.open_settings));
        bars.push(
            view! {
                <Banner
                    test_id="storage-banner-copies"
                    intent=MessageBarIntent::Info
                    title="Cost may be higher".to_string()
                    action=action
                >
                    "The storage service doesn't say how many copies it keeps, so this counts one. If it keeps three, the cost is three times this."
                </Banner>
            }
            .into_any(),
        );
    }
    if !price_set {
        let action = owner.then_some(("Set a price", actions.open_settings));
        bars.push(
            view! {
                <Banner
                    test_id="storage-banner-size-only"
                    intent=MessageBarIntent::Info
                    title="Size only".to_string()
                    action=action
                >
                    "A site owner hasn't set a storage price, so there's no cost estimate."
                </Banner>
            }
            .into_any(),
        );
    }
    if view.missing_size_files > 0 {
        let n = view.missing_size_files;
        let verb = if n == 1 { "has" } else { "have" };
        let body = if view.size == SizeSource::Live {
            format!(
                "{} {verb} no size recorded, so they count as 0.",
                files_text(n)
            )
        } else {
            format!(
                "{} {verb} no size recorded, and storage didn't answer when the daily run tried to measure them. They count as 0 until the next run.",
                files_text(n)
            )
        };
        bars.push(
            view! {
                <Banner
                    test_id="storage-banner-missing"
                    intent=MessageBarIntent::Warning
                    title="The total may be low".to_string()
                >
                    {body}
                </Banner>
            }
            .into_any(),
        );
    }

    (!bars.is_empty()).then(|| {
        view! {
            <Flex vertical=true gap=uf_product::components::SpacingSize::Size120.flex_gap()>
                {bars}
            </Flex>
        }
    })
}

/// One multiline banner with an optional action button.
#[component]
pub(super) fn Banner(
    test_id: &'static str,
    intent: MessageBarIntent,
    title: String,
    #[prop(default = None)] action: Option<(&'static str, Callback<()>)>,
    children: Children,
) -> impl IntoView {
    view! {
        <div data-testid=test_id>
            <MessageBar intent=intent layout=MessageBarLayout::Multiline>
                <MessageBarBody>
                    <MessageBarTitle>{title}</MessageBarTitle>
                    {children()}
                </MessageBarBody>
                {action.map(|(label, on_click)| view! {
                    <MessageBarActions>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            on_click=Callback::new(move |_| on_click.run(()))
                        >
                            {label}
                        </Button>
                    </MessageBarActions>
                })}
            </MessageBar>
        </div>
    }
}
