//! Read the newest `meson_storage_daily` snapshot and the daily totals before it.
//!
//! [`latest_snapshot`] finds the newest `ds`, then reads every row of that
//! day's newest run, 1,000 rows a page (the mem and sqlite Spectra backends
//! cap one query at 1,000 rows). [`history`] returns one total a day for the
//! 30-day chart. The row shapes are documented in [`super::schema`].
//!
//! An empty or missing table is `Ok(None)` / an empty history: Spectra returns
//! no rows for a table nobody has written yet. Only a backend failure or a row
//! that breaks the contract is an `Err`.

use chrono::{DateTime, NaiveDate, Utc};
use serde_json::json;
use spectra::spectra_core::{
    EventRow, EventsQueryFilter, GridFilterItem, GridFilterModel, GridFilterOperator,
};
use spectra::SpectraRouter;
use thiserror::Error;

use super::schema::{MesonStorageDaily, ALL, SITE_ID, SITE_KIND, TABLE};

/// Rows per page; matches [`spectra::MAX_EVENT_QUERY_LIMIT`].
pub const PAGE_SIZE: u32 = spectra::MAX_EVENT_QUERY_LIMIT;

/// Why a snapshot or history read failed.
#[derive(Debug, Error)]
pub enum SnapshotError {
    /// Spectra query failed.
    #[error("spectra query failed: {0}")]
    Spectra(#[from] spectra::Error),
    /// A row doesn't follow the row contract.
    #[error("bad meson_storage_daily row: {0}")]
    BadRow(String),
}

/// Whose usage a row describes: the whole site, or one Valence owner.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Owner {
    /// `site`, or the owner's Valence table (`user`, `service_user`, ...).
    pub kind: String,
    /// `all` for the site, else the owner's record key.
    pub id: String,
}

impl Owner {
    /// The site-wide owner (`site` / `all`).
    #[must_use]
    pub fn site() -> Self {
        Self {
            kind: SITE_KIND.to_string(),
            id: SITE_ID.to_string(),
        }
    }

    /// Owner for a Valence record id (`user:abc` becomes `user` / `abc`).
    #[must_use]
    pub fn from_record(rid: &valence::RecordId) -> Self {
        Self {
            kind: rid.table().to_string(),
            id: rid.id().to_string(),
        }
    }

    /// Whether this is the site-wide owner.
    #[must_use]
    pub fn is_site(&self) -> bool {
        self.kind == SITE_KIND
    }
}

/// One parsed `meson_storage_daily` row.
#[derive(Clone, Debug, PartialEq)]
pub struct StorageRow {
    /// Day the snapshot covers.
    pub ds: NaiveDate,
    /// When the run measured.
    pub measured_at: DateTime<Utc>,
    /// Run that wrote the row.
    pub run_id: String,
    /// Whose usage this is.
    pub owner: Owner,
    /// File table, or `*`.
    pub app_table: String,
    /// Scan status, or `*`.
    pub file_status: String,
    /// File count.
    pub files: i64,
    /// Bytes before copies.
    pub bytes: i64,
    /// Files with no recorded size.
    pub missing_size_files: i64,
    /// Copies the blob store reports (site rows only).
    pub copies_reported: Option<i64>,
}

impl StorageRow {
    fn from_event(row: &EventRow) -> Result<Self, SnapshotError> {
        let raw: MesonStorageDaily = serde_json::from_value(row.fields.clone())
            .map_err(|e| SnapshotError::BadRow(e.to_string()))?;
        let ds = parse_ds(&raw.ds)?;
        let measured_at = DateTime::parse_from_rfc3339(&raw.measured_at)
            .map_err(|e| SnapshotError::BadRow(format!("measured_at {:?}: {e}", raw.measured_at)))?
            .with_timezone(&Utc);
        Ok(Self {
            ds,
            measured_at,
            run_id: raw.run_id,
            owner: Owner {
                kind: raw.owner_kind,
                id: raw.owner_id,
            },
            app_table: raw.app_table,
            file_status: raw.file_status,
            files: raw.files,
            bytes: raw.bytes,
            missing_size_files: raw.missing_size_files,
            copies_reported: (raw.copies_reported > 0).then_some(raw.copies_reported),
        })
    }

