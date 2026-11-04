use super::*;

#[tauri::command]
pub fn backup_now(state: State<AppState>) -> Result<String, String> {
    backup_cfg(&state)
}

#[tauri::command]
pub fn read_config(state: State<AppState>) -> Result<ini::Doc, String> {
    read_text(&state.cfg_dir().join("server_cfg.ini")).map(|text| ini::parse(&text))
}

#[tauri::command]
pub fn save_config(state: State<AppState>, sections: Vec<ini::Section>) -> Result<String, String> {
    let path = state.cfg_dir().join("server_cfg.ini");
    let base = read_text(&path)?;
    let backup = backup_cfg(&state).unwrap_or_default();
    let text = ini::apply(&base, &sections);
    write_text(&path, &text)?;
    Ok(backup)
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct CarSlot {
    pub driver_name: String,
    pub team: String,
    pub model: String,
    pub skin: String,
    pub spectator_mode: i64,
    pub guid: String,
    pub ballast: i64,
    pub restrictor: i64,
}

#[tauri::command]
pub fn read_entry_list(state: State<AppState>) -> Result<Vec<CarSlot>, String> {
    let text = read_text(&state.cfg_dir().join("entry_list.ini"))?;
    let mut slots = Vec::new();
    for section in ini::parse(&text).sections {
        if !section.name.to_uppercase().starts_with("CAR_") {
            continue;
        }
        let mut slot = CarSlot::default();
        for entry in section.entries {
            let value = entry.value;
            match entry.key.to_uppercase().as_str() {
                "DRIVERNAME" => slot.driver_name = value,
                "TEAM" => slot.team = value,
                "MODEL" => slot.model = value,
                "SKIN" => slot.skin = value,
                "SPECTATOR_MODE" => slot.spectator_mode = value.parse().unwrap_or(0),
                "GUID" => slot.guid = value,
                "BALLAST" => slot.ballast = value.parse().unwrap_or(0),
                "RESTRICTOR" => slot.restrictor = value.parse().unwrap_or(0),
                _ => {}
            }
        }
        slots.push(slot);
    }
    Ok(slots)
}

#[tauri::command]
pub fn save_entry_list(state: State<AppState>, slots: Vec<CarSlot>) -> Result<String, String> {
    let path = state.cfg_dir().join("entry_list.ini");
    let base = read_text(&path).unwrap_or_default();
    let newline = if base.contains("\r\n") { "\r\n" } else { "\n" };
    let backup = backup_cfg(&state).unwrap_or_default();

    let mut out = String::new();
    for line in base.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            break;
        }
        if !trimmed.is_empty() {
            out.push_str(line.trim_end_matches('\r'));
            out.push_str(newline);
        }
    }
    for (index, slot) in slots.iter().enumerate() {
        out.push_str(&format!("[CAR_{}]{}", index, newline));
        out.push_str(&format!("DRIVERNAME={}{}", slot.driver_name, newline));
        out.push_str(&format!("TEAM={}{}", slot.team, newline));
        out.push_str(&format!("MODEL={}{}", slot.model, newline));
        out.push_str(&format!("SKIN={}{}", slot.skin, newline));
        out.push_str(&format!(
            "SPECTATOR_MODE={}{}",
            slot.spectator_mode, newline
        ));
        out.push_str(&format!("GUID={}{}", slot.guid, newline));
        out.push_str(&format!("BALLAST={}{}", slot.ballast, newline));
        out.push_str(&format!("RESTRICTOR={}{}", slot.restrictor, newline));
        out.push_str(newline);
    }
    write_text(&path, &out)?;
    Ok(backup)
}

#[tauri::command]
pub fn read_welcome(state: State<AppState>) -> Result<String, String> {
    read_text(&state.cfg_dir().join("welcome.txt"))
}

#[tauri::command]
pub fn save_welcome(state: State<AppState>, text: String) -> Result<(), String> {
    write_text(&state.cfg_dir().join("welcome.txt"), &text)
}

// ---------------------------------------------------------------- content
