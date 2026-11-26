//! Process-wide Valence + Higgs for Playwright (meson mem + sqlite alias).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use chrono::{DateTime, Duration, Utc};
use higgs::actor_policy::external_actor_json_policy;
use higgs::{HiggsConfig, HiggsValenceFactory};
use meson::generated::{E2eMesonFile, E2eMesonOwnedFile, FileFileStatus};
use meson::touch_schema_inventory;
use valence::{
    register_backend_logical_names, router_key, Actor, DatabaseBackend, DatabaseRouter,
    InMemoryBackend, Model, RecordId, RegisterBackendLogicalNamesOptions, RouterValenceFactory,
    RouterValenceFactoryConfig, Valence, ValenceFactory, MEM_ENGINE_ID, SQLITE_ENGINE_ID,
};

/// 8x8 solid-blue PNG served for the seeded image files.
pub const E2E_PNG_BYTES: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 8, 0, 0, 0, 8, 8, 2, 0,
    0, 0, 75, 109, 41, 220, 0, 0, 0, 17, 73, 68, 65, 84, 120, 156, 99, 208, 168, 56, 129, 21, 49,
    12, 45, 9, 0, 2, 213, 90, 1, 196, 145, 123, 173, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

/// Body of the seeded `notes.txt`; `meson.spec.ts` asserts these exact lines.
pub const E2E_TEXT_BYTES: &[u8] = b"Meson preview fixture\nline two: seeded text\n";

struct E2eState {
    router: Arc<DatabaseRouter>,
    higgs: Arc<HiggsConfig>,
    default_backend_key: String,
    fixtures: Mutex<FixtureIds>,
}

/// Stable fixture ids exposed to seed JSON / Playwright.
// Field names mirror the `seed-data` JSON response / TS `SeedFixtures` type;
// dropping the shared `_file_id` suffix would desync that wire contract.
#[allow(clippy::struct_field_names)]
#[derive(Clone, Debug, Default)]
pub struct FixtureIds {
    pub owner_image_file_id: String,
    pub owner_text_file_id: String,
    pub peer_file_id: String,
    pub owner_scanning_file_id: String,
    pub owner_slow_scan_file_id: String,
    pub owner_quarantined_file_id: String,
    pub owner_big_file_id: String,
    pub owner_pdf_file_id: String,
}

/// Size the "too big to preview" fixture claims (6 MB, over the 5 MB preview cap).
const BIG_FILE_SIZE: i64 = 6_291_456;

static E2E_STATE: OnceLock<Arc<E2eState>> = OnceLock::new();

struct HiggsFactory(RouterValenceFactory);

impl HiggsValenceFactory for HiggsFactory {
    fn build(&self, actor_json: &serde_json::Value) -> anyhow::Result<Valence> {
        self.0.build(actor_json).map_err(|e| anyhow::anyhow!("{e}"))
    }
}

fn prepare_env() {
    valence::deletion::register_noop_deletion_dispatcher_for_tests();
    valence::clear_for_test();
    // SAFETY: host boot only.
    unsafe {
        if std::env::var_os("VALENCE_OWNERSHIP_UNIFIED_FETCH").is_none() {
            std::env::set_var("VALENCE_OWNERSHIP_UNIFIED_FETCH", "0");
        }
    }
}

fn owner_rid() -> RecordId {
    RecordId::new("user", "owner")
}

fn peer_rid() -> RecordId {
    RecordId::new("user", "peer")
}

/// One fixture row.
struct SeedSpec<'a> {
    id: &'a str,
    uploader: RecordId,
    file_name: &'a str,
    mime: &'a str,
    storage_path: &'a str,
    bytes: &'a [u8],
    /// Size recorded on the row when it differs from `bytes` (the too-big fixture).
    size_bytes: Option<i64>,
    status: FileFileStatus,
    uploaded_at: DateTime<Utc>,
}

impl<'a> SeedSpec<'a> {
    fn available(
        id: &'a str,
        uploader: RecordId,
        file_name: &'a str,
        mime: &'a str,
        storage_path: &'a str,
        bytes: &'a [u8],
    ) -> Self {
        Self {
            id,
            uploader,
            file_name,
            mime,
            storage_path,
            bytes,
            size_bytes: None,
            status: FileFileStatus::Available,
            uploaded_at: Utc::now(),
        }
    }
}

/// Store the bytes and return `(extension, size)` for the row.
async fn put_bytes(spec: &SeedSpec<'_>) -> (String, i64) {
    meson::installed_blob_store()
        .expect("e2e blob store installed before seeding")
        .put(spec.storage_path, spec.bytes)
        .await
        .expect("put fixture bytes");
    let size_bytes = spec
        .size_bytes
        .unwrap_or_else(|| i64::try_from(spec.bytes.len()).expect("fixture length fits i64"));
    let extension = spec
        .file_name
        .rsplit('.')
        .next()
        .unwrap_or("bin")
        .to_string();
    (extension, size_bytes)
}

