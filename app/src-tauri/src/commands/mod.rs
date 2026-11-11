use crate::cm;
use crate::ini;
use crate::messages;
use crate::state::{AppState, LogEntry, Settings};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Mutex, OnceLock},
    time::Instant,
};
use tauri::{AppHandle, Emitter, Manager, State};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

// ---------------------------------------------------------------- helpers

/// Lock condiviso da chiunque modifichi `entry_list.ini`: prenotazioni HTTP,
/// salvataggio manuale degli slot, caricamento dei preset.
static ENTRY_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub(crate) fn entry_lock() -> &'static Mutex<()> {
    ENTRY_LOCK.get_or_init(|| Mutex::new(()))
}

/// Impronta del file (mtime in nanosecondi + dimensione): due scritture diverse
/// danno quasi certamente impronte diverse.
pub(crate) fn file_stamp(path: &Path) -> u64 {
    match fs::metadata(path) {
        Ok(meta) => {
            let modified = meta
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_nanos() as u64)
                .unwrap_or(0);
            modified ^ meta.len().rotate_left(17)
        }
        Err(_) => 0,
    }
}

pub(crate) fn read_text(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("{}: {}", path.display(), error))
}

/// Scrittura di un file di configurazione: passa dalla scrittura atomica
/// condivisa, così un errore non lascia mai il file a metà.
pub(crate) fn write_text(path: &Path, text: &str) -> Result<(), String> {
    write_text_atomic(path, text)
}

/// Come `read_text`, ma un file assente vale come vuoto: serve a creare un
/// file nuovo. Ogni altro errore (permessi, I/O, encoding) viene propagato,
/// altrimenti si sovrascriverebbe un contenuto esistente con una lista vuota.
pub(crate) fn read_text_or_empty(path: &Path) -> Result<String, String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(format!("{}: {}", path.display(), error)),
    }
}

/// Scrittura atomica condivisa con profili e launcher: vedi [`crate::fsx`].
pub(crate) use crate::fsx::write_text_atomic;

pub(crate) fn config_sections(state: &AppState) -> Vec<ini::Section> {
    let path = state.cfg_dir().join("server_cfg.ini");
    fs::read_to_string(path)
        .map(|text| ini::parse(&text).sections)
        .unwrap_or_default()
}

pub(crate) fn section_value(sections: &[ini::Section], section: &str, key: &str) -> Option<String> {
    sections
        .iter()
        .find(|item| item.name.eq_ignore_ascii_case(section))
        .and_then(|item| {
            item.entries
                .iter()
                .find(|entry| entry.key.eq_ignore_ascii_case(key))
                .map(|entry| entry.value.clone())
        })
}

