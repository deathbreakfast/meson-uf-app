//! Numbers and sentences the Storage page shows, kept apart from the views so
//! they can be tested without a browser.

use chrono::{DateTime, Utc};

use crate::server::MyFileRow;
use crate::storage::settings::{StorageSettings, DEFAULT_CURRENCY};
use crate::storage::view::{
    app_display_name, build_view, CopiesSource, HistoryState, PriceSource, SizeSource, StorageView,
    Usage, ViewInput,
};

const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];

/// Bytes in decimal units, as storage providers bill them: `18.4 GB`,
/// `410 KB`, `1.2 TB`. One decimal below 100, none above.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn format_bytes(bytes: i64) -> String {
    let negative = bytes < 0;
    let mut value = bytes.unsigned_abs() as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    let sign = if negative { "-" } else { "" };
    if unit == 0 {
        format!("{sign}{} B", bytes.unsigned_abs())
    } else if value < 100.0 {
        format!("{sign}{value:.1} {}", UNITS[unit])
    } else {
        format!("{sign}{value:.0} {}", UNITS[unit])
    }
}

/// A count with thousands separators: `12,486`.
#[must_use]
pub fn format_count(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 {
        format!("-{out}")
    } else {
        out
    }
}

fn money(amount: &str, currency: &str) -> String {
    if currency.is_empty() || currency == DEFAULT_CURRENCY {
        format!("${amount}")
    } else {
        format!("{amount} {currency}")
    }
}

/// A monthly cost: `$1.27`, or `under $0.01` for anything above zero but
/// below a cent.
#[must_use]
pub fn format_cost(amount: f64, currency: &str) -> String {
    if amount > 0.0 && amount < 0.01 {
        format!("under {}", money("0.01", currency))
    } else {
        money(&format!("{amount:.2}"), currency)
    }
}

/// A price per GB, with as many decimals as it was typed with (at least 2):
/// `$0.023`, `$0.10`.
#[must_use]
pub fn format_price(per_gb: f64, currency: &str) -> String {
    let mut text = format!("{per_gb:.6}");
    while text.ends_with('0') && text.len() - text.find('.').unwrap_or(0) > 3 {
        text.pop();
    }
    money(&text, currency)
}

/// `1 copy`, `3 copies`.
#[must_use]
pub fn copies_text(n: i64) -> String {
    if n == 1 {
        "1 copy".to_string()
    } else {
        format!("{n} copies")
    }
}

/// `1 file`, `212 files`.
#[must_use]
pub fn files_text(n: i64) -> String {
    if n == 1 {
        "1 file".to_string()
    } else {
        format!("{} files", format_count(n))
    }
}

/// The price and currency, when a site owner set one.
#[must_use]
pub fn price_of(price: &PriceSource) -> Option<(f64, &str)> {
    match price {
        PriceSource::Set {
            per_gb_month,
            currency,
        } => Some((*per_gb_month, currency.as_str())),
        PriceSource::Unset => None,
    }
}

/// The arithmetic behind the estimate:
/// `18.4 GB × 3 copies × $0.023 = $1.27 a month`, or
/// `18.4 GB × 1 copy = 18.4 GB stored` without a price.
#[must_use]
pub fn worked_out(bytes: i64, copies: i64, price: &PriceSource) -> String {
    let size = format_bytes(bytes);
    let copies_part = copies_text(copies);
    match price_of(price) {
        Some((per_gb, currency)) => {
            let cost = crate::storage::view::cost_per_month(bytes, copies, Some(per_gb))
                .unwrap_or_default();
            format!(
                "{size} × {copies_part} × {} = {} a month",
                format_price(per_gb, currency),
                format_cost(cost, currency)
            )
        }
        None => format!(
            "{size} × {copies_part} = {} stored",
            format_bytes(bytes.saturating_mul(copies))
        ),
    }
}

/// Value of the cost stat: `$1.27`, or `$0.42 or more` when copies are
/// assumed. `None` without a price (the page shows the stored size instead).
#[must_use]
pub fn cost_stat(view: &StorageView) -> Option<String> {
    let (_, currency) = price_of(&view.price)?;
    let cost = format_cost(view.totals.cost_per_month.unwrap_or_default(), currency);
    Some(if view.copies == CopiesSource::Assumed {
        format!("{cost} or more")
    } else {
        cost
    })
}

/// Bytes added over the charted days (last point minus first), when there
/// are at least two points.
#[must_use]
pub fn change_over_history(history: &HistoryState) -> Option<i64> {
    match history {
        HistoryState::Points(points) if points.len() >= 2 => {
            let first = points.first().map_or(0, |p| p.1);
            let last = points.last().map_or(0, |p| p.1);
            Some(last - first)
        }
        _ => None,
    }
}

