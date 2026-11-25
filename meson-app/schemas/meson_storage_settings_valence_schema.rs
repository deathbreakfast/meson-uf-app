use valence::prelude::*;
use valence::privacy_policies::common::{AUTHENTICATED, BLOCK_ALL};

/// Site-wide storage cost settings: one row with id `site`.
valence_schema! {
    MesonStorageSettings {
        repository: "https://github.com/unified-field-dev/meson-uf-app",
        table: "meson_storage_settings",
        version: "0.1.0",
        database: crate::storage::DEFAULT_STORAGE,
        description: "Site storage cost settings (copies override, price per GB-month, currency). One row, id `site`. Writers: meson_app::server::save_storage_settings.",

        policies: {
            read:   { allow: [AUTHENTICATED] },
            // StorageAdmin is checked by the only writer, the server fn (Gauge stays a host concern).
            create: { allow: [AUTHENTICATED] },
            update: { allow: [AUTHENTICATED] },
            delete: { always_block: [BLOCK_ALL] },
        },

        fields: [
            id: { r#type: FieldType::String, primary_key: true, required: true },
            copies_override: { r#type: FieldType::Integer, required: false },
            price_per_gb_month: { r#type: FieldType::Decimal, required: false },
            currency: { r#type: FieldType::String, required: true, default: "USD" },
            updated_by: { r#type: FieldType::Record("user"), required: false },
            updated_at: { r#type: FieldType::DateTime, required: true },
        ],

        connections: [
            updated_by: {
                table: "user",
                cardinality: HasOne,
                required: false,
                on_delete: SetNull,
                // Codegen stub for the `user` hop, as in Meson's File trait; ids stay `user:<id>`.
                model: "meson::generated::E2eMesonSoftUser",
            },
        ],
    }
}
