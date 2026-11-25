use spectra::spectra_schema;

spectra_schema! {
    MesonStorageDaily {
        store: "default",
        table: "meson_storage_daily",
        version: "0.1.0",
        description: "Daily Meson storage snapshot: files and bytes per owner, app and scan status, with a site total",
        fields: [
            ds: {
                r#type: String,
                classification: { pii: false, safe_for_console: true },
            },
            measured_at: {
                r#type: String,
                classification: { pii: false, safe_for_console: true },
            },
            run_id: {
                r#type: String,
                classification: { pii: false, safe_for_console: true },
            },
            owner_kind: {
                r#type: String,
                classification: { pii: false, safe_for_console: true },
            },
            owner_id: {
                r#type: String,
                classification: { pii: true, safe_for_console: false },
            },
            app_table: {
                r#type: String,
                classification: { pii: false, safe_for_console: true },
            },
            file_status: {
                r#type: String,
                classification: { pii: false, safe_for_console: true },
            },
            files: {
                r#type: i64,
                classification: { pii: false, safe_for_console: true },
            },
            bytes: {
                r#type: i64,
                classification: { pii: false, safe_for_console: true },
            },
            missing_size_files: {
                r#type: i64,
                classification: { pii: false, safe_for_console: true },
            },
            copies_reported: {
                r#type: i64,
                classification: { pii: false, safe_for_console: true },
            },
        ],
    }
}
