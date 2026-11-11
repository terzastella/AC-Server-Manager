use super::*;

/// Converte il valore `ActiveUser` (`0x...`) in SteamID64. L'account id è a
/// 32 bit: un valore fuori intervallo viene scartato subito, invece di produrre
/// un GUID che il server rifiuterebbe.
fn steam_id64_from_hex(hex: &str) -> Option<String> {
    /// `76561197960265728` + account id
    const STEAM_ID64_BASE: u64 = 76_561_197_960_265_728;
    let digits = hex.strip_prefix("0x").or_else(|| hex.strip_prefix("0X"))?;
    let account = u64::from_str_radix(digits, 16).ok()?;
    if account == 0 || account > u64::from(u32::MAX) {
        return None;
    }
    STEAM_ID64_BASE
        .checked_add(account)
        .map(|steam_id| steam_id.to_string())
}

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
    steam_id64_from_hex(token)
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

/// Legge il `race.ini` esistente: un file assente vale come vuoto, qualsiasi
/// altro errore (permessi, byte non UTF-8) blocca il join invece di sovrascrivere
/// una configurazione che non si è riusciti a leggere.
fn read_race_base(path: &Path) -> Result<String, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(messages::tr(
            &format!("race.ini non leggibile ({}): join annullato", error),
            &format!("race.ini cannot be read ({}): join cancelled", error),
        )),
    }
}

/// Copie di `race.ini` conservate: quelle più vecchie vengono rimosse.
const RACE_BACKUPS: usize = 10;

/// `race.ini` deve essere un file regolare: un link o un reparse point
/// porterebbe la scrittura fuori dalla cartella del gioco.
fn ensure_regular_file(path: &Path) -> Result<(), String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => {
            let file_type = metadata.file_type();
            if crate::fsx::is_link(&file_type) || !file_type.is_file() {
                return Err(messages::tr(
                    "race.ini non è un file regolare (link o reparse point): join annullato",
                    "race.ini is not a regular file (link or reparse point): join cancelled",
                ));
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(messages::tr(
            &format!("race.ini non verificabile ({}): join annullato", error),
            &format!("race.ini cannot be checked ({}): join cancelled", error),
        )),
    }
}

/// Tiene solo le copie più recenti, così la cartella non cresce a ogni join.
fn prune_race_backups(dir: &Path) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    let mut backups: Vec<PathBuf> = read
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with("race.ini.") && name.ends_with(".bak"))
                .unwrap_or(false)
        })
        .collect();
    if backups.len() <= RACE_BACKUPS {
        return;
    }
    // il nome contiene la data: l'ordine alfabetico è quello cronologico
    backups.sort();
    let excess = backups.len() - RACE_BACKUPS;
    for old in backups.into_iter().take(excess) {
        let _ = std::fs::remove_file(old);
    }
}

/// Copia di sicurezza di `race.ini` con nome datato: ogni join conserva lo
/// stato precedente, senza sovrascrivere le copie già presenti.
fn backup_race_ini(path: &Path) -> Result<(), String> {
    ensure_regular_file(path)?;
    match std::fs::metadata(path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(messages::tr(
                &format!("race.ini non leggibile ({}): join annullato", error),
                &format!("race.ini cannot be read ({}): join cancelled", error),
            ))
        }
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S-%3f");
    let destination = parent.join(format!("race.ini.{}.bak", stamp));
    let copied = std::fs::copy(path, &destination)
        .map_err(|error| {
            messages::tr(
                &format!(
                    "backup di race.ini non riuscito ({}): join annullato",
                    error
                ),
                &format!("race.ini backup failed ({}): join cancelled", error),
            )
        })
        .and_then(|written| {
            let expected = std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
            if written != expected {
                return Err(messages::tr(
                    "copia di sicurezza incompleta: join annullato",
                    "incomplete backup: join cancelled",
                ));
            }
            Ok(())
        });
    if copied.is_ok() {
        prune_race_backups(parent);
    }
    copied
}

