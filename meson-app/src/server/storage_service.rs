//! Testable Storage logic: snapshot + settings + history into a view.
//!
//! Server fns in [`super::storage`] recover the session, the installed
//! Spectra router and the StorageAdmin check, then call these.

use chrono::{DateTime, Utc};
use spectra::SpectraRouter;
use valence::{Model, RecordId, Valence};

use super::helpers::{forbidden_error, MesonAppError};
use crate::generated::MesonStorageSettings;
use crate::storage::settings::{validate_settings, StorageSettings, DEFAULT_CURRENCY, SETTINGS_ID};
use crate::storage::snapshot::{history, latest_snapshot, Owner, Snapshot, StorageRow};
use crate::storage::view::{
    build_people, build_view, HistoryState, SiteStorageView, SizeSource, StorageView, Usage,
    ViewInput, HISTORY_DAYS,
};

fn usage(row: &StorageRow) -> Usage {
    Usage {
        files: row.files,
        bytes: row.bytes,
        missing_size_files: row.missing_size_files,
    }
}

/// What came back from Spectra for one owner.
enum Read {
    /// No router installed, or the query failed.
    Down,
    /// The table has no rows yet.
    Empty,
    /// The newest snapshot and the owner's history.
    Found(Box<Snapshot>, HistoryState),
}

async fn read(router: Option<&SpectraRouter>, owner: &Owner) -> Read {
    let Some(router) = router else {
        tracing::warn!(
            outcome = "spectra_not_installed",
            "storage snapshot unavailable"
        );
        return Read::Down;
    };
    match latest_snapshot(router).await {
        Ok(Some(snapshot)) => {
            let history = match history(router, owner, HISTORY_DAYS).await {
                Ok(points) if points.is_empty() => HistoryState::NoneYet,
                Ok(points) => HistoryState::Points(points),
                Err(e) => {
                    tracing::error!(error = %e, "storage history read failed");
                    HistoryState::Down
                }
            };
            Read::Found(Box::new(snapshot), history)
        }
        Ok(None) => Read::Empty,
        Err(e) => {
            tracing::error!(error = %e, "storage snapshot read failed");
            Read::Down
        }
    }
}

fn input_for(read: &Read, owner: &Owner, now: DateTime<Utc>) -> ViewInput {
    match read {
        Read::Found(snapshot, history) => ViewInput {
            size: SizeSource::for_run(snapshot.ds, snapshot.measured_at, now),
            total: snapshot.owner_total(owner).map(usage).unwrap_or_default(),
            by_app: snapshot
                .owner_by_app(owner)
                .into_iter()
                .map(|r| (r.app_table.clone(), usage(r)))
                .collect(),
            copies_reported: snapshot.copies_reported(),
            history: history.clone(),
        },
        // TODO(mes-lib-scale): sum live with `meson::storage_summary` (Some(user)
        // for a member, None for the site) and return `SizeSource::Live`.
        Read::Empty => ViewInput {
            size: SizeSource::Unavailable,
            total: Usage::default(),
            by_app: Vec::new(),
            copies_reported: None,
            history: HistoryState::NoneYet,
        },
        Read::Down => ViewInput {
            size: SizeSource::Unavailable,
            total: Usage::default(),
            by_app: Vec::new(),
            copies_reported: None,
            history: HistoryState::Down,
        },
    }
}

/// The signed-in member's own storage.
///
/// # Errors
///
/// `io:` when the settings row can't be read. Spectra failures don't error:
/// they show up as [`SizeSource::Unavailable`] / [`HistoryState::Down`].
pub async fn my_storage(
    router: Option<&SpectraRouter>,
    v: &Valence,
    user_rid: &RecordId,
    now: DateTime<Utc>,
) -> Result<StorageView, MesonAppError> {
    let settings = storage_settings(v).await?;
    let owner = Owner::from_record(user_rid);
    let read = read(router, &owner).await;
    Ok(build_view(input_for(&read, &owner, now), &settings))
}

/// Site-wide storage for site owners.
///
/// # Errors
///
/// `forbidden:` unless `is_storage_admin`; `io:` when settings can't be read.
pub async fn site_storage(
    router: Option<&SpectraRouter>,
    v: &Valence,
    is_storage_admin: bool,
    now: DateTime<Utc>,
) -> Result<SiteStorageView, MesonAppError> {
    if !is_storage_admin {
        return Err(forbidden_error("StorageAdmin permission required"));
    }
    let settings = storage_settings(v).await?;
    let site = Owner::site();
    let read = read(router, &site).await;
    let view = build_view(input_for(&read, &site, now), &settings);
    let (people, run_id, measured_at) = match &read {
        Read::Found(snapshot, _) => {
            let owners = snapshot
                .people()
                .into_iter()
                .map(|r| (r.owner.kind.clone(), r.owner.id.clone(), usage(r)))
                .collect();
            (
                build_people(owners, view.totals.bytes, &view),
                Some(snapshot.run_id.clone()),
                Some(snapshot.measured_at),
            )
        }
        Read::Empty | Read::Down => (Vec::new(), None, None),
    };
    Ok(SiteStorageView {
        view,
        people,
        run_id,
        measured_at,
    })
}

