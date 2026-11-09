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
    // lock condiviso: il preset cattura i tre file nello stesso istante,
    // senza prenotazioni o salvataggi a metà
    let _guard = entry_lock().lock().unwrap();
    let source = state.cfg_dir();
    for file in ["server_cfg.ini", "entry_list.ini", "welcome.txt"] {
        let from = source.join(file);
        if from.is_file() {
            fs::copy(&from, destination.join(file)).map_err(|error| error.to_string())?;
        }
    }
    Ok(destination.to_string_lossy().to_string())
}

/// Cartella di un preset, verificata: il nome arriva dall'interfaccia e non deve
/// poter uscire da `presets/` (né con `..`, né con percorsi assoluti o separatori).
/// Il confronto è fatto su percorsi canonici, quindi vale anche con junction e
/// collegamenti simbolici.
fn preset_dir(root: &Path, name: &str) -> Result<PathBuf, String> {
    let base = root.join("presets");
    crate::cm::presets::canonical_inside(&base.join(name), &base).map_err(|error| match error {
        crate::cm::presets::ConfineError::Missing => messages::tr(
            &format!("preset {} non trovato", name),
            &format!("preset {} not found", name),
        ),
        crate::cm::presets::ConfineError::Outside => {
            messages::tr("nome preset non valido", "invalid preset name")
        }
    })
}

/// Copia un file di un preset solo se è un file regolare. `Path::is_file()`
/// segue i collegamenti: un symlink o una junction con il nome di un file di
/// configurazione porterebbe dentro `cfg/` il contenuto di un file esterno.
fn copy_preset_file(from: &Path, to: &Path) -> Result<(), String> {
    let metadata =
        fs::symlink_metadata(from).map_err(|error| format!("{}: {}", from.display(), error))?;
    #[cfg(windows)]
    let is_link = {
        use std::os::windows::fs::FileTypeExt;
        let file_type = metadata.file_type();
        file_type.is_symlink_dir() || file_type.is_symlink_file()
    };
    #[cfg(not(windows))]
    let is_link = metadata.file_type().is_symlink();
    if is_link || !metadata.is_file() {
        return Err(messages::tr(
            &format!(
                "il preset contiene un collegamento al posto di un file: {}",
                from.display()
            ),
            &format!(
                "the preset contains a link instead of a file: {}",
                from.display()
            ),
        ));
    }
    fs::copy(from, to)
        .map(|_| ())
        .map_err(|error| format!("{}: {}", from.display(), error))
}

#[tauri::command]
pub fn load_preset(state: State<AppState>, name: String) -> Result<(), String> {
    let directory = preset_dir(&state.server_dir(), &name)?;
    if !directory.is_dir() {
        return Err(messages::tr(
            &format!("preset {} non trovato", name),
            &format!("preset {} not found", name),
        ));
    }
    // senza backup il preset non deve sovrascrivere la configurazione corrente
    backup_cfg(&state)?;
    // stesso lock delle prenotazioni: il preset non deve sovrascriverle a metà
    let _guard = entry_lock().lock().unwrap();
    let destination = state.cfg_dir();
    for file in ["server_cfg.ini", "entry_list.ini", "welcome.txt"] {
        let from = directory.join(file);
        match fs::symlink_metadata(&from) {
            Ok(_) => copy_preset_file(&from, &destination.join(file))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("{}: {}", from.display(), error)),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sandbox(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("acsc-preset-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("presets").join("buono")).expect("preset");
        fs::create_dir_all(root.join("fuori")).expect("cartella fuori");
        fs::write(root.join("fuori").join("server_cfg.ini"), "[SERVER]\n").expect("file");
        root
    }

    #[test]
    fn preset_name_cannot_escape_the_presets_folder() {
        let root = sandbox("escape");
        assert!(preset_dir(&root, "buono").is_ok());
        // `..`, percorsi relativi che risalgono e percorsi assoluti: rifiutati
        assert!(preset_dir(&root, "..").is_err());
        assert!(preset_dir(&root, "../fuori").is_err());
        assert!(preset_dir(&root, r"..\fuori").is_err());
        assert!(preset_dir(&root, "manca").is_err());
        let absolute = root.join("fuori").to_string_lossy().to_string();
        assert!(preset_dir(&root, &absolute).is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_regular_preset_file_is_copied() {
        let root = sandbox("copy");
        let from = root.join("presets").join("buono").join("server_cfg.ini");
        fs::write(&from, "[SERVER]\nNAME=Test\n").expect("file");
        let to = root.join("copia.ini");

        copy_preset_file(&from, &to).expect("copia");

        assert_eq!(
            fs::read_to_string(&to).expect("letto"),
            "[SERVER]\nNAME=Test\n"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(windows)]
    #[test]
    fn a_junction_named_like_a_preset_file_is_refused() {
        let root = sandbox("link");
        let preset = root.join("presets").join("buono");
        // una junction con il nome di un file di configurazione: `fs::copy` la
        // seguirebbe e copierebbe contenuto che sta fuori dal preset
        junction::create(root.join("fuori"), preset.join("welcome.txt")).expect("junction");

        let error = copy_preset_file(&preset.join("welcome.txt"), &root.join("welcome.txt"))
            .expect_err("la junction deve essere rifiutata");

        assert!(
            error.to_lowercase().contains("collegamento") || error.to_lowercase().contains("link"),
            "{}",
            error
        );
        assert!(!root.join("welcome.txt").exists());
        let _ = fs::remove_dir_all(&root);
    }
}
