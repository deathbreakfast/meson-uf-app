//! Valence-backed integration tests for meson-app's My Files service logic.

#![cfg(feature = "ssr")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod common;

use common::{as_user, owner_rid, peer_rid, seed_file, setup_valence, OWNER_USER_ID};
use meson_app::server::{get_file_for_user, list_files_for_user, preview_file_for_user};

/// Owner can fetch their own file by id.
#[tokio::test]
async fn meson_app_get_file_for_user_owner_happy() {
    let system = setup_valence().await;
    let file_id = seed_file(
        &system,
        "owner-1",
        owner_rid(),
        "receipt.png",
        "image/png",
        "owner-1.png",
        2048,
    )
    .await;
    let session = as_user(&system, OWNER_USER_ID);

    let row = get_file_for_user(&session, owner_rid(), &file_id)
        .await
        .expect("owner reads own file");
    assert_eq!(row.file_name, "receipt.png");
}

/// A peer's file id collapses to the same `not_found:` error as a missing id (IDOR-safe).
#[tokio::test]
async fn meson_app_get_file_for_user_peer_id_sad() {
    let system = setup_valence().await;
    let peer_file_id = seed_file(
        &system,
        "peer-1",
        peer_rid(),
        "peer-notes.txt",
        "text/plain",
        "peer-1.txt",
        128,
    )
    .await;
    let session = as_user(&system, OWNER_USER_ID);

    let peer_err = get_file_for_user(&session, owner_rid(), &peer_file_id)
        .await
        .expect_err("peer id must not resolve for a different owner");
    let missing_err = get_file_for_user(&session, owner_rid(), "e2e_meson_file:does-not-exist")
        .await
        .expect_err("missing id must not resolve");

    assert!(peer_err.to_string().contains("not_found:"), "{peer_err}");
    assert!(
        missing_err.to_string().contains("not_found:"),
        "{missing_err}"
    );
    assert_eq!(
        peer_err.to_string(),
        missing_err.to_string(),
        "foreign and missing ids must collapse to the identical wire error"
    );
}

/// Listing only returns rows owned by the requesting user.
#[tokio::test]
async fn meson_app_list_files_for_user_scoped_happy() {
    let system = setup_valence().await;
    seed_file(
        &system,
        "owner-2",
        owner_rid(),
        "owned.png",
        "image/png",
        "owner-2.png",
        4096,
    )
    .await;
    seed_file(
        &system,
        "peer-2",
        peer_rid(),
        "not-owned.png",
        "image/png",
        "peer-2.png",
        4096,
    )
    .await;
    let session = as_user(&system, OWNER_USER_ID);

    let rows = list_files_for_user(&session, owner_rid())
        .await
        .expect("list scoped to owner");
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].file_name, "owned.png");
}

/// Preview rejects an oversized file before ever touching the blob store.
#[tokio::test]
async fn meson_app_preview_file_for_user_size_guard_sad() {
    const OVER_LIMIT: i64 = 5 * 1024 * 1024 + 1;
    let system = setup_valence().await;
    let file_id = seed_file(
        &system,
        "huge-1",
        owner_rid(),
        "huge.png",
        "image/png",
        "huge-1.png",
        OVER_LIMIT,
    )
    .await;
    let session = as_user(&system, OWNER_USER_ID);

    let err = preview_file_for_user(&session, owner_rid(), &file_id)
        .await
        .expect_err("oversized metadata must reject before blob fetch");
    assert!(
        err.to_string().contains("exceeds preview size limit"),
        "{err}"
    );
}
