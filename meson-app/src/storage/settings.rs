//! Site storage cost settings: a copies override and a price per GB-month.
//!
//! Stored as one `meson_storage_settings` Valence row (id [`SETTINGS_ID`]).
//! Any signed-in member can read it; only an actor holding the Gauge
//! permission `StorageAdmin` can create or update it, enforced both by the
//! server fn and by the schema's privacy rule.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::view::PriceSource;

/// Record id of the single settings row.
pub const SETTINGS_ID: &str = "site";

/// Currency when none has been set.
pub const DEFAULT_CURRENCY: &str = "USD";

/// Allowed copies override.
pub const COPIES_RANGE: std::ops::RangeInclusive<i64> = 1..=10;

/// The site's cost settings, or the defaults when nobody has saved any.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StorageSettings {
    /// Copies typed in by a site owner; `None` follows what storage reports.
    pub copies_override: Option<i64>,
    /// Price per GB each month; `None` shows sizes only.
    pub price_per_gb_month: Option<f64>,
    /// ISO currency code for the price.
    pub currency: String,
    /// Who saved last (`user:<id>`), when known.
    pub updated_by: Option<String>,
    /// When they saved.
    pub updated_at: Option<DateTime<Utc>>,
}

impl Default for StorageSettings {
    fn default() -> Self {
        Self {
            copies_override: None,
            price_per_gb_month: None,
            currency: DEFAULT_CURRENCY.to_string(),
            updated_by: None,
            updated_at: None,
        }
    }
}

impl StorageSettings {
    /// The price as a [`PriceSource`].
    #[must_use]
    pub fn price_source(&self) -> PriceSource {
        match self.price_per_gb_month {
            Some(per_gb_month) => PriceSource::Set {
                per_gb_month,
                currency: self.currency.clone(),
            },
            None => PriceSource::Unset,
        }
    }
}

/// Check a save request: copies within [`COPIES_RANGE`], price finite and ≥ 0.
///
/// # Errors
///
/// A sentence for the settings dialog naming the bad field.
pub fn validate_settings(
    copies_override: Option<i64>,
    price_per_gb_month: Option<f64>,
) -> Result<(), String> {
    if let Some(n) = copies_override {
        if !COPIES_RANGE.contains(&n) {
            return Err(format!(
                "Copies must be between {} and {}.",
                COPIES_RANGE.start(),
                COPIES_RANGE.end()
            ));
        }
    }
    if let Some(p) = price_per_gb_month {
        if !p.is_finite() || p < 0.0 {
            return Err("Price per GB must be zero or more.".to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_bounds() {
        assert!(validate_settings(None, None).is_ok());
        assert!(validate_settings(Some(1), Some(0.0)).is_ok());
        assert!(validate_settings(Some(10), Some(0.023)).is_ok());
        assert!(validate_settings(Some(0), None).is_err());
        assert!(validate_settings(Some(11), None).is_err());
        assert!(validate_settings(None, Some(-0.01)).is_err());
        assert!(validate_settings(None, Some(f64::NAN)).is_err());
    }
}
