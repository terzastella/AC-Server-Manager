use super::*;

#[derive(Serialize)]
pub struct AssistsState {
    pub path: Option<String>,
    pub exists: bool,
    pub values: crate::assists::Assists,
}

fn assists_snapshot() -> AssistsState {
    let path = crate::assists::config_path();
    let exists = path.as_deref().map(Path::is_file).unwrap_or(false);
    let values = path
        .as_deref()
        .map(crate::assists::load)
        .unwrap_or_default();
    AssistsState {
        path: path.map(|value| value.display().to_string()),
        exists,
        values,
    }
}

#[tauri::command]
pub fn assists_state() -> AssistsState {
    assists_snapshot()
}

#[tauri::command]
pub fn save_assists(values: crate::assists::Assists) -> Result<AssistsState, String> {
    let path = crate::assists::config_path()
        .ok_or_else(|| "cartella Documenti di Windows non trovata".to_string())?;
    crate::assists::save(&path, &values)?;
    Ok(assists_snapshot())
}