pub(crate) fn used_cars(sections: &[ini::Section]) -> Vec<String> {
    section_value(sections, "SERVER", "CARS")
        .map(|value| {
            value
                .split(';')
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn used_weather(sections: &[ini::Section]) -> Vec<String> {
    sections
        .iter()
        .filter(|item| item.name.to_uppercase().starts_with("WEATHER_"))
        .filter_map(|item| {
            item.entries
                .iter()
                .find(|entry| entry.key.eq_ignore_ascii_case("GRAPHICS"))
                .map(|entry| entry.value.clone())
        })
        .filter(|value| !value.is_empty())
        .collect()
}

fn subdirs(path: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let candidate = entry.path();
            if candidate.is_dir() {
                dirs.push(candidate);
            }
        }
    }
    dirs.sort();
    dirs
}

fn dir_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

fn read_ui_json(dir: &Path, file: &str) -> serde_json::Value {
    fs::read_to_string(dir.join("ui").join(file))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(serde_json::Value::Null)
}

// ---------------------------------------------------------------- app

#[derive(Serialize)]
pub struct AppInfo {
    pub server_dir: String,
    pub exe_path: String,
    pub has_server: bool,
    pub version: String,
    pub launcher_mode: bool,
    pub profile: String,
    pub profile_name: String,
    pub profiles: Vec<crate::profiles::ProfileEntry>,
}

#[tauri::command]
pub fn app_info(state: State<AppState>) -> AppInfo {
    let dir = state.server_dir();
    let active = state.active_profile();
    let profiles = state.profiles.lock().unwrap().profiles.clone();
    AppInfo {
        server_dir: dir.to_string_lossy().to_string(),
        exe_path: state.exe_path.to_string_lossy().to_string(),
        has_server: dir.join("acServer.exe").is_file(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        launcher_mode: state.launcher_mode,
        profile: active.id,
        profile_name: active.name,
        profiles,
    }
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

/// Non salva mai un token d'invito vuoto: se la configurazione in arrivo non ne
/// ha uno si tiene quello in uso, o se ne genera uno nuovo. Un token vuoto
/// renderebbe ogni invito inutilizzabile (e l'API rifiuta comunque un token
/// atteso vuoto).
fn keep_or_create_token(current: &str, incoming: &mut Settings) -> Result<(), String> {
    if !incoming.invite.token.trim().is_empty() {
        return Ok(());
    }
    incoming.invite.token = if current.trim().is_empty() {
        crate::invite::new_token()?
    } else {
        current.to_string()
    };
    Ok(())
}

#[tauri::command]
pub fn save_settings(state: State<AppState>, mut settings: Settings) -> Result<(), String> {
    {
        let current = state.settings.lock().unwrap();
        keep_or_create_token(&current.invite.token, &mut settings)?;
    }
    *state.settings.lock().unwrap() = settings;
    state.persist()
}

/// Lingua dell'interfaccia: `it` o `en`. Vale anche per i messaggi del backend.
#[tauri::command]
pub fn set_language(state: State<AppState>, language: String) -> Result<Settings, String> {
    let lang = match language.as_str() {
        "it" | "en" => language,
        _ => {
            return Err(messages::tr(
                "lingua non supportata",
                "unsupported language",
            ))
        }
    };
    {
        let mut settings = state.settings.lock().unwrap();
        settings.language = lang.clone();
    }
    state.persist()?;
    crate::messages::set(&lang);
    Ok(state.settings.lock().unwrap().clone())
}

#[tauri::command]
pub fn set_server_dir(state: State<AppState>, dir: String) -> Result<AppInfo, String> {
    let path = PathBuf::from(&dir);
    if !path.join("acServer.exe").is_file() {
        return Err(messages::tr(
            &format!("acServer.exe non trovato in {}", path.display()),
            &format!("acServer.exe not found in {}", path.display()),
        ));
    }
    *state.server_dir.lock().unwrap() = path.clone();
    state.settings.lock().unwrap().server_dir = Some(path.to_string_lossy().to_string());
    *state.profiles.lock().unwrap() = crate::state::load_profiles(&path);
    invalidate_library(&state);
    state.persist()?;
    Ok(app_info(state))
}

// ---------------------------------------------------------------- profili server

fn server_running(state: &AppState) -> bool {
    let mut guard = state.child.lock().unwrap();
    match guard.as_mut() {
        Some(child) => matches!(child.try_wait(), Ok(None)),
        None => false,
    }
}

// --------------------------------------------------------- moduli
mod profiles_cmds;
pub use profiles_cmds::*;
mod server;
pub use server::*;
mod config_cmds;
pub use config_cmds::*;
mod content;
pub use content::*;
mod network;
pub use network::*;
mod content_manager;
pub use content_manager::*;
mod presets;
pub use presets::*;
mod invite_cmds;
pub use invite_cmds::*;
mod join;
pub use join::*;
mod assists_cmds;
pub use assists_cmds::*;

#[cfg(test)]
mod stamp_tests {
    use super::*;

    #[test]
    fn stamp_changes_when_the_file_changes() {
        let dir = std::env::temp_dir().join(format!("acsc-stamp-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("cartella");
        let path = dir.join("entry_list.ini");

        fs::write(&path, "a").expect("scrittura");
        let first = file_stamp(&path);
        std::thread::sleep(std::time::Duration::from_millis(10));
        fs::write(&path, "bb").expect("riscrittura");
        let second = file_stamp(&path);

        assert_ne!(first, second);
        assert_ne!(second, 0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failed_write_leaves_no_temporary_file() {
        let dir = std::env::temp_dir().join(format!("acsc-atomic-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("cartella");
        // la destinazione è una cartella: il rename finale non può riuscire
        let target = dir.join("occupato");
        fs::create_dir_all(target.join("dentro")).expect("cartella");

        assert!(write_text_atomic(&target, "x").is_err());

        let temporaries = fs::read_dir(&dir)
            .expect("dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(temporaries, 0, "nessun .tmp deve restare dopo l'errore");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_empty_token_is_never_saved() {
        // configurazione in arrivo senza token: si tiene quello in uso
        let mut incoming = Settings::default();
        keep_or_create_token("token-corrente", &mut incoming).expect("token mantenuto");
        assert_eq!(incoming.invite.token, "token-corrente");

        // nessun token in uso: se ne genera uno nuovo
        let mut fresh = Settings::default();
        keep_or_create_token("", &mut fresh).expect("token generato");
        assert!(!fresh.invite.token.trim().is_empty());

        // un token esplicito non viene toccato
        let mut explicit = Settings::default();
        explicit.invite.token = "scelto".into();
        keep_or_create_token("altro", &mut explicit).expect("token esplicito");
        assert_eq!(explicit.invite.token, "scelto");
    }
}