async fn seed_file(valence: &Valence, spec: SeedSpec<'_>) -> String {
    let (extension, size_bytes) = put_bytes(&spec).await;
    let row = E2eMesonFile::new(
        spec.file_name.to_string(),
        extension,
        spec.mime.to_string(),
        size_bytes,
        spec.storage_path.to_string(),
        spec.status,
        spec.uploader,
        spec.uploaded_at,
    )
    .expect("E2eMesonFile::new");
    let created = E2eMesonFile::upsert(spec.id, row, valence, valence::use_!(r"**Test:** Fixture **E2e Meson File** save for `e2e_valence` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."))
        .await
        .expect("upsert e2e_meson_file");
    created
        .id()
        .map_or_else(|| spec.id.to_string(), std::string::ToString::to_string)
}

/// A row in `e2e_meson_owned_file`, which its uploader may delete; its app
/// refuses names starting with `keep-`.
async fn seed_owned_file(valence: &Valence, spec: SeedSpec<'_>) -> String {
    let (extension, size_bytes) = put_bytes(&spec).await;
    let row = E2eMesonOwnedFile::new(
        spec.file_name.to_string(),
        extension,
        spec.mime.to_string(),
        size_bytes,
        spec.storage_path.to_string(),
        spec.status,
        spec.uploader,
        spec.uploaded_at,
    )
    .expect("E2eMesonOwnedFile::new");
    let created = E2eMesonOwnedFile::upsert(spec.id, row, valence, valence::use_!(r"**Test:** Fixture **E2e Meson Owned File** save for `e2e_valence` so delete specs get their own rows to delete. CI and developers running the suite only."))
        .await
        .expect("upsert e2e_meson_owned_file");
    created
        .id()
        .map_or_else(|| spec.id.to_string(), std::string::ToString::to_string)
}

/// New deletable rows for one spec, so delete specs never share rows.
///
/// `plain` is a clean text file, `quarantined` a flagged image, and `keep` a
/// clean image its app refuses to delete. Unknown kinds are skipped.
pub async fn seed_fresh_files(kinds: &[String]) -> Vec<(String, String)> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let system = e2e_system_valence();
    let mut out = Vec::new();
    for kind in kinds {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let id = format!("fresh-{kind}-{n}");
        let path = format!("{id}.bin");
        let spec = match kind.as_str() {
            "plain" => SeedSpec::available(
                &id,
                owner_rid(),
                "delete-me.txt",
                "text/plain",
                &path,
                E2E_TEXT_BYTES,
            ),
            "quarantined" => SeedSpec {
                status: FileFileStatus::Quarantined,
                ..SeedSpec::available(
                    &id,
                    owner_rid(),
                    "flagged-statement.png",
                    "image/png",
                    &path,
                    E2E_PNG_BYTES,
                )
            },
            "keep" => SeedSpec::available(
                &id,
                owner_rid(),
                "keep-receipt.png",
                "image/png",
                &path,
                E2E_PNG_BYTES,
            ),
            _ => continue,
        };
        let created = seed_owned_file(&system, spec).await;
        out.push((kind.clone(), created));
    }
    out
}

