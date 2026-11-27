//! What the Storage page renders: totals, a per-app split and a source for
//! every input.
//!
//! The estimate is `bytes / 1e9 × copies × price per GB-month`. Any input can
//! be missing, and each has a source the page names:
//!
//! | Input | Best source | Fallback |
//! |-------|-------------|----------|
//! | Size | Last daily run ([`SizeSource::Measured`]) | [`SizeSource::Stale`] past 36 h; [`SizeSource::Live`] with no run; [`SizeSource::Unavailable`] when neither can be read |
//! | Copies | A site owner's override ([`CopiesSource::Set`]) | What the blob store reports ([`CopiesSource::Reported`]), else counted as 1 ([`CopiesSource::Assumed`]) |
//! | Price | A site owner's price ([`PriceSource::Set`]) | [`PriceSource::Unset`]: no cost anywhere |
//! | History | Spectra ([`HistoryState::Points`]) | [`HistoryState::NoneYet`] before the first run; [`HistoryState::Down`] when Spectra fails |
//!
//! [`build_view`] does the arithmetic; the server fns gather its inputs.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::settings::StorageSettings;

/// A snapshot older than this many hours is [`SizeSource::Stale`].
pub const STALE_AFTER_HOURS: i64 = 36;

/// Days of history the page charts.
pub const HISTORY_DAYS: usize = 30;

/// Where the size numbers came from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SizeSource {
    /// The last daily run, within [`STALE_AFTER_HOURS`].
    Measured {
        /// Day the run covers.
        ds: NaiveDate,
        /// When it measured.
        measured_at: DateTime<Utc>,
    },
    /// The last good run, older than [`STALE_AFTER_HOURS`].
    Stale {
        /// Day the run covers.
        ds: NaiveDate,
        /// When it measured.
        measured_at: DateTime<Utc>,
    },
    /// No run yet: summed live from file metadata.
    Live,
    /// Neither a run nor a live sum could be read.
    Unavailable,
}

impl SizeSource {
    /// `Measured` or `Stale` for a run that measured at `measured_at`, seen at `now`.
    #[must_use]
    pub fn for_run(ds: NaiveDate, measured_at: DateTime<Utc>, now: DateTime<Utc>) -> Self {
        if now - measured_at > Duration::hours(STALE_AFTER_HOURS) {
            Self::Stale { ds, measured_at }
        } else {
            Self::Measured { ds, measured_at }
        }
    }
}

/// Where the copy count came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CopiesSource {
    /// The blob store reports this many copies.
    Reported(i64),
    /// A site owner set this many copies.
    Set(i64),
    /// Nothing known; counted as 1.
    Assumed,
}

impl CopiesSource {
    /// Override first, then what storage reports, then 1.
    #[must_use]
    pub fn resolve(copies_override: Option<i64>, reported: Option<i64>) -> Self {
        match (copies_override, reported) {
            (Some(n), _) if n > 0 => Self::Set(n),
            (_, Some(n)) if n > 0 => Self::Reported(n),
            _ => Self::Assumed,
        }
    }

    /// The count to multiply by.
    #[must_use]
    pub fn count(&self) -> i64 {
        match self {
            Self::Reported(n) | Self::Set(n) => *n,
            Self::Assumed => 1,
        }
    }
}

/// Where the price came from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PriceSource {
    /// A site owner set a price.
    Set {
        /// Price per GB each month.
        per_gb_month: f64,
        /// ISO currency code.
        currency: String,
    },
    /// No price: the page shows sizes only.
    Unset,
}

impl PriceSource {
    fn per_gb_month(&self) -> Option<f64> {
        match self {
            Self::Set { per_gb_month, .. } => Some(*per_gb_month),
            Self::Unset => None,
        }
    }
}

/// Daily totals for the 30-day chart.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum HistoryState {
    /// One total per day, oldest first.
    Points(Vec<(NaiveDate, i64)>),
    /// No daily run has written a row yet.
    NoneYet,
    /// Spectra couldn't be read.
    Down,
}

/// Headline totals.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Totals {
    /// File count.
    pub files: i64,
    /// Bytes before copies.
    pub bytes: i64,
    /// Bytes times copies.
    pub bytes_with_copies: i64,
    /// Estimated cost a month; `None` without a price.
    pub cost_per_month: Option<f64>,
}

