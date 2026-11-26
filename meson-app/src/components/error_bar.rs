//! Plain-words errors for My Files server fns.
//!
//! The raw `ServerFnError` text never reaches the page: the server logs it in
//! `MesonAppError::into_server_fn`, and this module maps it to a title, a
//! sentence and an action.

use leptos::prelude::*;
use leptos_router::hooks::use_location;
use uf_product::components::{EmptyState, EmptyStateCallToAction};
use uf_product::paths::PERMISSION_PERMISSIONS;
use uf_product::primitives::{
    Button, ButtonAppearance, Link, MessageBar, MessageBarActions, MessageBarBody,
    MessageBarIntent, MessageBarLayout, MessageBarTitle,
};
use uf_product::routes::auth_signin_href;

/// Which state a server error maps to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// Invalid session or Valence build failure (`auth:`).
    SignIn,
    /// Backend failure (`io:`), or anything unrecognised.
    Load,
    /// Missing, foreign or malformed id (`not_found:`).
    NotFound,
    /// `FilesRead` withheld (`forbidden:` or the permission payload).
    Denied,
}

/// Map a server fn error to the state the page should show.
#[must_use]
pub fn classify(err: &ServerFnError) -> ErrorKind {
    let text = err.to_string();
    if text.contains("permission_denied::")
        || text.contains("permission_check_failed::")
        || text.contains("forbidden:")
    {
        ErrorKind::Denied
    } else if text.contains("auth:") {
        ErrorKind::SignIn
    } else if text.contains("not_found:") {
        ErrorKind::NotFound
    } else {
        ErrorKind::Load
    }
}

/// Error state for a failed My Files call.
///
/// `what` completes "Couldn't load …" ("your files", "this file"). `on_retry`
/// is the Try again action; callers pass a resource refetch.
#[component]
pub fn ErrorBar(
    /// The error from the server fn.
    error: ServerFnError,
    /// What failed to load, in the title.
    what: &'static str,
    /// Called when the user presses Try again.
    on_retry: Callback<()>,
) -> impl IntoView {
    let kind = classify(&error);
    // The raw text stays out of the DOM; keep a debug line for the console.
    log::debug!("meson-app: server fn error shown as {kind:?}");
    match kind {
        ErrorKind::Denied => view! { <FilesReadDenied /> }.into_any(),
        ErrorKind::SignIn => {
            let href = auth_signin_href(Some(&use_location().pathname.get_untracked()));
            view! {
                <div data-testid="meson-error-sign-in">
                    <MessageBar intent=MessageBarIntent::Warning layout=MessageBarLayout::Multiline>
                        <MessageBarTitle>"Sign in again to see your files"</MessageBarTitle>
                        <MessageBarBody>
                            "Your sign-in has expired or isn't valid any more. Signing in again brings you back here."
                        </MessageBarBody>
                        <MessageBarActions>
                            <Link href=href>
                                <Button appearance=ButtonAppearance::Primary>"Sign in again"</Button>
                            </Link>
                        </MessageBarActions>
                    </MessageBar>
                </div>
            }
            .into_any()
        }
        ErrorKind::Load | ErrorKind::NotFound => view! {
            <div data-testid="meson-error-load">
                <MessageBar intent=MessageBarIntent::Error layout=MessageBarLayout::Multiline>
                    <MessageBarTitle>{format!("Couldn't load {what}")}</MessageBarTitle>
                    <MessageBarBody>
                        "Meson didn't answer, so nothing here is up to date. Nothing was changed."
                    </MessageBarBody>
                    <MessageBarActions>
                        <Button
                            appearance=ButtonAppearance::Primary
                            on_click=Callback::new(move |_| on_retry.run(()))
                        >
                            "Try again"
                        </Button>
                    </MessageBarActions>
                </MessageBar>
            </div>
        }
        .into_any(),
    }
}

/// Shown where the viewer lacks `FilesRead`.
#[component]
pub fn FilesReadDenied() -> impl IntoView {
    view! {
        <div data-testid="meson-files-read-denied">
            <EmptyState
                message="You can't open My Files yet"
                description="My Files needs the Files read permission, which this site gives to members. Ask a site owner to add it to your account."
            >
                <EmptyStateCallToAction slot:call_to_action>
                    <Link href=PERMISSION_PERMISSIONS>
                        <Button appearance=ButtonAppearance::Primary>"Ask a site owner"</Button>
                    </Link>
                </EmptyStateCallToAction>
            </EmptyState>
        </div>
    }
}

/// Shown on a file page when the id isn't one of the viewer's files.
#[component]
pub fn NotInYourFiles() -> impl IntoView {
    view! {
        <div data-testid="meson-file-not-in-your-files">
            <EmptyState
                message="This file isn't in your files"
                description="It may have been deleted, or it may belong to someone else. My Files only shows files you uploaded."
            >
                <EmptyStateCallToAction slot:call_to_action>
                    <Link href="/meson">
                        <Button appearance=ButtonAppearance::Primary>"Back to My Files"</Button>
                    </Link>
                </EmptyStateCallToAction>
            </EmptyState>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(msg: &str) -> ErrorKind {
        classify(&ServerFnError::new(msg))
    }

    #[test]
    fn prefixes_map_to_states() {
        assert_eq!(kind("auth: Invalid session user id"), ErrorKind::SignIn);
        assert_eq!(
            kind("auth: Failed to build Valence: boom"),
            ErrorKind::SignIn
        );
        assert_eq!(kind("io: list files failed"), ErrorKind::Load);
        assert_eq!(kind("not_found: File not found"), ErrorKind::NotFound);
        assert_eq!(kind("forbidden: nope"), ErrorKind::Denied);
        assert_eq!(kind("permission_denied::FilesRead"), ErrorKind::Denied);
        assert_eq!(
            kind("permission_check_failed::FilesRead"),
            ErrorKind::Denied
        );
    }

    #[test]
    fn unknown_text_is_a_load_failure() {
        assert_eq!(kind("something odd"), ErrorKind::Load);
        assert_eq!(kind(""), ErrorKind::Load);
    }
}
