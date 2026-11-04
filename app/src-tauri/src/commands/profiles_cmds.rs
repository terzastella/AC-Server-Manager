use super::*;

#[tauri::command]
pub fn list_profiles(state: State<AppState>) -> crate::profiles::ProfileIndex {
    state.profiles.lock().unwrap().clone()
}

#[tauri::command]
pub fn create_profile(
    state: State<AppState>,
    name: String,
) -> Result<crate::profiles::ProfileIndex, String> {
    if server_running(&state) {
        return Err(messages::tr(
            "ferma il server prima di creare un profilo",
            "stop the server before creating a profile",
        ));
    }
    let root = state.server_dir();
    let mut index = state.profiles.lock().unwrap();
    let active = index.active.clone();
    let from = index
        .profiles
        .iter()
        .find(|entry| entry.id == active)
        .cloned()
        .or_else(|| index.profiles.first().cloned())
        .ok_or_else(|| "nessun profilo disponibile".to_string())?;
    let entry = crate::profiles::create(&root, &mut index, &name, &from)?;
    index.active = entry.id.clone();
    crate::profiles::save(&root, &index)?;
    Ok(index.clone())
}

#[tauri::command]
pub fn import_profile(
    state: State<AppState>,
    path: String,
    name: String,
) -> Result<crate::profiles::ProfileIndex, String> {
    if server_running(&state) {
        return Err(messages::tr(
            "ferma il server prima di importare un profilo",
            "stop the server before importing a profile",
        ));
    }
    let folder = PathBuf::from(path.trim());
    if !folder.is_dir() {
        return Err(messages::tr("cartella non trovata", "folder not found"));
    }
    let cfg = if folder.join("cfg").join("server_cfg.ini").is_file() {
        folder.join("cfg")
    } else if folder.join("server_cfg.ini").is_file() {
        folder.clone()
    } else {
        return Err(messages::tr(
            "nella cartella non c'è cfg/server_cfg.ini",
            "cfg/server_cfg.ini not found in that folder",
        ));
    };
    let root = state.server_dir();
    let mut index = state.profiles.lock().unwrap();
    let entry = crate::profiles::import_from(&root, &mut index, &cfg, &name)?;
    index.active = entry.id.clone();
    crate::profiles::save(&root, &index)?;
    Ok(index.clone())
}

#[tauri::command]
pub fn duplicate_profile(
    state: State<AppState>,
    id: String,
) -> Result<crate::profiles::ProfileIndex, String> {
    if server_running(&state) {
        return Err(messages::tr(
            "ferma il server prima di duplicare un profilo",
            "stop the server before duplicating a profile",
        ));
    }
    let root = state.server_dir();
    let mut index = state.profiles.lock().unwrap();
    crate::profiles::duplicate(&root, &mut index, &id)?;
    Ok(index.clone())
}

#[tauri::command]
pub fn rename_profile(
    state: State<AppState>,
    id: String,
    name: String,
) -> Result<crate::profiles::ProfileIndex, String> {
    let root = state.server_dir();
    let mut index = state.profiles.lock().unwrap();
    crate::profiles::rename(&mut index, &id, &name)?;
    crate::profiles::save(&root, &index)?;
    Ok(index.clone())
}

#[tauri::command]
pub fn delete_profile(
    state: State<AppState>,
    id: String,
) -> Result<crate::profiles::ProfileIndex, String> {
    if server_running(&state) {
        return Err(messages::tr(
            "ferma il server prima di eliminare un profilo",
            "stop the server before deleting a profile",
        ));
    }
    let root = state.server_dir();
    let mut index = state.profiles.lock().unwrap();
    crate::profiles::delete(&root, &mut index, &id)?;
    crate::profiles::save(&root, &index)?;
    Ok(index.clone())
}

#[tauri::command]
pub fn set_active_profile(
    state: State<AppState>,
    id: String,
) -> Result<crate::profiles::ProfileIndex, String> {
    if server_running(&state) {
        return Err(messages::tr(
            "ferma il server prima di cambiare profilo",
            "stop the server before switching profile",
        ));
    }
    let root = state.server_dir();
    let mut index = state.profiles.lock().unwrap();
    if !index.profiles.iter().any(|entry| entry.id == id) {
        return Err(messages::tr(
            &format!("server {} non trovato", id),
            &format!("server {} not found", id),
        ));
    }
    index.active = id;
    crate::profiles::save(&root, &index)?;
    Ok(index.clone())
}

// ---------------------------------------------------------------- server
