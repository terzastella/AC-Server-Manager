use super::*;

#[derive(Serialize)]
pub struct CmScan {
    pub location: cm::CmLocation,
    pub settings: Vec<cm::CmSetting>,
    pub server_presets: Vec<cm::CmServerPreset>,
    pub user_presets: Vec<cm::CmUserPreset>,
    pub total_keys: usize,
    pub error: Option<String>,
}

#[tauri::command]
pub fn cm_scan(state: State<AppState>) -> CmScan {
    let custom = state.settings.lock().unwrap().cm_dir.clone();
    let location = cm::locate::locate(custom.as_deref());

    let mut error = None;
    let mut entries = Vec::new();
    match &location.values_file {
        Some(values) => match cm::storage::read_file(Path::new(values)) {
            Ok(parsed) => entries = parsed,
            Err(message) => error = Some(message),
        },
        None => error = Some("Values.data di Content Manager non trovato".into()),
    }

    let settings = cm::storage::settings_from(&entries);
    let server_presets = location
        .ac_root
        .as_deref()
        .map(cm::presets::scan_server_presets)
        .unwrap_or_default();
    let user_presets = location
        .data_dir
        .as_deref()
        .map(cm::presets::scan_user_presets)
        .unwrap_or_default();

    CmScan {
        location,
        total_keys: settings.len(),
        settings,
        server_presets,
        user_presets,
        error,
    }
}

#[tauri::command]
pub fn cm_import_server_preset(state: State<AppState>, source: String) -> Result<String, String> {
    let destination_root = state.server_dir().join("presets");
    fs::create_dir_all(&destination_root).map_err(|error| error.to_string())?;
    let destination = cm::presets::import_server_preset(Path::new(&source), &destination_root)?;
    Ok(destination.to_string_lossy().to_string())
}

#[tauri::command]
pub fn cm_save_snapshot(
    state: State<AppState>,
    settings: Vec<cm::CmSetting>,
    source: Option<String>,
) -> Result<String, String> {
    let payload = serde_json::json!({
        "generated_at": chrono::Local::now().to_rfc3339(),
        "source": source,
        "settings": settings,
    });
    let path = state.server_dir().join("cm-import.json");
    let text = serde_json::to_string_pretty(&payload).map_err(|error| error.to_string())?;
    write_text(&path, &text)?;
    Ok(path.to_string_lossy().to_string())
}

// ---------------------------------------------------------------- presets
