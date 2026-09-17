//! Session helpers for Meson SSR server functions.

#[cfg(feature = "ssr")]
mod ssr {
    use leptos::prelude::ServerFnError;
    use thiserror::Error;

    /// Domain errors for My Files SSR; mapped to prefixed [`ServerFnError`] at the wire.
    #[derive(Debug, Error)]
    pub enum MesonAppError {
        /// Missing or invalid session.
        #[error("auth: {0}")]
        Auth(String),
        /// Missing row or ownership collapse.
        #[error("not_found: {0}")]
        NotFound(String),
        /// Ownership denial (wire usually collapses to `not_found`; kept for stable `forbidden:` prefix).
        #[allow(dead_code)] // via `forbidden_error` in unit tests
        #[error("forbidden: {0}")]
        Forbidden(String),
        /// Backend / Valence I/O failure.
        #[error("io: {msg}")]
        Io {
            /// Client-visible detail after the `io:` prefix.
            msg: String,
            /// Retained source for operator tracing (not on the wire).
            #[source]
            source: Option<Box<dyn std::error::Error + Send + Sync>>,
        },
    }

    impl MesonAppError {
        pub(crate) fn auth(detail: impl Into<String>) -> Self {
            Self::Auth(detail.into())
        }

        pub(crate) fn not_found(detail: impl Into<String>) -> Self {
            Self::NotFound(detail.into())
        }

        #[allow(dead_code)] // via `forbidden_error` in unit tests
        pub(crate) fn forbidden(detail: impl Into<String>) -> Self {
            Self::Forbidden(detail.into())
        }

        pub(crate) fn io_msg(msg: impl Into<String>) -> Self {
            Self::Io {
                msg: msg.into(),
                source: None,
            }
        }

        pub(crate) fn io_source(
            msg: impl Into<String>,
            source: impl std::error::Error + Send + Sync + 'static,
        ) -> Self {
            Self::Io {
                msg: msg.into(),
                source: Some(Box::new(source)),
            }
        }

        /// Log once at the server-fn boundary, then convert to wire error.
        pub(crate) fn into_server_fn(self) -> ServerFnError {
            match &self {
                Self::Auth(_) => {
                    tracing::warn!(outcome = "auth", error = %self, "meson-app server fn failed");
                }
                Self::NotFound(_) => {
                    tracing::debug!(outcome = "not_found", error = %self, "meson-app server fn failed");
                }
                Self::Forbidden(_) => {
                    tracing::debug!(
                        target: "security",
                        outcome = "forbidden",
                        error = %self,
                        "meson-app ownership denial"
                    );
                }
                Self::Io { source, .. } => {
                    if let Some(src) = source {
                        tracing::error!(
                            outcome = "io",
                            error = %self,
                            source = %src,
                            "meson-app server fn failed"
                        );
                    } else {
                        tracing::error!(outcome = "io", error = %self, "meson-app server fn failed");
                    }
                }
            }
            ServerFnError::new(self.to_string())
        }
    }

    /// Stable `auth:` prefix for missing session failures.
    pub(crate) fn auth_error(detail: impl Into<String>) -> MesonAppError {
        MesonAppError::auth(detail)
    }

    /// Stable `not_found:` prefix.
    pub(crate) fn not_found_error(detail: impl Into<String>) -> MesonAppError {
        MesonAppError::not_found(detail)
    }

    /// Stable `forbidden:` prefix for ownership denials.
    #[allow(dead_code)] // asserted in service unit tests
    pub(crate) fn forbidden_error(detail: impl Into<String>) -> MesonAppError {
        MesonAppError::forbidden(detail)
    }

    /// Stable `io:` prefix for backend failures.
    pub(crate) fn io_error(detail: impl Into<String>) -> MesonAppError {
        MesonAppError::io_msg(detail)
    }

    /// Max preview payload size (matches lepton upload cap).
    pub(crate) const MAX_PREVIEW_BYTES: usize = 5 * 1024 * 1024;

    /// Reject when metadata size already exceeds the preview cap (before blob fetch).
    pub(crate) fn reject_oversized_preview_metadata(size_bytes: i64) -> Result<(), MesonAppError> {
        let max = i64::try_from(MAX_PREVIEW_BYTES).unwrap_or(i64::MAX);
        if size_bytes > max {
            Err(MesonAppError::io_msg("File exceeds preview size limit"))
        } else {
            Ok(())
        }
    }

    pub(crate) fn require_session(ctx: &higgs::Higgs) -> Result<(), MesonAppError> {
        if ctx.session_user_id().is_some() {
            Ok(())
        } else {
            Err(auth_error("Authentication required"))
        }
    }

    pub(crate) fn session_valence_from_ctx(
        ctx: &higgs::Higgs,
    ) -> Result<valence::Valence, MesonAppError> {
        require_session(ctx)?;
        ctx.valence()
            .map_err(|e| MesonAppError::auth(format!("Failed to build Valence: {e}")))
    }

    pub(crate) fn session_user_record_id(
        ctx: &higgs::Higgs,
    ) -> Result<valence::RecordId, MesonAppError> {
        let raw = ctx
            .session_user_id()
            .ok_or_else(|| auth_error("Authentication required"))?;
        valence::RecordId::parse(raw).ok_or_else(|| auth_error("Invalid session user id"))
    }
}

#[cfg(feature = "ssr")]
pub(crate) use ssr::*;
// `MesonAppError` itself is part of the ssr-only public surface (it's the error
// type of the re-exported `service::*` functions integration tests call).
#[cfg(feature = "ssr")]
pub use ssr::MesonAppError;
