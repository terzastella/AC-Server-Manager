use super::*;

/// SteamID64 dell'utente locale: serve a `entry_list.ini` (`GUID=`) perché AC
/// faccia entrare nello slot prenotato. Da `HKCU\Software\Valve\Steam\ActiveProcess`.
#[cfg(windows)]
fn steam_id64() -> Option<String> {
    let mut process = Command::new("reg");
    process.args([
        "query",
        r"HKCU\Software\Valve\Steam\ActiveProcess",
        "/v",
        "ActiveUser",
    ]);
    process.creation_flags(CREATE_NO_WINDOW);
    let output = process.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let token = text
        .split_whitespace()
        .find(|token| token.starts_with("0x"))?;
    let account = u64::from_str_radix(&token[2..], 16).ok()?;
    if account == 0 {
        return None;
    }
    Some((76_561_197_960_265_728u64 + account).to_string())
}

#[cfg(not(windows))]
fn steam_id64() -> Option<String> {
    None
}

#[tauri::command]
pub fn steam_guid() -> Option<String> {
    steam_id64()
}

fn set_values(sections: &mut Vec<ini::Section>, name: &str, pairs: &[(&str, String)]) {
    let index = sections
        .iter()
        .position(|section| section.name.eq_ignore_ascii_case(name))
        .unwrap_or_else(|| {
            sections.push(ini::Section {
                name: name.to_string(),
                entries: Vec::new(),
            });
            sections.len() - 1
        });
    for (key, value) in pairs {
        let section = &mut sections[index];
        if let Some(entry) = section
            .entries
            .iter_mut()
            .find(|entry| entry.key.eq_ignore_ascii_case(key))
        {
            entry.value = value.clone();
        } else {
            section.entries.push(ini::Entry {
                key: key.to_string(),
                value: value.clone(),
            });
        }
    }
}

/// Scrive `race.ini` con la sezione `[REMOTE]` e avvia `acs.exe`: il gioco entra
/// direttamente nel server, senza passare da Content Manager.
#[tauri::command]
pub fn launch_game(
    state: State<AppState>,
    ip: String,
    port: u16,
    http_port: u16,
    name: String,
    car: String,
    skin: String,
    server_name: String,
) -> Result<String, String> {
    if !crate::invite::valid_host(&ip) {
        return Err(messages::tr(
            "indirizzo del server non valido",
            "invalid server address",
        ));
    }
    let game_dir = {
        let configured = state.settings.lock().unwrap().game_dir.clone();
        configured
            .filter(|dir| Path::new(dir.as_str()).join("acs.exe").is_file())
            .or_else(|| {
                crate::cm::locate::locate(None)
                    .ac_root
                    .filter(|dir| Path::new(dir.as_str()).join("acs.exe").is_file())
            })
    }
    .ok_or_else(|| {
        "Assetto Corsa non trovato: imposta la cartella del gioco nelle Impostazioni".to_string()
    })?;

    let cfg_dir = crate::assists::ac_cfg_dir()
        .ok_or_else(|| "cartella Documenti\\Assetto Corsa non trovata".to_string())?;
    std::fs::create_dir_all(&cfg_dir).map_err(|error| error.to_string())?;
    let path = cfg_dir.join("race.ini");
    let base = std::fs::read_to_string(&path).unwrap_or_default();
    if path.is_file() {
        let _ = std::fs::copy(&path, path.with_extension("ini.bak"));
    }
    let mut sections = ini::parse(&base).sections;
    let guid = steam_id64().unwrap_or_default();
    set_values(
        &mut sections,
        "REMOTE",
        &[
            ("ACTIVE", "1".to_string()),
            ("GUID", guid),
            ("NAME", name.clone()),
            ("PASSWORD", String::new()),
            ("REQUESTED_CAR", car.clone()),
            ("SERVER_IP", ip),
            ("SERVER_PORT", port.to_string()),
            ("SERVER_HTTP_PORT", http_port.to_string()),
            ("SERVER_NAME", server_name),
            ("TEAM", String::new()),
        ],
    );
    set_values(
        &mut sections,
        "CAR_0",
        &[("MODEL", car), ("SKIN", skin), ("DRIVER_NAME", name)],
    );
    let text = ini::apply(&base, &sections);
    std::fs::write(&path, text).map_err(|error| error.to_string())?;

    let acs = Path::new(&game_dir).join("acs.exe");
    Command::new(&acs)
        .current_dir(&game_dir)
        .spawn()
        .map_err(|error| format!("avvio del gioco non riuscito: {}", error))?;
    Ok(game_dir)
}