/// One app's share of the total.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppRow {
    /// File table (`receipt_scan`).
    pub app_table: String,
    /// Name to show (`Receipt scan`).
    pub app_name: String,
    /// File count.
    pub files: i64,
    /// Bytes before copies.
    pub bytes: i64,
    /// Share of the total bytes, 0.0 to 1.0.
    pub share: f64,
    /// Estimated cost a month; `None` without a price.
    pub cost_per_month: Option<f64>,
}

/// Everything the Storage page renders for one owner (a member, or the site).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StorageView {
    /// Where the sizes came from.
    pub size: SizeSource,
    /// Where the copy count came from.
    pub copies: CopiesSource,
    /// Copies the blob store reported, even when a site owner's number wins.
    pub copies_reported: Option<i64>,
    /// Where the price came from.
    pub price: PriceSource,
    /// The 30-day chart.
    pub history: HistoryState,
    /// Files whose size wasn't recorded (counted as 0, so the total may be low).
    pub missing_size_files: i64,
    /// Headline totals.
    pub totals: Totals,
    /// Per-app split, biggest first.
    pub by_app: Vec<AppRow>,
}

/// One owner's total on the site owners' Everyone tab.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PersonRow {
    /// Valence owner table (`user`, `service_user`, ...).
    pub owner_kind: String,
    /// Owner record key.
    pub owner_id: String,
    /// File count.
    pub files: i64,
    /// Bytes before copies.
    pub bytes: i64,
    /// Share of the site's bytes, 0.0 to 1.0.
    pub share: f64,
    /// Estimated cost a month; `None` without a price.
    pub cost_per_month: Option<f64>,
}

/// The site-wide view for site owners: the site [`StorageView`] plus people
/// and the last run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SiteStorageView {
    /// Site totals, sources and per-app split.
    pub view: StorageView,
    /// Every owner's total, biggest first.
    pub people: Vec<PersonRow>,
    /// Last daily run, when there is one.
    pub run_id: Option<String>,
    /// When the last daily run measured.
    pub measured_at: Option<DateTime<Utc>>,
}

/// Files and bytes for one slice of a snapshot.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    /// File count.
    pub files: i64,
    /// Bytes before copies.
    pub bytes: i64,
    /// Files with no recorded size.
    pub missing_size_files: i64,
}

/// Inputs [`build_view`] turns into a [`StorageView`].
#[derive(Clone, Debug, PartialEq)]
pub struct ViewInput {
    /// Where the sizes came from.
    pub size: SizeSource,
    /// The owner's total.
    pub total: Usage,
    /// The owner's split by File table.
    pub by_app: Vec<(String, Usage)>,
    /// Copies the blob store reported, if it did.
    pub copies_reported: Option<i64>,
    /// The 30-day chart.
    pub history: HistoryState,
}

/// Estimated monthly cost: `bytes / 1e9 × copies × price`.
#[must_use]
#[allow(clippy::cast_precision_loss)] // byte counts far below 2^53 in practice
pub fn cost_per_month(bytes: i64, copies: i64, per_gb_month: Option<f64>) -> Option<f64> {
    per_gb_month.map(|price| bytes as f64 / 1e9 * copies as f64 * price)
}

#[allow(clippy::cast_precision_loss)]
fn share(part: i64, whole: i64) -> f64 {
    if whole > 0 {
        part as f64 / whole as f64
    } else {
        0.0
    }
}