/// Scrive `race.ini` con la sezione `[REMOTE]` e avvia `acs.exe`: il gioco entra
/// direttamente nel server, senza passare da Content Manager.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
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
    // un link o un reparse point porterebbe la scrittura altrove
    ensure_regular_file(&path)?;
    // un file illeggibile non va mai trattato come vuoto: si sovrascriverebbe
    let base = read_race_base(&path)?;
    let mut sections = ini::parse(&base).sections;
    // senza SteamID64 la prenotazione non può combaciare: meglio fermarsi qui
    // che avviare il gioco con un GUID vuoto che non entra nello slot
    let guid = steam_id64().ok_or_else(|| {
        messages::tr(
            "Steam non rilevato: apri Steam e riprova, serve lo SteamID64 per entrare nello slot",
            "Steam not detected: open Steam and retry, the SteamID64 is required to join the slot",
        )
    })?;
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
    let text = ini::apply(&base, &sections).map_err(|error| {
        messages::tr(
            &format!("valore non valido per race.ini: {}", error),
            &format!("invalid value for race.ini: {}", error),
        )
    })?;
    // backup solo a configurazione già valida, subito prima della scrittura: la
    // copia resta l'ultimo stato precedente a un join che arriva in fondo, e un
    // join annullato prima non la tocca
    backup_race_ini(&path)?;
    write_text_atomic(&path, &text)?;

    let acs = Path::new(&game_dir).join("acs.exe");
    Command::new(&acs)
        .current_dir(&game_dir)
        .spawn()
        .map_err(|error| format!("avvio del gioco non riuscito: {}", error))?;
    Ok(game_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("acsc-join-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("cartella");
        dir
    }

    fn list_backups(dir: &Path) -> Vec<PathBuf> {
        std::fs::read_dir(dir)
            .expect("dir")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.starts_with("race.ini.") && name.ends_with(".bak"))
                    .unwrap_or(false)
            })
            .collect()
    }

    #[test]
    fn old_backups_are_pruned() {
        let dir = temp_dir("prune");
        let path = dir.join("race.ini");
        std::fs::write(&path, "[REMOTE]\r\n").expect("file");
        for _ in 0..RACE_BACKUPS + 5 {
            backup_race_ini(&path).expect("backup");
        }
        let backups = list_backups(&dir);
        assert!(
            backups.len() <= RACE_BACKUPS,
            "restano {} copie, attese al massimo {}",
            backups.len(),
            RACE_BACKUPS
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(windows)]
    #[test]
    fn a_race_ini_that_is_a_reparse_point_is_refused() {
        let dir = temp_dir("link");
        let target = dir.join("altrove");
        std::fs::create_dir_all(&target).expect("cartella");
        let path = dir.join("race.ini");
        junction::create(&target, &path).expect("junction");

        assert!(ensure_regular_file(&path).is_err());
        assert!(backup_race_ini(&path).is_err());
        let _ = std::fs::remove_dir(&path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_race_ini_is_treated_as_empty() {
        let dir = temp_dir("missing");
        assert_eq!(
            read_race_base(&dir.join("race.ini")).expect("assente va bene"),
            ""
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unreadable_race_ini_stops_the_join() {
        let dir = temp_dir("unreadable");
        let path = dir.join("race.ini");
        let original = vec![0xFFu8, 0xFE, 0x00];
        std::fs::write(&path, &original).expect("file");

        assert!(read_race_base(&path).is_err());
        // il file resta intatto: nessuna scrittura è avvenuta
        assert_eq!(std::fs::read(&path).expect("letto"), original);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_backup_is_written_and_the_file_is_replaced_atomically() {
        let dir = temp_dir("backup");
        let path = dir.join("race.ini");
        std::fs::write(&path, "[REMOTE]\r\nACTIVE=0\r\n").expect("file");

        backup_race_ini(&path).expect("backup");
        let backups = list_backups(&dir);
        assert_eq!(backups.len(), 1, "una copia datata: {:?}", backups);
        assert!(backups[0]
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with("race.ini.") && name.ends_with(".bak"))
            .unwrap_or(false));

        write_text_atomic(&path, "nuovo").expect("scrittura");
        assert_eq!(std::fs::read_to_string(&path).expect("letto"), "nuovo");

        let temporary = std::fs::read_dir(&dir)
            .expect("dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(temporary, 0, "nessun file temporaneo deve restare");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_active_user_becomes_a_steam_id64() {
        assert_eq!(
            steam_id64_from_hex("0x1").as_deref(),
            Some("76561197960265729")
        );
        assert_eq!(
            steam_id64_from_hex("0xFFFFFFFF").as_deref(),
            Some("76561202255233023")
        );
    }

    #[test]
    fn an_account_id_out_of_range_is_rejected() {
        // zero e valori oltre i 32 bit non sono account individuali validi
        assert!(steam_id64_from_hex("0x0").is_none());
        assert!(steam_id64_from_hex("0x100000000").is_none());
        assert!(steam_id64_from_hex("0xFFFFFFFFFFFFFFFF").is_none());
        // prefisso assente o cifre non esadecimali
        assert!(steam_id64_from_hex("1a2b").is_none());
        assert!(steam_id64_from_hex("0xzz").is_none());
    }
}
