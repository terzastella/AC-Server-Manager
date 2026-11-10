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
        // stessa regola del caricamento: i collegamenti non vengono seguiti
        match fs::symlink_metadata(&from) {
            Ok(_) => copy_preset_file(&from, &destination.join(file))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("{}: {}", from.display(), error)),
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

/// Rifiuta tutto ciò che non è un file regolare: `Path::is_file()` segue i
/// collegamenti, un symlink o una junction con il nome di un file di
/// configurazione porterebbe dentro `cfg/` il contenuto di un file esterno.
fn ensure_regular_file(path: &Path) -> Result<(), String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("{}: {}", path.display(), error))?;
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
                path.display()
            ),
            &format!(
                "the preset contains a link instead of a file: {}",
                path.display()
            ),
        ));
    }
    Ok(())
}

/// Copia un file di un preset solo se è un file regolare.
fn copy_preset_file(from: &Path, to: &Path) -> Result<(), String> {
    ensure_regular_file(from)?;
    fs::copy(from, to)
        .map(|_| ())
        .map_err(|error| format!("{}: {}", from.display(), error))
}

/// Legge in memoria i file del preset, validandoli tutti prima di toccare
/// `cfg/`: se uno è un collegamento o è illeggibile, la configurazione attuale
/// non viene sfiorata.
fn read_preset_files(directory: &Path) -> Result<Vec<(String, String)>, String> {
    let mut staged: Vec<(String, String)> = Vec::new();
    for file in ["server_cfg.ini", "entry_list.ini", "welcome.txt"] {
        let from = directory.join(file);
        match fs::symlink_metadata(&from) {
            Ok(_) => {
                ensure_regular_file(&from)?;
                let text = fs::read_to_string(&from)
                    .map_err(|error| format!("{}: {}", from.display(), error))?;
                staged.push((file.to_string(), text));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("{}: {}", from.display(), error)),
        }
    }
    Ok(staged)
}

/// Contenuto precedente di un file di configurazione: assente, oppure il testo
/// esatto da rimettere in caso di rollback.
enum Previous {
    Missing,
    Content(String),
}

/// Legge il contenuto precedente. Un file assente è normale; un file che c'è ma
/// non si legge (permessi, byte non UTF-8) non lo è: senza il contenuto
/// precedente il rollback potrebbe cancellare un file che va ripristinato,
/// quindi ci si ferma prima di scrivere qualsiasi cosa.
fn snapshot_file(path: &Path) -> Result<Previous, String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Previous::Content(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Previous::Missing),
        Err(error) => Err(messages::tr(
            &format!(
                "impossibile leggere la configurazione attuale ({}): nessuna modifica applicata",
                error
            ),
            &format!(
                "cannot read the current configuration ({}): no change applied",
                error
            ),
        )),
    }
}

