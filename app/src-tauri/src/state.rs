use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Child,
    sync::Mutex,
    time::Instant,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct InviteSettings {
    pub name: String,
    pub token: String,
    pub accent: String,
    pub banner: String,
    pub api_port: u16,
}

impl Default for InviteSettings {
    fn default() -> Self {
        Self {
            name: "AC Server".into(),
            token: crate::invite::new_token(),
            accent: "#ff5a1f".into(),
            banner: String::new(),
            api_port: 8082,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SavedInvite {
    pub link: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub server_dir: Option<String>,
    pub game_dir: Option<String>,
    pub cm_dir: Option<String>,
    pub theme: String,
    pub language: String,
    pub invite: InviteSettings,
    pub driver_name: String,
    pub saved_invites: Vec<SavedInvite>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            server_dir: None,
            game_dir: None,
            cm_dir: None,
            theme: "dark".into(),
            language: "it".into(),
            invite: InviteSettings::default(),
            driver_name: String::new(),
            saved_invites: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub ts: String,
    pub stream: String,
    pub message: String,
}

pub struct AppState {
    pub exe_path: PathBuf,
    pub settings: Mutex<Settings>,
    pub server_dir: Mutex<PathBuf>,
    pub child: Mutex<Option<Child>>,
    pub logs: Mutex<Vec<LogEntry>>,
    pub started: Mutex<Option<Instant>>,
    pub current_log: Mutex<Option<String>>,
    pub image_cache: Mutex<HashMap<String, String>>,
    pub library_cache: Mutex<Option<(Instant, crate::commands::ContentLibrary)>>,
    pub join_uri: Mutex<Option<String>>,
    pub launcher_mode: bool,
    pub profiles: Mutex<crate::profiles::ProfileIndex>,
}

impl AppState {
    pub fn server_dir(&self) -> PathBuf {
        self.server_dir.lock().unwrap().clone()
    }

    pub fn active_profile(&self) -> crate::profiles::ProfileEntry {
        let index = self.profiles.lock().unwrap();
        index
            .profiles
            .iter()
            .find(|entry| entry.id == index.active)
            .cloned()
            .or_else(|| index.profiles.first().cloned())
            .unwrap_or_else(|| crate::profiles::ProfileEntry {
                id: "default".into(),
                name: "Predefinito".into(),
                slug: String::new(),
                legacy: true,
                created: String::new(),
            })
    }

    pub fn cfg_dir(&self) -> PathBuf {
        crate::profiles::cfg_dir(&self.server_dir(), &self.active_profile())
    }

    pub fn active_profile_dir(&self) -> PathBuf {
        crate::profiles::profile_dir(&self.server_dir(), &self.active_profile())
    }

    pub fn persist(&self) {
        let snapshot = self.settings.lock().unwrap().clone();
        if let Ok(text) = serde_json::to_string_pretty(&snapshot) {
            let path = settings_path(&self.exe_path);
            let temp = path.with_extension("json.tmp");
            if fs::write(&temp, text).is_ok() {
                let _ = fs::rename(&temp, &path);
            }
        }
    }
}

pub fn settings_path(exe: &Path) -> PathBuf {
    exe.parent()
        .unwrap_or_else(|| Path::new("."))
        .join("settings.json")
}

pub fn load_settings(exe: &Path) -> Settings {
    let path = settings_path(exe);
    if let Ok(text) = fs::read_to_string(path) {
        if let Ok(settings) = serde_json::from_str::<Settings>(&text) {
            return settings;
        }
    }
    Settings::default()
}

/// Profili del server: indice in `<root>\profiles\index.json`, con il profilo
/// `default` che punta alla `cfg/` della radice.
pub fn load_profiles(root: &Path) -> crate::profiles::ProfileIndex {
    crate::profiles::load(root)
}

/// Walks up from the executable looking for acServer.exe, so the tool works
/// from the server folder, from `dist\` or from `src-tauri\target\release`.
pub fn detect_server_dir(exe: &Path) -> Option<PathBuf> {
    let mut current = exe.parent();
    for _ in 0..8 {
        let dir = current?;
        if dir.join("acServer.exe").is_file() {
            return Some(dir.to_path_buf());
        }
        let nested = dir.join("server");
        if nested.join("acServer.exe").is_file() {
            return Some(nested);
        }
        current = dir.parent();
    }
    None
}
