//! Process-wide Valence + Higgs for Playwright (meson mem + sqlite alias).

use std::sync::{Arc, Mutex, OnceLock};

use chrono::Utc;
use higgs::actor_policy::external_actor_json_policy;
use higgs::{HiggsConfig, HiggsValenceFactory};
use meson::generated::{E2eMesonFile, FileFileStatus};
use meson::touch_schema_inventory;
use valence::{
    register_backend_logical_names, router_key, Actor, DatabaseBackend, DatabaseRouter,
    InMemoryBackend, Model, RecordId, RegisterBackendLogicalNamesOptions, RouterValenceFactory,
    RouterValenceFactoryConfig, Valence, ValenceFactory, MEM_ENGINE_ID, SQLITE_ENGINE_ID,
};

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
}

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

async fn seed_file(
    valence: &Valence,
    id: &str,
    uploader: RecordId,
    file_name: &str,
    mime: &str,
    storage_path: &str,
    size_bytes: i64,
) -> String {
    let extension = file_name.rsplit('.').next().unwrap_or("bin").to_string();
    let row = E2eMesonFile::new(
        file_name.to_string(),
        extension,
        mime.to_string(),
        size_bytes,
        storage_path.to_string(),
        FileFileStatus::Available,
        uploader,
        Utc::now(),
    )
    .expect("E2eMesonFile::new");
    let created = E2eMesonFile::upsert(id, row, valence, valence::use_!(r"**Test:** Fixture **E2e Meson File** save for `e2e_valence` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."))
        .await
        .expect("upsert e2e_meson_file");
    created
        .id()
        .map_or_else(|| id.to_string(), std::string::ToString::to_string)
}

/// Build shared Valence/Higgs once and seed baseline fixtures.
pub async fn init_e2e_valence() {
    if E2E_STATE.get().is_some() {
        return;
    }

    prepare_env();
    touch_schema_inventory();

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
        "owner-image",
        owner_rid(),
        "receipt.png",
        "image/png",
        "owner-image.png",
        2048,
    )
    .await;
    let owner_text_file_id = seed_file(
        &system,
        "owner-text",
        owner_rid(),
        "notes.txt",
        "text/plain",
        "owner-text.txt",
        64,
    )
    .await;
    let peer_file_id = seed_file(
        &system,
        "peer-file",
        peer_rid(),
        "peer-only.png",
        "image/png",
        "peer-file.png",
        1024,
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

#[allow(dead_code)] // parity with gauge's e2e_valence shape; unused until a seed-time fixture reset is needed
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