/// `+2.5 GB`, `-1.0 GB`, `No change`.
#[must_use]
pub fn format_change(delta: i64) -> String {
    match delta {
        0 => "No change".to_string(),
        d if d > 0 => format!("+{}", format_bytes(d)),
        d => format_bytes(d),
    }
}

/// Whole days since `measured_at`, at least 1.
#[must_use]
pub fn days_old(measured_at: DateTime<Utc>, now: DateTime<Utc>) -> i64 {
    (now - measured_at).num_days().max(1)
}

/// `1 day`, `3 days`.
#[must_use]
pub fn days_text(n: i64) -> String {
    if n == 1 {
        "1 day".to_string()
    } else {
        format!("{n} days")
    }
}

/// Error shown under the price field.
pub const PRICE_HINT: &str = "Enter a price like 0.023";

/// Parse the price field: empty means no price; otherwise a number ≥ 0, with
/// an optional leading `$`.
///
/// # Errors
///
/// [`PRICE_HINT`] when the text isn't a price.
pub fn parse_price(text: &str) -> Result<Option<f64>, &'static str> {
    let trimmed = text.trim();
    let trimmed = trimmed.strip_prefix('$').unwrap_or(trimmed).trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    match trimmed.parse::<f64>() {
        Ok(p) if p.is_finite() && p >= 0.0 => Ok(Some(p)),
        _ => Err(PRICE_HINT),
    }
}

/// Error shown under the copies field.
pub const COPIES_HINT: &str = "Enter a whole number from 1 to 10";

/// Parse the copies field.
///
/// # Errors
///
/// [`COPIES_HINT`] unless the text is a whole number in 1..=10.
pub fn parse_copies(text: &str) -> Result<i64, &'static str> {
    match text.trim().parse::<i64>() {
        Ok(n) if crate::storage::settings::COPIES_RANGE.contains(&n) => Ok(n),
        _ => Err(COPIES_HINT),
    }
}

/// The daily run's status on the Everyone tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunStatus {
    /// The last run is under 36 hours old.
    Healthy,
    /// The last good run is this many days old.
    Failing(i64),
    /// Chronon is wired, but no run has written a snapshot yet.
    NotRunYet,
    /// No snapshot and no Chronon on this site.
    NotScheduled,
}

/// Status from the snapshot age and whether the host installed a run hook.
#[must_use]
pub fn run_status(
    measured_at: Option<DateTime<Utc>>,
    stale: bool,
    hook_installed: bool,
    now: DateTime<Utc>,
) -> RunStatus {
    match measured_at {
        Some(at) if stale => RunStatus::Failing(days_old(at, now)),
        Some(_) => RunStatus::Healthy,
        None if hook_installed => RunStatus::NotRunYet,
        None => RunStatus::NotScheduled,
    }
}

/// Whether a size source is a snapshot older than 36 hours.
#[must_use]
pub fn is_stale(size: &SizeSource) -> bool {
    matches!(size, SizeSource::Stale { .. })
}

/// Settings that reproduce `view`'s copies and price, for rebuilding it.
fn settings_of(view: &StorageView) -> StorageSettings {
    let (price_per_gb_month, currency) = match &view.price {
        PriceSource::Set {
            per_gb_month,
            currency,
        } => (Some(*per_gb_month), currency.clone()),
        PriceSource::Unset => (None, DEFAULT_CURRENCY.to_string()),
    };
    StorageSettings {
        copies_override: match view.copies {
            CopiesSource::Set(n) => Some(n),
            _ => None,
        },
        price_per_gb_month,
        currency,
        updated_by: None,
        updated_at: None,
    }
}

/// When no daily run could be read, sum the member's own files instead
/// ([`SizeSource::Live`]). Any other view comes back unchanged.
///
/// Files with no recorded size count as missing. The history state is kept,
/// so a Spectra outage still says history is down.
#[must_use]
pub fn with_live_sizes(view: StorageView, files: &[MyFileRow]) -> StorageView {
    if view.size != SizeSource::Unavailable {
        return view;
    }
    let mut total = Usage::default();
    let mut by_app: Vec<(String, Usage)> = Vec::new();
    for f in files {
        let bytes = f.size_bytes.max(0);
        let missing = i64::from(f.size_bytes <= 0);
        total.files += 1;
        total.bytes += bytes;
        total.missing_size_files += missing;
        let i = by_app
            .iter()
            .position(|(t, _)| *t == f.source_table)
            .unwrap_or_else(|| {
                by_app.push((f.source_table.clone(), Usage::default()));
                by_app.len() - 1
            });
        let slot = &mut by_app[i].1;
        slot.files += 1;
        slot.bytes += bytes;
        slot.missing_size_files += missing;
    }
    let settings = settings_of(&view);
    build_view(
        ViewInput {
            size: SizeSource::Live,
            total,
            by_app,
            copies_reported: view.copies_reported,
            history: view.history,
        },
        &settings,
    )
}

