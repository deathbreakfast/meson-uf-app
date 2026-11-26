//! Confirm-and-delete dialog for one or more of the viewer's files.
//!
//! Says what the app that stored each file loses, how much space it frees and
//! that it can't be undone, adds a note for files the virus scan flagged, and
//! shows the app's reason when it refuses. The file page uses it for one file;
//! bulk actions pass several.

use leptos::prelude::*;
use leptos::task::spawn_local;
use uf_product::components::{Body1, Caption1, SpacingSize};
use uf_product::primitives::{
    Button, ButtonAppearance, Dialog, DialogActions, DialogBody, DialogContent, DialogSurface,
    DialogTitle, Flex, MessageBar, MessageBarBody, MessageBarIntent, MessageBarLayout,
    MessageBarTitle,
};

use crate::format::size_label;
use crate::server::{delete_my_file, DeleteResult};

/// One file to delete.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeleteTarget {
    /// Full `table:id`.
    pub id: String,
    /// Name shown in the dialog.
    pub file_name: String,
    /// App that stored it ("Receipt scanner").
    pub app_name: String,
    /// Size freed.
    pub size_bytes: i64,
    /// The virus scan flagged it.
    pub quarantined: bool,
}

/// What the app refused, per file.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Refusal {
    file_name: String,
    app_name: String,
    reason: String,
}

/// The dialog. `open` is owned by the caller; `on_deleted` runs once every
/// file is gone (the dialog closes itself first).
#[component]
pub fn DeleteDialog(
    /// Open state.
    open: RwSignal<bool>,
    /// Files to delete.
    #[prop(into)]
    files: Signal<Vec<DeleteTarget>>,
    /// Called after every file was deleted.
    on_deleted: Callback<()>,
) -> impl IntoView {
    let busy = RwSignal::new(false);
    let refusals = RwSignal::new(Vec::<Refusal>::new());
    let failed = RwSignal::new(false);

    // A fresh open starts clean.
    Effect::new(move |_| {
        if open.get() {
            refusals.set(Vec::new());
            failed.set(false);
        }
    });

    let title = move || {
        let files = files.get();
        match files.as_slice() {
            [one] => format!("Delete {}?", one.file_name),
            many => format!("Delete {} files?", many.len()),
        }
    };
    let confirm_label = move || {
        let files = files.get();
        match files.as_slice() {
            [one] => format!("Delete {}", one.file_name),
            many => format!("Delete {} files", many.len()),
        }
    };
    let body = move || {
        let files = files.get();
        let freed: i64 = files.iter().map(|f| f.size_bytes).sum();
        let loses = match files.as_slice() {
            [one] => format!("{} will no longer have this file.", one.app_name),
            _ => "The apps that stored them will no longer have them.".to_string(),
        };
        format!(
            "{loses} This frees {} and can't be undone.",
            size_label(freed)
        )
    };
    let any_quarantined = move || files.with(|f| f.iter().any(|f| f.quarantined));

    let confirm = Callback::new(move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        refusals.set(Vec::new());
        failed.set(false);
        let targets = files.get_untracked();
        spawn_local(async move {
            let mut refused = Vec::new();
            let mut error = false;
            for t in targets {
                match delete_my_file(t.id.clone()).await {
                    Ok(DeleteResult::Deleted { .. }) => {}
                    Ok(DeleteResult::Refused { reason }) => refused.push(Refusal {
                        file_name: t.file_name,
                        app_name: t.app_name,
                        reason,
                    }),
                    Err(_) => error = true,
                }
            }
            busy.set(false);
            if refused.is_empty() && !error {
                open.set(false);
                on_deleted.run(());
            } else {
                refusals.set(refused);
                failed.set(error);
            }
        });
    });

    view! {
        <Dialog open=open>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>{title}</DialogTitle>
                    <DialogContent>
                        <div data-testid="meson-delete-dialog">
                            <Flex vertical=true gap=SpacingSize::Size160.flex_gap()>
                                <Body1>{body}</Body1>
                                <Show when=any_quarantined>
                                    <Caption1>
                                        "The virus scan flagged this file. Deleting it removes the locked copy from storage."
                                    </Caption1>
                                </Show>
                                <For
                                    each=move || refusals.get()
                                    key=|r| r.file_name.clone()
                                    let:r
                                >
                                    <div data-testid="meson-delete-refused">
                                        <MessageBar intent=MessageBarIntent::Warning layout=MessageBarLayout::Multiline>
                                            <MessageBarTitle>{format!("{} kept {}", r.app_name, r.file_name)}</MessageBarTitle>
                                            <MessageBarBody>{format!("{} Nothing was deleted.", r.reason)}</MessageBarBody>
                                        </MessageBar>
                                    </div>
                                </For>
                                <Show when=move || failed.get()>
                                    <div data-testid="meson-delete-failed">
                                        <MessageBar intent=MessageBarIntent::Error layout=MessageBarLayout::Multiline>
                                            <MessageBarTitle>"Couldn't delete"</MessageBarTitle>
                                            <MessageBarBody>
                                                "Meson didn't finish the delete, so the file is still there. Try again."
                                            </MessageBarBody>
                                        </MessageBar>
                                    </div>
                                </Show>
                            </Flex>
                        </div>
                    </DialogContent>
                    <DialogActions>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            attr:data-testid="meson-delete-cancel"
                            on_click=Callback::new(move |_| open.set(false))
                        >
                            {move || if refusals.with(Vec::is_empty) { "Cancel" } else { "Close" }}
                        </Button>
                        <Show when=move || refusals.with(Vec::is_empty)>
                            <Button
                                appearance=ButtonAppearance::Danger
                                attr:data-testid="meson-delete-confirm"
                                loading=busy
                                on_click=confirm
                            >
                                {confirm_label}
                            </Button>
                        </Show>
                    </DialogActions>
                </DialogBody>
            </DialogSurface>
        </Dialog>
    }
}
