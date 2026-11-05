use super::*;

#[derive(Serialize)]
pub struct PresetInfo {
    pub name: String,
    pub server_name: String,
    pub track: String,
    pub cars: Vec<String>,
}

#[tauri::command]
pub fn list_presets(state: State<AppState>) -> Vec<PresetInfo> {
    let mut presets = Vec::new();
    for path in subdirs(&state.server_dir().join("presets")) {
        let config = fs::read_to_string(path.join("server_cfg.ini")).unwrap_or_default();
        let sections = ini::parse(&config).sections;
        presets.push(PresetInfo {
            name: dir_name(&path),
            server_name: section_value(&sections, "SERVER", "NAME").unwrap_or_default(),
            track: section_value(&sections, "SERVER", "TRACK").unwrap_or_default(),
            cars: used_cars(&sections),
        });
    }
    presets
}

#[tauri::command]
pub fn save_preset(state: State<AppState>, name: String) -> Result<String, String> {
    let safe: String = name
        .chars()
        .filter(|character| {
            character.is_alphanumeric()
                || *character == '_'
                || *character == '-'
                || *character == ' '
        })
        .collect();
    let safe = safe.trim().to_string();
    if safe.is_empty() {
        return Err(messages::tr(
            "nome preset non valido",
            "invalid preset name",
        ));
    }
    let destination = state.server_dir().join("presets").join(&safe);
    fs::create_dir_all(&destination).map_err(|error| error.to_string())?;
    let source = state.cfg_dir();
    for file in ["server_cfg.ini", "entry_list.ini", "welcome.txt"] {
        let from = source.join(file);
        if from.is_file() {
            fs::copy(&from, destination.join(file)).map_err(|error| error.to_string())?;
        }
    }
    Ok(destination.to_string_lossy().to_string())
}

#[tauri::command]
pub fn load_preset(state: State<AppState>, name: String) -> Result<(), String> {
    let directory = state.server_dir().join("presets").join(&name);
    if !directory.is_dir() {
        return Err(messages::tr(
            &format!("preset {} non trovato", name),
            &format!("preset {} not found", name),
        ));
    }
    // senza backup il preset non deve sovrascrivere la configurazione corrente
    backup_cfg(&state)?;
    let destination = state.cfg_dir();
    for file in ["server_cfg.ini", "entry_list.ini", "welcome.txt"] {
        let from = directory.join(file);
        if from.is_file() {
            fs::copy(&from, destination.join(file)).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

#[derive(Serialize)]
pub struct BackupInfo {
    pub name: String,
    pub path: String,
    pub modified: u64,
}

#[tauri::command]
pub fn list_backups(state: State<AppState>) -> Vec<BackupInfo> {
    let mut backups: Vec<BackupInfo> = subdirs(&state.server_dir().join("backups"))
        .into_iter()
        .filter(|path| dir_name(path) != "profiles")
        .map(|path| BackupInfo {
            name: dir_name(&path),
            modified: fs::metadata(&path)
                .and_then(|meta| meta.modified())
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs())
                .unwrap_or(0),
            path: path.to_string_lossy().to_string(),
        })
        .collect();
    backups.sort_by_key(|item| std::cmp::Reverse(item.modified));
    backups
}