/// The site's cost settings, or the defaults when none are saved.
///
/// # Errors
///
/// `io:` when Valence fails.
pub async fn storage_settings(v: &Valence) -> Result<StorageSettings, MesonAppError> {
    let row = MesonStorageSettings::get(SETTINGS_ID, v, valence::use_!(r"To work out **how much your storage costs**, the Storage page **loads the site's storage cost settings**: the number of copies a site owner typed in, the **price per GB each month** and its currency. The page shows them to signed-in members next to the estimate."))
        .await
        .map_err(|e| MesonAppError::io_source("storage settings read failed", e))?;
    Ok(
        row.map_or_else(StorageSettings::default, |row| StorageSettings {
            copies_override: row.copies_override().copied(),
            price_per_gb_month: row.price_per_gb_month().copied(),
            currency: row.currency().clone(),
            updated_by: row.updated_by().map(ToString::to_string),
            updated_at: Some(*row.updated_at()),
        }),
    )
}

/// Save the copies override and price.
///
/// # Errors
///
/// `forbidden:` unless `is_storage_admin`; `invalid:` when copies is outside
/// 1..=10 or the price is negative; `io:` when Valence fails.
pub async fn save_settings(
    v: &Valence,
    is_storage_admin: bool,
    updated_by: RecordId,
    copies_override: Option<i64>,
    price_per_gb_month: Option<f64>,
    now: DateTime<Utc>,
) -> Result<StorageSettings, MesonAppError> {
    if !is_storage_admin {
        return Err(forbidden_error("StorageAdmin permission required"));
    }
    validate_settings(copies_override, price_per_gb_month).map_err(MesonAppError::invalid)?;
    let currency = storage_settings(v).await?.currency;
    let currency = if currency.is_empty() {
        DEFAULT_CURRENCY.to_string()
    } else {
        currency
    };
    let row = MesonStorageSettings::new(
        copies_override,
        price_per_gb_month,
        currency,
        Some(updated_by),
        now,
    )
    .map_err(|e| MesonAppError::io_source("storage settings invalid", e))?;
    MesonStorageSettings::upsert(SETTINGS_ID, row, v, valence::use_!(r"When a site owner **changes the storage cost settings**, we **save the copies and price per GB they entered**, with **who saved them and when**. Every signed-in member's Storage page uses these numbers for its cost estimate, and site owners see who changed them last."))
        .await
        .map_err(|e| MesonAppError::io_source("storage settings save failed", e))?;
    storage_settings(v).await
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use chrono::Duration;

    use super::*;
    use crate::storage::test_support::{mem_router, setup_valence, write_row, RowSpec};
    use crate::storage::view::{CopiesSource, PriceSource};

    fn user() -> RecordId {
        RecordId::new("user", "u1")
    }

    fn as_user(v: &Valence) -> Valence {
        v.with_actor(valence::Actor::User {
            user_id: "u1".to_string(),
        })
    }

    fn now() -> DateTime<Utc> {
        "2026-11-23T12:00:00Z".parse().unwrap()
    }

    #[tokio::test]
    async fn no_table_gives_unavailable_view() {
        let system = setup_valence().await;
        let (router, _) = mem_router();
        let view = my_storage(Some(&router), &as_user(&system), &user(), now())
            .await
            .expect("view");
        // TODO(mes-lib-scale): Live once `storage_summary` lands.
        assert_eq!(view.size, SizeSource::Unavailable);
        assert_eq!(view.history, HistoryState::NoneYet);
        assert_eq!(view.copies, CopiesSource::Assumed);
        assert_eq!(view.price, PriceSource::Unset);
        assert_eq!(view.totals.bytes, 0);
    }

    #[tokio::test]
    async fn no_router_is_down() {
        let system = setup_valence().await;
        let view = my_storage(None, &as_user(&system), &user(), now())
            .await
            .expect("view");
        assert_eq!(view.size, SizeSource::Unavailable);
        assert_eq!(view.history, HistoryState::Down);
    }

    #[tokio::test]
    async fn member_sees_own_rows_and_stale_flag() {
        let system = setup_valence().await;
        let (router, events) = mem_router();
        let ds = "2026-11-21";
        let mut total = RowSpec::site_total(ds, "r", 9_000);
        total.copies_reported = 3;
        write_row(&events, total).await;
        let mut mine = RowSpec::owner_total(ds, "r", "u1", 1_000);
        mine.missing_size_files = 2;
        write_row(&events, mine).await;
        write_row(
            &events,
            RowSpec::owner_by_app(ds, "r", "u1", "receipt_scan", 1_000),
        )
        .await;
        write_row(&events, RowSpec::owner_total(ds, "r", "u2", 8_000)).await;

        // RowSpec measures at 03:00 on ds; 2026-11-21T03:00 is 57 h before now().
        let view = my_storage(Some(&router), &as_user(&system), &user(), now())
            .await
            .expect("view");
        assert!(
            matches!(view.size, SizeSource::Stale { .. }),
            "{:?}",
            view.size
        );
        assert_eq!(view.totals.bytes, 1_000);
        assert_eq!(view.totals.bytes_with_copies, 3_000);
        assert_eq!(view.missing_size_files, 2);
        assert_eq!(view.copies, CopiesSource::Reported(3));
        assert_eq!(view.by_app.len(), 1);
        assert!(matches!(view.history, HistoryState::Points(ref p) if p.len() == 1));

        let fresh = my_storage(
            Some(&router),
            &as_user(&system),
            &user(),
            "2026-11-21T20:00:00Z".parse().unwrap(),
        )
        .await
        .expect("view");
        assert!(matches!(fresh.size, SizeSource::Measured { .. }));
    }

    #[tokio::test]
    async fn non_admin_gets_forbidden_from_site_fns() {
        let system = setup_valence().await;
        let (router, _) = mem_router();
        let session = as_user(&system);

        let err = site_storage(Some(&router), &session, false, now())
            .await
            .expect_err("non-admin");
        assert!(err.to_string().starts_with("forbidden:"), "{err}");

        let err = save_settings(&session, false, user(), Some(2), Some(0.02), now())
            .await
            .expect_err("non-admin");
        assert!(err.to_string().starts_with("forbidden:"), "{err}");
    }

    #[tokio::test]
    async fn invalid_settings_are_rejected() {
        let system = setup_valence().await;
        let session = as_user(&system);
        for (copies, price) in [(Some(0), None), (Some(11), None), (None, Some(-1.0))] {
            let err = save_settings(&session, true, user(), copies, price, now())
                .await
                .expect_err("invalid");
            assert!(err.to_string().starts_with("invalid:"), "{err}");
        }
    }

    #[tokio::test]
    async fn admin_saves_settings_and_site_view_uses_them() {
        let system = setup_valence().await;
        let (router, events) = mem_router();
        let ds = "2026-11-23";
        let mut total = RowSpec::site_total(ds, "run-7", 4_000_000_000);
        total.copies_reported = 3;
        write_row(&events, total).await;
        write_row(
            &events,
            RowSpec::site_by_app(ds, "run-7", "receipt_scan", 4_000_000_000),
        )
        .await;
        write_row(
            &events,
            RowSpec::owner_total(ds, "run-7", "u1", 1_000_000_000),
        )
        .await;
        write_row(
            &events,
            RowSpec::owner_total(ds, "run-7", "u2", 3_000_000_000),
        )
        .await;
        let session = as_user(&system);

        assert_eq!(
            storage_settings(&session).await.expect("read"),
            StorageSettings::default()
        );
        let saved = save_settings(&session, true, user(), Some(2), Some(0.5), now())
            .await
            .expect("save");
        assert_eq!(saved.copies_override, Some(2));
        assert_eq!(saved.price_per_gb_month, Some(0.5));
        assert_eq!(saved.currency, "USD");
        assert_eq!(saved.updated_by.as_deref(), Some("user:u1"));

        let site = site_storage(Some(&router), &session, true, now())
            .await
            .expect("site");
        assert_eq!(site.view.copies, CopiesSource::Set(2));
        assert_eq!(site.view.totals.cost_per_month, Some(4.0));
        assert_eq!(site.run_id.as_deref(), Some("run-7"));
        assert!(site.measured_at.is_some());
        assert_eq!(site.people.len(), 2);
        assert_eq!(site.people[0].owner_id, "u2");
        assert_eq!(site.people[0].share, 0.75);
        assert_eq!(site.people[0].cost_per_month, Some(3.0));

        // Clearing the override falls back to what storage reports.
        save_settings(
            &session,
            true,
            user(),
            None,
            None,
            now() + Duration::minutes(1),
        )
        .await
        .expect("save");
        let site = site_storage(Some(&router), &session, true, now())
            .await
            .expect("site");
        assert_eq!(site.view.copies, CopiesSource::Reported(3));
        assert_eq!(site.view.totals.cost_per_month, None);
    }
}
