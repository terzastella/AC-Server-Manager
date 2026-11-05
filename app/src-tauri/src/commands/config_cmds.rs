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
    // senza backup il salvataggio non deve avvenire
    let backup = backup_cfg(&state)?;
    let text = ini::apply(&base, &sections);
    write_text_atomic(&path, &text)?;
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

fn slot_entries(slot: &CarSlot) -> Vec<ini::Entry> {
    [
        ("DRIVERNAME", slot.driver_name.clone()),
        ("TEAM", slot.team.clone()),
        ("MODEL", slot.model.clone()),
        ("SKIN", slot.skin.clone()),
        ("SPECTATOR_MODE", slot.spectator_mode.to_string()),
        ("GUID", slot.guid.clone()),
        ("BALLAST", slot.ballast.to_string()),
        ("RESTRICTOR", slot.restrictor.to_string()),
    ]
    .into_iter()
    .map(|(key, value)| ini::Entry {
        key: key.to_string(),
        value,
    })
    .collect()
}

/// Rimuove interi blocchi `[...]` con i nomi indicati, lasciando il resto com'è.
fn remove_sections(text: &str, names: &[String]) -> String {
    if names.is_empty() {
        return text.to_string();
    }
    let mut out = String::new();
    let mut skipping = false;
    for raw in text.split_inclusive('\n') {
        let line = raw.trim_end_matches('\n').trim_end_matches('\r');
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let name = trimmed[1..trimmed.len() - 1].trim();
            skipping = names.iter().any(|item| item.eq_ignore_ascii_case(name));
            if skipping {
                continue;
            }
        }
        if !skipping {
            out.push_str(raw);
        }
    }
    out
}

/// Applica gli slot al file esistente: aggiorna solo le chiavi note, mantiene
/// commenti, ordine e chiavi sconosciute, aggiunge o rimuove i blocchi necessari.
pub(crate) fn apply_slots(base: &str, slots: &[CarSlot]) -> String {
    let parsed = ini::parse(base);
    let mut car_names: Vec<String> = Vec::new();
    let mut model: Vec<ini::Section> = Vec::new();

    for section in &parsed.sections {
        if section.name.to_uppercase().starts_with("CAR_") {
            car_names.push(section.name.clone());
        } else {
            model.push(section.clone());
        }
    }

    for (index, slot) in slots.iter().enumerate() {
        let name = car_names
            .get(index)
            .cloned()
            .unwrap_or_else(|| format!("CAR_{}", index));
        model.push(ini::Section {
            name,
            entries: slot_entries(slot),
        });
    }

    let applied = ini::apply(base, &model);
    let extra: Vec<String> = car_names.iter().skip(slots.len()).cloned().collect();
    remove_sections(&applied, &extra)
}

#[tauri::command]
pub fn save_entry_list(state: State<AppState>, slots: Vec<CarSlot>) -> Result<String, String> {
    let path = state.cfg_dir().join("entry_list.ini");
    let base = read_text(&path).unwrap_or_default();
    let backup = backup_cfg(&state)?;
    let text = apply_slots(&base, &slots);
    write_text_atomic(&path, &text)?;
    Ok(backup)
}

#[tauri::command]
pub fn read_welcome(state: State<AppState>) -> Result<String, String> {
    read_text(&state.cfg_dir().join("welcome.txt"))
}

#[tauri::command]
pub fn save_welcome(state: State<AppState>, text: String) -> Result<(), String> {
    write_text_atomic(&state.cfg_dir().join("welcome.txt"), &text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(driver: &str, model: &str) -> CarSlot {
        CarSlot {
            driver_name: driver.into(),
            model: model.into(),
            ..CarSlot::default()
        }
    }

    #[test]
    fn keeps_comments_and_unknown_keys() {
        let base =
            "; lista\r\n[CAR_0]\r\nMODEL=old\r\n; commento interno\r\nUNKNOWN=7\r\nDRIVERNAME=\r\n";
        let out = apply_slots(base, &[slot("Luca", "new")]);
        assert!(out.contains("; lista"));
        assert!(out.contains("; commento interno"));
        assert!(out.contains("UNKNOWN=7"));
        assert!(out.contains("MODEL=new"));
        assert!(out.contains("DRIVERNAME=Luca"));
    }

    #[test]
    fn removes_extra_slots() {
        let base = "[CAR_0]\nMODEL=a\n[CAR_1]\nMODEL=b\n[CAR_2]\nMODEL=c\n";
        let out = apply_slots(base, &[slot("A", "a")]);
        assert!(out.contains("[CAR_0]"));
        assert!(!out.contains("[CAR_1]"));
        assert!(!out.contains("[CAR_2]"));
    }

    #[test]
    fn appends_missing_slots() {
        let base = "[CAR_0]\nMODEL=a\n";
        let out = apply_slots(base, &[slot("A", "a"), slot("B", "b")]);
        assert!(out.contains("[CAR_1]"));
        assert!(out.contains("DRIVERNAME=B"));
    }

    #[test]
    fn keeps_crlf() {
        let base = "[CAR_0]\r\nMODEL=a\r\n";
        let out = apply_slots(base, &[slot("A", "a")]);
        assert!(out.contains("\r\n"));
    }

    #[test]
    fn empty_base_creates_slots() {
        let out = apply_slots("", &[slot("A", "a"), slot("B", "b")]);
        assert!(out.contains("[CAR_0]"));
        assert!(out.contains("[CAR_1]"));
        assert!(out.contains("DRIVERNAME=A"));
        assert!(out.contains("DRIVERNAME=B"));
    }
}