/// Scrive i file già validati nella configurazione, con rollback: se una
/// scrittura fallisce i file tornano al contenuto precedente (e quelli creati
/// ora vengono rimossi). Se anche il ripristino fallisce, l'errore lo dice
/// esplicitamente invece di dichiarare riuscito ciò che non lo è.
/// Il chiamante tiene il lock di `entry_list.ini`.
fn apply_preset_files(cfg_dir: &Path, files: &[(String, String)]) -> Result<(), String> {
    let mut previous: Vec<(String, Previous)> = Vec::new();
    for (name, _) in files {
        previous.push((name.clone(), snapshot_file(&cfg_dir.join(name))?));
    }

    for (name, text) in files {
        if let Err(error) = write_text_atomic(&cfg_dir.join(name), text) {
            let mut rollback_errors: Vec<String> = Vec::new();
            for (old_name, old) in &previous {
                let restored: Result<(), String> = match old {
                    Previous::Content(restore) => {
                        write_text_atomic(&cfg_dir.join(old_name), restore)
                    }
                    Previous::Missing => match fs::remove_file(cfg_dir.join(old_name)) {
                        Ok(()) => Ok(()),
                        // il file non è stato creato (la scrittura è fallita prima): va bene
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                        Err(error) => Err(error.to_string()),
                    },
                };
                if let Err(rollback_error) = restored {
                    rollback_errors.push(format!("{}: {}", old_name, rollback_error));
                }
            }
            if rollback_errors.is_empty() {
                return Err(messages::tr(
                    &format!("{}, la configurazione è stata ripristinata", error),
                    &format!("{}, the configuration was restored", error),
                ));
            }
            return Err(messages::tr(
                &format!(
                    "{}; ripristino incompleto ({}): controlla i backup in backups/",
                    error,
                    rollback_errors.join("; ")
                ),
                &format!(
                    "{}; incomplete rollback ({}): check the backups in backups/",
                    error,
                    rollback_errors.join("; ")
                ),
            ));
        }
    }
    Ok(())
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
    let files = read_preset_files(&directory)?;
    if files.is_empty() {
        return Err(messages::tr(
            &format!("il preset {} non ha file di configurazione", name),
            &format!("preset {} has no configuration files", name),
        ));
    }
    // lock e backup nello stesso ordine: lo snapshot salvato è coerente con la
    // configurazione che viene sovrascritta, e il caricamento non può
    // sovrapporsi a una prenotazione
    let _guard = entry_lock().lock().unwrap();
    // senza backup il preset non deve sovrascrivere la configurazione corrente
    backup_cfg(&state)?;
    apply_preset_files(&state.cfg_dir(), &files)
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

    #[cfg(windows)]
    #[test]
    fn a_preset_with_a_junction_is_not_read() {
        let root = sandbox("readlink");
        let preset = root.join("presets").join("buono");
        fs::write(preset.join("server_cfg.ini"), "[SERVER]\nNAME=Ok\n").expect("file");
        junction::create(root.join("fuori"), preset.join("entry_list.ini")).expect("junction");

        // la lettura si ferma prima di toccare cfg/: nessun file applicato
        assert!(read_preset_files(&preset).is_err());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_failed_preset_write_restores_the_previous_configuration() {
        let root =
            std::env::temp_dir().join(format!("acsc-preset-rollback-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let cfg = root.join("cfg");
        fs::create_dir_all(&cfg).expect("cfg");
        fs::write(cfg.join("server_cfg.ini"), "vecchio").expect("file");
        // una cartella al posto dell'ultimo file: la scrittura fallisce
        fs::create_dir_all(cfg.join("entry_list.ini")).expect("cartella");

        let files = vec![
            // creato dal caricamento: in rollback deve sparire
            ("welcome.txt".to_string(), "ciao".to_string()),
            ("server_cfg.ini".to_string(), "nuovo".to_string()),
            ("entry_list.ini".to_string(), "contenuto".to_string()),
        ];
        let error = apply_preset_files(&cfg, &files).expect_err("la scrittura deve fallire");

        assert!(!error.is_empty());
        assert_eq!(
            fs::read_to_string(cfg.join("server_cfg.ini")).expect("letto"),
            "vecchio"
        );
        assert!(
            !cfg.join("welcome.txt").exists(),
            "il file creato durante il caricamento deve essere rimosso"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_configuration_that_cannot_be_read_stops_before_writing() {
        let root =
            std::env::temp_dir().join(format!("acsc-preset-unreadable-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let cfg = root.join("cfg");
        fs::create_dir_all(&cfg).expect("cfg");
        // il file esiste ma non è UTF-8: non si può leggere per ripristinarlo
        let original = vec![0xFFu8, 0xFE, 0x00, 0x01];
        fs::write(cfg.join("server_cfg.ini"), &original).expect("file");

        let files = vec![("server_cfg.ini".to_string(), "nuovo".to_string())];
        let error = apply_preset_files(&cfg, &files).expect_err("deve fermarsi prima di scrivere");

        assert!(!error.is_empty());
        // il file originale resta intatto: non sovrascritto né cancellato
        assert_eq!(
            fs::read(cfg.join("server_cfg.ini")).expect("letto"),
            original
        );
        let _ = fs::remove_dir_all(&root);
    }
}