    fn is_total(&self) -> bool {
        self.app_table == ALL && self.file_status == ALL
    }

    fn is_by_app(&self) -> bool {
        self.app_table != ALL && self.file_status == ALL
    }

    fn is_by_status(&self) -> bool {
        self.app_table == ALL && self.file_status != ALL
    }
}

fn parse_ds(ds: &str) -> Result<NaiveDate, SnapshotError> {
    NaiveDate::parse_from_str(ds, "%Y-%m-%d")
        .map_err(|e| SnapshotError::BadRow(format!("ds {ds:?}: {e}")))
}

/// Every row of one run: the newest `ds`, newest `run_id` within it.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    /// Day the snapshot covers.
    pub ds: NaiveDate,
    /// When the run measured.
    pub measured_at: DateTime<Utc>,
    /// Run that wrote the rows.
    pub run_id: String,
    /// All rows of the run.
    pub rows: Vec<StorageRow>,
}

impl Snapshot {
    /// The site total row (`site`, `all`, `*`, `*`).
    #[must_use]
    pub fn site_total(&self) -> Option<&StorageRow> {
        self.owner_total(&Owner::site())
    }

    /// Site rows split by File table.
    #[must_use]
    pub fn site_by_app(&self) -> Vec<&StorageRow> {
        self.owner_by_app(&Owner::site())
    }

    /// Site rows split by scan status.
    #[must_use]
    pub fn site_by_status(&self) -> Vec<&StorageRow> {
        let site = Owner::site();
        self.rows
            .iter()
            .filter(|r| r.owner == site && r.is_by_status())
            .collect()
    }

    /// One owner's total row; `None` when the owner had no files at run time.
    #[must_use]
    pub fn owner_total(&self, owner: &Owner) -> Option<&StorageRow> {
        self.rows.iter().find(|r| &r.owner == owner && r.is_total())
    }

    /// One owner's rows split by File table.
    #[must_use]
    pub fn owner_by_app(&self, owner: &Owner) -> Vec<&StorageRow> {
        self.rows
            .iter()
            .filter(|r| &r.owner == owner && r.is_by_app())
            .collect()
    }

    /// Every non-site owner's total row (people, service users, teams).
    #[must_use]
    pub fn people(&self) -> Vec<&StorageRow> {
        self.rows
            .iter()
            .filter(|r| !r.owner.is_site() && r.is_total())
            .collect()
    }

    /// Copies the blob store reported, from the site total.
    #[must_use]
    pub fn copies_reported(&self) -> Option<i64> {
        self.site_total().and_then(|r| r.copies_reported)
    }
}

fn equals(field: &str, value: &str) -> GridFilterItem {
    GridFilterItem {
        field: field.to_string(),
        operator: GridFilterOperator::Equals,
        value: json!(value),
    }
}

fn filter_of(items: Vec<GridFilterItem>) -> GridFilterModel {
    GridFilterModel {
        items,
        ..Default::default()
    }
}