/// Build shared Valence/Higgs once and seed baseline fixtures.
pub async fn init_e2e_valence() {
    if E2E_STATE.get().is_some() {
        return;
    }

    prepare_env();
    touch_schema_inventory();

    let blob_root =
        std::env::temp_dir().join(format!("meson-uf-app-e2e-blobs-{}", std::process::id()));
    meson::install_blob_store(Arc::new(meson::LocalDiskBlobStore::new(blob_root)))
        .expect("install e2e blob store");

    let backend: Arc<dyn DatabaseBackend> = Arc::new(InMemoryBackend::new());
    let mut router = DatabaseRouter::new();
    register_backend_logical_names(
        &mut router,
        Arc::clone(&backend),
        meson::embedded_surreal::EMBEDDED_SURREAL_LOGICAL_NAMES,
        RegisterBackendLogicalNamesOptions {
            register_alias_engine_id: Some(SQLITE_ENGINE_ID),
        },
    );
    router.register(
        router_key(
            meson::embedded_surreal::DEFAULT_LOGICAL_NAME,
            SQLITE_ENGINE_ID,
        ),
        backend,
    );
    let router = Arc::new(router);
    let default_key = router_key(meson::embedded_surreal::DEFAULT_LOGICAL_NAME, MEM_ENGINE_ID);

    let system = Valence::builder()
        .database_router(Arc::clone(&router))
        .default_backend_key(default_key.clone())
        .with_actor(Actor::System {
            operation: "e2e_meson_host".into(),
        })
        .build()
        .expect("e2e Valence");
    system
        .sync_typed_tables_from_registry()
        .await
        .expect("sync_typed_tables_from_registry");

    let owner_image_file_id = seed_file(
        &system,
        SeedSpec::available(
            "owner-image",
            owner_rid(),
            "receipt.png",
            "image/png",
            "owner-image.png",
            E2E_PNG_BYTES,
        ),
    )
    .await;
    let owner_text_file_id = seed_file(
        &system,
        SeedSpec::available(
            "owner-text",
            owner_rid(),
            "notes.txt",
            "text/plain",
            "owner-text.txt",
            E2E_TEXT_BYTES,
        ),
    )
    .await;
    let peer_file_id = seed_file(
        &system,
        SeedSpec::available(
            "peer-file",
            peer_rid(),
            "peer-only.png",
            "image/png",
            "peer-file.png",
            E2E_PNG_BYTES,
        ),
    )
    .await;
    let owner_scanning_file_id = seed_file(
        &system,
        SeedSpec {
            status: FileFileStatus::PendingVirusScan,
            uploaded_at: Utc::now() - Duration::minutes(1),
            ..SeedSpec::available(
                "owner-scanning",
                owner_rid(),
                "hardware-store.jpg",
                "image/jpeg",
                "owner-scanning.jpg",
                E2E_PNG_BYTES,
            )
        },
    )
    .await;
    let owner_slow_scan_file_id = seed_file(
        &system,
        SeedSpec {
            status: FileFileStatus::PendingVirusScan,
            uploaded_at: Utc::now() - Duration::hours(3),
            ..SeedSpec::available(
                "owner-slow-scan",
                owner_rid(),
                "slow-scan.jpg",
                "image/jpeg",
                "owner-slow-scan.jpg",
                E2E_PNG_BYTES,
            )
        },
    )
    .await;
    let owner_quarantined_file_id = seed_file(
        &system,
        SeedSpec {
            status: FileFileStatus::Quarantined,
            ..SeedSpec::available(
                "owner-quarantined",
                owner_rid(),
                "statement-sept.png",
                "image/png",
                "owner-quarantined.png",
                E2E_PNG_BYTES,
            )
        },
    )
    .await;
    let owner_big_file_id = seed_file(
        &system,
        SeedSpec {
            size_bytes: Some(BIG_FILE_SIZE),
            ..SeedSpec::available(
                "owner-big",
                owner_rid(),
                "ledger-export-2026-10.log",
                "text/plain",
                "owner-big.log",
                E2E_TEXT_BYTES,
            )
        },
    )
    .await;
    let owner_pdf_file_id = seed_file(
        &system,
        SeedSpec::available(
            "owner-pdf",
            owner_rid(),
            "lease-2026.pdf",
            "application/pdf",
            "owner-pdf.pdf",
            b"%PDF-1.4\n%fixture\n",
        ),
    )
    .await;

    let factory: Arc<dyn HiggsValenceFactory> = Arc::new(HiggsFactory(RouterValenceFactory::new(
        Arc::clone(&router),
        RouterValenceFactoryConfig::new(default_key.clone())
            .actor_json_policy(external_actor_json_policy()),
    )));
    let higgs = Arc::new(
        HiggsConfig::builder()
            .valence_factory_arc(factory)
            .build()
            .expect("e2e HiggsConfig"),
    );

    let state = Arc::new(E2eState {
        router,
        higgs,
        default_backend_key: default_key,
        fixtures: Mutex::new(FixtureIds {
            owner_image_file_id,
            owner_text_file_id,
            peer_file_id,
            owner_scanning_file_id,
            owner_slow_scan_file_id,
            owner_quarantined_file_id,
            owner_big_file_id,
            owner_pdf_file_id,
        }),
    });
    let _ = E2E_STATE.set(state);
}

fn state() -> Arc<E2eState> {
    E2E_STATE
        .get()
        .expect("init_e2e_valence must run first")
        .clone()
}

pub fn e2e_router() -> Arc<DatabaseRouter> {
    Arc::clone(&state().router)
}

pub fn e2e_higgs_config() -> Arc<HiggsConfig> {
    Arc::clone(&state().higgs)
}

pub fn e2e_fixtures() -> FixtureIds {
    state().fixtures.lock().expect("fixtures").clone()
}

pub fn e2e_system_valence() -> Valence {
    Valence::builder()
        .database_router(e2e_router())
        .default_backend_key(state().default_backend_key.clone())
        .with_actor(Actor::System {
            operation: "e2e_seed".into(),
        })
        .build()
        .expect("system valence")
}