/// `receipt_scan` → `Receipt scan`.
#[must_use]
pub fn app_display_name(app_table: &str) -> String {
    let spaced = app_table.replace('_', " ");
    let mut chars = spaced.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Combine a snapshot slice, the settings and the history into the page view.
#[must_use]
pub fn build_view(input: ViewInput, settings: &StorageSettings) -> StorageView {
    let copies = CopiesSource::resolve(settings.copies_override, input.copies_reported);
    let price = settings.price_source();
    let per_gb = price.per_gb_month();
    let n = copies.count();

    let mut by_app: Vec<AppRow> = input
        .by_app
        .into_iter()
        .map(|(app_table, usage)| AppRow {
            app_name: app_display_name(&app_table),
            app_table,
            files: usage.files,
            bytes: usage.bytes,
            share: share(usage.bytes, input.total.bytes),
            cost_per_month: cost_per_month(usage.bytes, n, per_gb),
        })
        .collect();
    by_app.sort_by(|a, b| {
        b.bytes
            .cmp(&a.bytes)
            .then_with(|| a.app_name.cmp(&b.app_name))
    });

    StorageView {
        size: input.size,
        totals: Totals {
            files: input.total.files,
            bytes: input.total.bytes,
            bytes_with_copies: input.total.bytes.saturating_mul(n),
            cost_per_month: cost_per_month(input.total.bytes, n, per_gb),
        },
        missing_size_files: input.total.missing_size_files,
        copies,
        copies_reported: input.copies_reported.filter(|n| *n > 0),
        price,
        history: input.history,
        by_app,
    }
}

/// People rows for the site view, biggest first.
#[must_use]
pub fn build_people(
    owners: Vec<(String, String, Usage)>,
    site_bytes: i64,
    view: &StorageView,
) -> Vec<PersonRow> {
    let per_gb = view.price.per_gb_month();
    let n = view.copies.count();
    let mut people: Vec<PersonRow> = owners
        .into_iter()
        .map(|(owner_kind, owner_id, usage)| PersonRow {
            owner_kind,
            owner_id,
            files: usage.files,
            bytes: usage.bytes,
            share: share(usage.bytes, site_bytes),
            cost_per_month: cost_per_month(usage.bytes, n, per_gb),
        })
        .collect();
    people.sort_by(|a, b| {
        b.bytes
            .cmp(&a.bytes)
            .then_with(|| a.owner_id.cmp(&b.owner_id))
    });
    people
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::float_cmp)]

    use super::*;

    fn ds() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 23).unwrap()
    }

    fn input(bytes: i64, reported: Option<i64>) -> ViewInput {
        ViewInput {
            size: SizeSource::Live,
            total: Usage {
                files: 3,
                bytes,
                missing_size_files: 1,
            },
            by_app: vec![
                (
                    "profile_photo".into(),
                    Usage {
                        files: 1,
                        bytes: bytes / 4,
                        missing_size_files: 0,
                    },
                ),
                (
                    "receipt_scan".into(),
                    Usage {
                        files: 2,
                        bytes: bytes - bytes / 4,
                        missing_size_files: 1,
                    },
                ),
            ],
            copies_reported: reported,
            history: HistoryState::NoneYet,
        }
    }

    #[test]
    fn forty_hour_old_snapshot_is_stale() {
        let now = Utc::now();
        let measured = now - Duration::hours(40);
        assert_eq!(
            SizeSource::for_run(ds(), measured, now),
            SizeSource::Stale {
                ds: ds(),
                measured_at: measured
            }
        );
        let fresh = now - Duration::hours(20);
        assert!(matches!(
            SizeSource::for_run(ds(), fresh, now),
            SizeSource::Measured { .. }
        ));
    }

    #[test]
    fn copies_override_beats_reported_beats_assumed() {
        assert_eq!(
            CopiesSource::resolve(Some(2), Some(3)),
            CopiesSource::Set(2)
        );
        assert_eq!(
            CopiesSource::resolve(None, Some(3)),
            CopiesSource::Reported(3)
        );
        assert_eq!(CopiesSource::resolve(None, None), CopiesSource::Assumed);
        assert_eq!(CopiesSource::Assumed.count(), 1);
    }

    #[test]
    fn no_price_gives_no_cost() {
        let view = build_view(input(4_000_000_000, Some(3)), &StorageSettings::default());
        assert_eq!(view.price, PriceSource::Unset);
        assert_eq!(view.totals.cost_per_month, None);
        assert!(view.by_app.iter().all(|a| a.cost_per_month.is_none()));
        assert_eq!(view.totals.bytes_with_copies, 12_000_000_000);
    }

    #[test]
    fn cost_is_gb_times_copies_times_price() {
        let settings = StorageSettings {
            copies_override: Some(2),
            price_per_gb_month: Some(0.5),
            ..StorageSettings::default()
        };
        let view = build_view(input(4_000_000_000, Some(3)), &settings);
        assert_eq!(view.copies, CopiesSource::Set(2));
        assert_eq!(view.copies_reported, Some(3));
        assert_eq!(view.totals.cost_per_month, Some(4.0));
        assert_eq!(view.missing_size_files, 1);
        assert_eq!(view.by_app[0].app_name, "Receipt scan");
        assert_eq!(view.by_app[0].share, 0.75);
        assert_eq!(view.by_app[0].cost_per_month, Some(3.0));
    }

    #[test]
    fn zero_total_has_zero_share() {
        let view = build_view(input(0, None), &StorageSettings::default());
        assert!(view.by_app.iter().all(|a| a.share == 0.0));
    }

    #[test]
    fn display_name_humanizes_table() {
        assert_eq!(app_display_name("receipt_scan"), "Receipt scan");
        assert_eq!(app_display_name(""), "");
    }
}