/// Read the newest snapshot, or `Ok(None)` when the table has no rows.
///
/// # Errors
///
/// [`SnapshotError::Spectra`] when a query fails; [`SnapshotError::BadRow`]
/// when a row breaks the contract in [`super::schema`].
pub async fn latest_snapshot(router: &SpectraRouter) -> Result<Option<Snapshot>, SnapshotError> {
    // Step 1: the newest row names the day and the run to read.
    let newest = router
        .query_events(EventsQueryFilter {
            table: TABLE.to_string(),
            sort_field: Some("ds".to_string()),
            sort_desc: true,
            limit: Some(1),
            ..Default::default()
        })
        .await?;
    let Some(newest) = newest.first() else {
        return Ok(None);
    };
    let newest = StorageRow::from_event(newest)?;
    let ds = newest.ds.format("%Y-%m-%d").to_string();

    // Step 2: every row of that run, a page at a time.
    let mut rows = Vec::new();
    let mut offset = 0u32;
    loop {
        let page = router
            .query_events(EventsQueryFilter {
                table: TABLE.to_string(),
                sort_field: Some("owner_id".to_string()),
                limit: Some(PAGE_SIZE),
                offset: Some(offset),
                filter: filter_of(vec![equals("ds", &ds), equals("run_id", &newest.run_id)]),
                ..Default::default()
            })
            .await?;
        let got = u32::try_from(page.len()).unwrap_or(u32::MAX);
        for row in &page {
            rows.push(StorageRow::from_event(row)?);
        }
        if got < PAGE_SIZE {
            break;
        }
        offset = offset.saturating_add(got);
    }

    Ok(Some(Snapshot {
        ds: newest.ds,
        measured_at: newest.measured_at,
        run_id: newest.run_id,
        rows,
    }))
}

