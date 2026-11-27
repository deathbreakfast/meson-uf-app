//! Storage cost settings dialog: copies kept and the price per GB each month.

use leptos::prelude::*;
use leptos::task::spawn_local;
use uf_product::components::{Caption1, FormHint, SpacingSize};
use uf_product::primitives::{
    Button, ButtonAppearance, Dialog, DialogActions, DialogBody, DialogContent, DialogSurface,
    DialogTitle, Field, FieldValidationState, Flex, Input, InputAppearance, InputBind, InputRule,
    InputRuleTrigger, InputSuffix, InputType, MessageBar, MessageBarBody, MessageBarIntent, Radio,
    RadioGroup, RadioGroupBind,
};

use super::format::{copies_text, parse_copies, parse_price, price_of, worked_out};
use crate::server::save_storage_settings;
use crate::storage::settings::DEFAULT_CURRENCY;
use crate::storage::view::{CopiesSource, SiteStorageView};

const MODE_REPORT: &str = "report";
const MODE_SET: &str = "set";

/// The dialog. Reads the site view for its starting values and preview.
#[component]
pub(super) fn SettingsDialog(
    open: RwSignal<bool>,
    site_res: Resource<Result<Option<SiteStorageView>, ServerFnError>>,
    on_saved: Callback<()>,
) -> impl IntoView {
    let mode = RwSignal::new(Some(MODE_REPORT.to_string()));
    let copies_input = RwSignal::new(String::new());
    let price_input = RwSignal::new(String::new());
    let saving = RwSignal::new(false);
    let failed = RwSignal::new(false);

    let site = move || site_res.get_untracked().and_then(Result::ok).flatten();

    // Start from the saved values each time the dialog opens.
    Effect::new(move |_| {
        if !open.get() {
            return;
        }
        failed.set(false);
        let Some(site) = site() else {
            return;
        };
        match site.view.copies {
            CopiesSource::Set(n) => {
                mode.set(Some(MODE_SET.to_string()));
                copies_input.set(n.to_string());
            }
            _ => {
                mode.set(Some(MODE_REPORT.to_string()));
                copies_input.set(site.view.copies_reported.unwrap_or(1).to_string());
            }
        }
        price_input.set(
            price_of(&site.view.price)
                .map(|(p, _)| format!("{p}"))
                .unwrap_or_default(),
        );
    });

    let setting_copies = move || mode.get().as_deref() == Some(MODE_SET);
    let copies = Memo::new(move |_| {
        if setting_copies() {
            parse_copies(&copies_input.get()).ok().map(Some)
        } else {
            Some(None)
        }
    });
    let price = Memo::new(move |_| parse_price(&price_input.get()).ok());
    let valid = move || copies.get().is_some() && price.get().is_some();

    let reported = move || {
        site_res
            .get()
            .and_then(Result::ok)
            .flatten()
            .and_then(|s| s.view.copies_reported)
    };
    let report_label = move || match reported() {
        Some(n) => format!("Use what the storage service reports ({})", copies_text(n)),
        None => "Use what the storage service reports (it doesn't say)".to_string(),
    };
    let currency = move || {
        site_res
            .get()
            .and_then(Result::ok)
            .flatten()
            .and_then(|s| price_of(&s.view.price).map(|(_, c)| c.to_string()))
            .unwrap_or_else(|| DEFAULT_CURRENCY.to_string())
    };

    let preview = move || {
        let site = site_res.get().and_then(Result::ok).flatten()?;
        let copies_override = copies.get()?;
        let per_gb = price.get()?;
        let n = copies_override.unwrap_or_else(|| reported().unwrap_or(1));
        let price = per_gb.map_or(crate::storage::view::PriceSource::Unset, |p| {
            crate::storage::view::PriceSource::Set {
                per_gb_month: p,
                currency: currency(),
            }
        });
        Some(format!(
            "Preview: {} for the whole site.",
            worked_out(site.view.totals.bytes, n, &price)
        ))
    };

    let save = move |_| {
        let (Some(copies_override), Some(per_gb)) = (copies.get_untracked(), price.get_untracked())
        else {
            return;
        };
        saving.set(true);
        failed.set(false);
        spawn_local(async move {
            let result = save_storage_settings(copies_override, per_gb).await;
            saving.set(false);
            if result.is_ok() {
                open.set(false);
                on_saved.run(());
            } else {
                failed.set(true);
            }
        });
    };

    let price_rule = InputRule::validator(|value, _| {
        parse_price(value)
            .map(|_| ())
            .map_err(|hint| FieldValidationState::Error(hint.to_string()))
    })
    .with_trigger(InputRuleTrigger::Input);

    view! {
        <Dialog open=open>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>"Storage cost settings"</DialogTitle>
                    <DialogContent>
                        <div data-testid="storage-settings-dialog">
                            <Flex vertical=true gap=SpacingSize::Size160.flex_gap()>
                                <Field label="Copies kept">
                                    <div data-testid="storage-settings-copies-mode">
                                        <RadioGroup bind=RadioGroupBind::new(mode)>
                                            <Radio value=MODE_REPORT label=Signal::derive(report_label) />
                                            <Radio value=MODE_SET label="Set it myself" />
                                        </RadioGroup>
                                    </div>
                                </Field>
                                <Show when=setting_copies>
                                    <Field label="Copies">
                                        <div data-testid="storage-settings-copies">
                                            <Input
                                                bind=InputBind {
                                                    rules: vec![InputRule::validator(|value, _| {
                                                        parse_copies(value)
                                                            .map(|_| ())
                                                            .map_err(|hint| FieldValidationState::Error(hint.to_string()))
                                                    })
                                                    .with_trigger(InputRuleTrigger::Input)],
                                                    ..InputBind::new(copies_input)
                                                }
                                                appearance=InputAppearance {
                                                    input_type: InputType::Number.into(),
                                                    ..Default::default()
                                                }
                                            >
                                                <InputSuffix slot>"copies"</InputSuffix>
                                            </Input>
                                        </div>
                                    </Field>
                                </Show>
                                <FormHint>
                                    "Count every full copy: replicas, plus a backup if you pay for it."
                                </FormHint>
                                <Field label="Price per GB each month">
                                    <div data-testid="storage-settings-price">
                                        <Input
                                            bind=InputBind {
                                                rules: vec![price_rule],
                                                ..InputBind::new(price_input)
                                            }
                                        >
                                            <InputSuffix slot>{currency}</InputSuffix>
                                        </Input>
                                    </div>
                                </Field>
                                <FormHint>
                                    "From your storage provider's price list. Leave it empty to show sizes without a cost."
                                </FormHint>
                                <span data-testid="storage-settings-preview">
                                    <Caption1>{preview}</Caption1>
                                </span>
                                <Caption1>
                                    "Applies to everyone's Storage page from the next refresh."
                                </Caption1>
                                {move || failed.get().then(|| view! {
                                    <MessageBar intent=MessageBarIntent::Error>
                                        <MessageBarBody>
                                            "Couldn't save the settings. Nothing was changed."
                                        </MessageBarBody>
                                    </MessageBar>
                                })}
                            </Flex>
                        </div>
                    </DialogContent>
                    <DialogActions>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            attr:data-testid="storage-settings-cancel"
                            on_click=Callback::new(move |_| open.set(false))
                        >
                            "Cancel"
                        </Button>
                        <Button
                            appearance=ButtonAppearance::Primary
                            attr:data-testid="storage-settings-save"
                            disabled=Signal::derive(move || !valid() || saving.get())
                            loading=saving
                            on_click=Callback::new(save)
                        >
                            "Save"
                        </Button>
                    </DialogActions>
                </DialogBody>
            </DialogSurface>
        </Dialog>
    }
}
