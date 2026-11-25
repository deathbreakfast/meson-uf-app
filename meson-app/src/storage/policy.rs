//! Valence write rule for `meson_storage_settings`: only an actor holding the
//! Gauge permission `StorageAdmin` may create or update the row.
//!
//! The rule runs with the session actor; nothing here elevates to System.

use std::any::Any;

use async_trait::async_trait;
use valence::{ActorContext, PolicyEvaluator, PrivacyOperation, Result, Valence};

use crate::permissions::MesonPermission;

/// Allows the operation when the actor holds `StorageAdmin`.
///
/// Fails closed: a Gauge read error denies. `actor_can_raw` lets System and
/// Super User through.
#[derive(Debug, Clone)]
pub struct StorageAdminPolicy {
    /// Valence privacy rule name (namespaced).
    pub rule_name: &'static str,
}

#[async_trait]
impl PolicyEvaluator for StorageAdminPolicy {
    fn name(&self) -> &'static str {
        self.rule_name
    }

    fn description(&self) -> Option<&'static str> {
        Some("Gauge permission StorageAdmin")
    }

    async fn evaluate(
        &self,
        _op: PrivacyOperation,
        _record: &serde_json::Value,
        _actor: &dyn ActorContext,
        v: &Valence,
    ) -> Result<bool> {
        // `actor_can_raw` is required inside a PolicyEvaluator (`actor_can` re-enters privacy).
        match gauge::service::actor_can_raw(v, MesonPermission::StorageAdmin.as_str()).await {
            Ok(allowed) => Ok(allowed),
            Err(e) => {
                tracing::error!(error = %e, "StorageAdmin check failed; denying settings write");
                Ok(false)
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Create / update gate for `meson_storage_settings`.
pub const STORAGE_ADMIN: StorageAdminPolicy = StorageAdminPolicy {
    rule_name: "meson::STORAGE_ADMIN",
};