/// The member's biggest files, largest first.
#[must_use]
pub fn biggest_files(files: &[MyFileRow], n: usize) -> Vec<MyFileRow> {
    let mut sorted: Vec<MyFileRow> = files.to_vec();
    sorted.sort_by(|a, b| {
        b.size_bytes
            .cmp(&a.size_bytes)
            .then_with(|| a.file_name.cmp(&b.file_name))
    });
    sorted.truncate(n);
    sorted
}

/// Files locked by the virus scan.
#[must_use]
pub fn quarantined_files(files: &[MyFileRow]) -> Vec<MyFileRow> {
    files
        .iter()
        .filter(|f| f.file_status == "quarantined")
        .cloned()
        .collect()
}

/// `Receipt scan` for a File row's table.
#[must_use]
pub fn app_name(source_table: &str) -> String {
    app_display_name(source_table)
}

/// Path segment for a file id (`table:id` → `table%3Aid`).
#[must_use]
pub fn file_href(id: &str) -> String {
    format!("/meson/files/{}", id.replace(':', "%3A"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use chrono::{Duration, NaiveDate};

    use super::*;
    use crate::storage::view::Totals;

    fn file(table: &str, name: &str, size: i64, status: &str) -> MyFileRow {
        MyFileRow {
            id: format!("{table}:{name}"),
            source_table: table.to_string(),
            file_name: name.to_string(),
            mime_type: "image/png".to_string(),
            size_bytes: size,
            file_status: status.to_string(),
            uploaded_at: String::new(),
        }
    }

    fn unavailable(price: PriceSource, history: HistoryState) -> StorageView {
        StorageView {
            size: SizeSource::Unavailable,
            copies: CopiesSource::Assumed,
            copies_reported: None,
            price,
            history,
            missing_size_files: 0,
            totals: Totals::default(),
            by_app: Vec::new(),
        }
    }

    #[test]
    fn bytes_use_decimal_units() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(410_000), "410 KB");
        assert_eq!(format_bytes(6_300_000), "6.3 MB");
        assert_eq!(format_bytes(18_400_000_000), "18.4 GB");
        assert_eq!(format_bytes(412_000_000_000), "412 GB");
        assert_eq!(format_bytes(1_236_000_000_000), "1.2 TB");
        assert_eq!(format_bytes(-2_500_000_000), "-2.5 GB");
    }

    #[test]
    fn counts_get_separators() {
        assert_eq!(format_count(3), "3");
        assert_eq!(format_count(12_486), "12,486");
        assert_eq!(format_count(1_234_567), "1,234,567");
    }

    #[test]
    fn cost_below_a_cent_says_under() {
        assert_eq!(format_cost(1.2696, "USD"), "$1.27");
        assert_eq!(format_cost(0.004, "USD"), "under $0.01");
        assert_eq!(format_cost(0.0, "USD"), "$0.00");
        assert_eq!(format_cost(2.5, "EUR"), "2.50 EUR");
    }

    #[test]
    fn price_keeps_typed_decimals() {
        assert_eq!(format_price(0.023, "USD"), "$0.023");
        assert_eq!(format_price(0.1, "USD"), "$0.10");
        assert_eq!(format_price(1.0, "USD"), "$1.00");
    }

    #[test]
    fn worked_out_with_and_without_price() {
        let price = PriceSource::Set {
            per_gb_month: 0.023,
            currency: "USD".into(),
        };
        assert_eq!(
            worked_out(18_400_000_000, 3, &price),
            "18.4 GB × 3 copies × $0.023 = $1.27 a month"
        );
        assert_eq!(
            worked_out(18_400_000_000, 1, &PriceSource::Unset),
            "18.4 GB × 1 copy = 18.4 GB stored"
        );
        assert_eq!(
            worked_out(18_400_000_000, 3, &PriceSource::Unset),
            "18.4 GB × 3 copies = 55.2 GB stored"
        );
    }

    #[test]
    fn price_parsing() {
        assert_eq!(parse_price(""), Ok(None));
        assert_eq!(parse_price("  "), Ok(None));
        assert_eq!(parse_price("0.023"), Ok(Some(0.023)));
        assert_eq!(parse_price("$0.5"), Ok(Some(0.5)));
        assert_eq!(parse_price("abc"), Err(PRICE_HINT));
        assert_eq!(parse_price("-1"), Err(PRICE_HINT));
        assert_eq!(parse_price("NaN"), Err(PRICE_HINT));
    }

    #[test]
    fn copies_parsing() {
        assert_eq!(parse_copies("3"), Ok(3));
        assert_eq!(parse_copies("0"), Err(COPIES_HINT));
        assert_eq!(parse_copies("11"), Err(COPIES_HINT));
        assert_eq!(parse_copies("2.5"), Err(COPIES_HINT));
    }

    #[test]
    fn change_needs_two_points() {
        let d = |n| NaiveDate::from_ymd_opt(2026, 11, n).unwrap();
        assert_eq!(change_over_history(&HistoryState::NoneYet), None);
        assert_eq!(
            change_over_history(&HistoryState::Points(vec![(d(1), 5)])),
            None
        );
        let up = HistoryState::Points(vec![(d(1), 1_000), (d(2), 3_500)]);
        assert_eq!(change_over_history(&up), Some(2_500));
        assert_eq!(format_change(2_500_000_000), "+2.5 GB");
        assert_eq!(format_change(0), "No change");
        assert_eq!(format_change(-1_000_000_000), "-1.0 GB");
    }

    #[test]
    fn run_status_from_snapshot_and_hook() {
        let now = Utc::now();
        let fresh = Some(now - Duration::hours(5));
        let old = Some(now - Duration::hours(80));
        assert_eq!(run_status(fresh, false, false, now), RunStatus::Healthy);
        assert_eq!(run_status(old, true, false, now), RunStatus::Failing(3));
        assert_eq!(run_status(None, false, true, now), RunStatus::NotRunYet);
        assert_eq!(run_status(None, false, false, now), RunStatus::NotScheduled);
        assert_eq!(days_old(now - Duration::hours(40), now), 1);
    }

    #[test]
    fn live_sizes_sum_own_files() {
        let price = PriceSource::Set {
            per_gb_month: 0.5,
            currency: "USD".into(),
        };
        let files = vec![
            file("receipt_scan", "a.png", 3_000_000_000, "available"),
            file("receipt_scan", "b.png", 0, "pending_virus_scan"),
            file("profile_photo", "c.png", 1_000_000_000, "quarantined"),
        ];
        let view = with_live_sizes(unavailable(price, HistoryState::NoneYet), &files);
        assert_eq!(view.size, SizeSource::Live);
        assert_eq!(view.totals.files, 3);
        assert_eq!(view.totals.bytes, 4_000_000_000);
        assert_eq!(view.missing_size_files, 1);
        assert_eq!(view.totals.cost_per_month, Some(2.0));
        assert_eq!(view.by_app[0].app_name, "Receipt scan");
        assert_eq!(view.by_app[0].files, 2);
        assert_eq!(view.history, HistoryState::NoneYet);
    }

    #[test]
    fn live_sizes_leave_measured_views_alone() {
        let mut view = unavailable(PriceSource::Unset, HistoryState::Down);
        view.size = SizeSource::Live;
        let files = vec![file("t", "a", 10, "available")];
        assert_eq!(with_live_sizes(view.clone(), &files), view);
    }

    #[test]
    fn biggest_and_quarantined() {
        let files = vec![
            file("t", "small", 1, "available"),
            file("t", "big", 50, "available"),
            file("t", "mid", 10, "quarantined"),
        ];
        let top: Vec<String> = biggest_files(&files, 2)
            .into_iter()
            .map(|f| f.file_name)
            .collect();
        assert_eq!(top, vec!["big", "mid"]);
        assert_eq!(quarantined_files(&files)[0].file_name, "mid");
        assert_eq!(file_href("t:mid"), "/meson/files/t%3Amid");
    }

    #[test]
    fn cost_stat_flags_assumed_copies() {
        let mut view = unavailable(
            PriceSource::Set {
                per_gb_month: 0.023,
                currency: "USD".into(),
            },
            HistoryState::NoneYet,
        );
        view.totals.cost_per_month = Some(0.42);
        assert_eq!(cost_stat(&view).as_deref(), Some("$0.42 or more"));
        view.copies = CopiesSource::Reported(3);
        assert_eq!(cost_stat(&view).as_deref(), Some("$0.42"));
        view.price = PriceSource::Unset;
        assert_eq!(cost_stat(&view), None);
    }
}
