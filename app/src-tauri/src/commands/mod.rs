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
    time::Instant,
};
use tauri::{AppHandle, Emitter, Manager, State};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

// ---------------------------------------------------------------- helpers

pub(crate) fn read_text(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("{}: {}", path.display(), error))
}

pub(crate) fn write_text(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(path, text).map_err(|error| format!("{}: {}", path.display(), error))
}

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

#[tauri::command]
pub fn save_settings(state: State<AppState>, settings: Settings) -> Result<(), String> {
    *state.settings.lock().unwrap() = settings;
    state.persist();
    Ok(())
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
    state.persist();
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
    state.persist();
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