/// Daily totals for `owner` over the last `days` partitions, oldest first.
///
/// One point per `ds`; when a day has more than one run, the newest wins.
/// Returns an empty list when the table has no rows for the owner.
///
/// # Errors
///
/// [`SnapshotError::Spectra`] when the query fails; [`SnapshotError::BadRow`]
/// when a row breaks the contract.
pub async fn history(
    router: &SpectraRouter,
    owner: &Owner,
    days: usize,
) -> Result<Vec<(NaiveDate, i64)>, SnapshotError> {
    let rows = router
        .query_events(EventsQueryFilter {
            table: TABLE.to_string(),
            sort_field: Some("ds".to_string()),
            sort_desc: true,
            limit: Some(PAGE_SIZE),
            filter: filter_of(vec![
                equals("owner_kind", &owner.kind),
                equals("owner_id", &owner.id),
                equals("app_table", ALL),
                equals("file_status", ALL),
            ]),
            ..Default::default()
        })
        .await?;

    // Sorted by ds then ts, both descending: the first row per ds is its newest run.
    let mut points: Vec<(NaiveDate, i64)> = Vec::new();
    for row in &rows {
        let row = StorageRow::from_event(row)?;
        if points.last().is_some_and(|(ds, _)| *ds == row.ds) {
            continue;
        }
        if points.len() == days {
            break;
        }
        points.push((row.ds, row.bytes));
    }
    points.reverse();
    Ok(points)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use crate::storage::test_support::{mem_router, write_row, RowSpec};

    #[tokio::test]
    async fn no_table_is_none() {
        let (router, _) = mem_router();
        assert_eq!(latest_snapshot(&router).await.expect("query"), None);
        assert!(history(&router, &Owner::site(), 30)
            .await
            .expect("query")
            .is_empty());
    }

    #[tokio::test]
    async fn two_partitions_return_only_the_latest() {
        let (router, events) = mem_router();
        write_row(&events, RowSpec::site_total("2026-11-22", "run-a", 100)).await;
        write_row(&events, RowSpec::site_total("2026-11-23", "run-b", 300)).await;
        write_row(
            &events,
            RowSpec::owner_total("2026-11-22", "run-a", "u1", 40),
        )
        .await;
        write_row(
            &events,
            RowSpec::owner_total("2026-11-23", "run-b", "u1", 70),
        )
        .await;

        let snap = latest_snapshot(&router)
            .await
            .expect("query")
            .expect("some");
        assert_eq!(snap.ds, NaiveDate::from_ymd_opt(2026, 11, 23).unwrap());
        assert_eq!(snap.run_id, "run-b");
        assert_eq!(snap.rows.len(), 2);
        assert_eq!(snap.site_total().map(|r| r.bytes), Some(300));
        let u1 = Owner {
            kind: "user".into(),
            id: "u1".into(),
        };
        assert_eq!(snap.owner_total(&u1).map(|r| r.bytes), Some(70));
    }

    #[tokio::test]
    async fn rerun_on_same_day_reads_newest_run() {
        let (router, events) = mem_router();
        let mut first = RowSpec::site_total("2026-11-23", "run-a", 100);
        first.hours_after_midnight = 3;
        let mut second = RowSpec::site_total("2026-11-23", "run-b", 250);
        second.hours_after_midnight = 9;
        write_row(&events, first).await;
        write_row(&events, second).await;

        let snap = latest_snapshot(&router)
            .await
            .expect("query")
            .expect("some");
        assert_eq!(snap.run_id, "run-b");
        assert_eq!(snap.rows.len(), 1);
        let points = history(&router, &Owner::site(), 30).await.expect("query");
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].1, 250);
    }

    #[tokio::test]
    async fn reads_2500_owner_rows_across_pages() {
        let (router, events) = mem_router();
        write_row(&events, RowSpec::site_total("2026-11-23", "run-a", 2_500)).await;
        for i in 0..2_500 {
            write_row(
                &events,
                RowSpec::owner_total("2026-11-23", "run-a", &format!("u{i:05}"), 1),
            )
            .await;
        }

        let snap = latest_snapshot(&router)
            .await
            .expect("query")
            .expect("some");
        assert_eq!(snap.rows.len(), 2_501);
        assert_eq!(snap.people().len(), 2_500);
        let mut ids: Vec<&str> = snap.people().iter().map(|r| r.owner.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 2_500, "no row read twice");
    }

    #[tokio::test]
    async fn accessors_split_rows_by_shape() {
        let (router, events) = mem_router();
        let ds = "2026-11-23";
        let mut total = RowSpec::site_total(ds, "r", 30);
        total.copies_reported = 3;
        write_row(&events, total).await;
        write_row(&events, RowSpec::site_by_app(ds, "r", "receipt_scan", 20)).await;
        write_row(&events, RowSpec::site_by_app(ds, "r", "profile_photo", 10)).await;
        write_row(&events, RowSpec::site_by_status(ds, "r", "quarantined", 5)).await;
        write_row(&events, RowSpec::owner_total(ds, "r", "u1", 30)).await;
        write_row(
            &events,
            RowSpec::owner_by_app(ds, "r", "u1", "receipt_scan", 20),
        )
        .await;

        let snap = latest_snapshot(&router)
            .await
            .expect("query")
            .expect("some");
        let u1 = Owner {
            kind: "user".into(),
            id: "u1".into(),
        };
        assert_eq!(snap.site_by_app().len(), 2);
        assert_eq!(snap.site_by_status().len(), 1);
        assert_eq!(snap.owner_by_app(&u1).len(), 1);
        assert_eq!(snap.people().len(), 1);
        assert_eq!(snap.copies_reported(), Some(3));
    }

    #[tokio::test]
    async fn history_is_oldest_first_and_capped() {
        let (router, events) = mem_router();
        for day in 1..=5 {
            let ds = format!("2026-11-{day:02}");
            write_row(&events, RowSpec::owner_total(&ds, &ds, "u1", day * 10)).await;
        }
        let u1 = Owner {
            kind: "user".into(),
            id: "u1".into(),
        };
        let points = history(&router, &u1, 3).await.expect("query");
        let bytes: Vec<i64> = points.iter().map(|(_, b)| *b).collect();
        assert_eq!(bytes, vec![30, 40, 50]);
    }

    #[tokio::test]
    async fn bad_ds_is_an_error() {
        let (router, events) = mem_router();
        write_row(&events, RowSpec::site_total("23/11/2026", "r", 1)).await;
        assert!(matches!(
            latest_snapshot(&router).await,
            Err(SnapshotError::BadRow(_))
        ));
    }
}
