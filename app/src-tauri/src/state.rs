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
    /// Interfaccia su cui ascolta l'API: "0.0.0.0" = tutte.
    #[serde(default = "default_bind")]
    pub api_bind: String,
    /// Scadenza dell'invito (secondi Unix). `None` = nessuna scadenza.
    pub expires_at: Option<i64>,
}

fn default_bind() -> String {
    "0.0.0.0".into()
}

impl Default for InviteSettings {
    fn default() -> Self {
        Self {
            name: "AC Server".into(),
            token: crate::invite::new_token().unwrap_or_default(),
            accent: "#ff5a1f".into(),
            banner: String::new(),
            api_port: 8082,
            api_bind: default_bind(),
            expires_at: None,
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

    /// Scrive `settings.json` in modo atomico. L'errore viene restituito:
    /// l'interfaccia non deve credere salvato ciò che non è finito su disco.
    pub fn persist(&self) -> Result<(), String> {
        let snapshot = self.settings.lock().unwrap().clone();
        let text = serde_json::to_string_pretty(&snapshot)
            .map_err(|error| format!("settings.json non serializzabile: {}", error))?;
        let path = settings_path(&self.exe_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let temp = path.with_file_name(format!("settings.{}.tmp", std::process::id()));
        fs::write(&temp, &text).map_err(|error| format!("{}: {}", temp.display(), error))?;
        let written =
            fs::read_to_string(&temp).map_err(|error| format!("{}: {}", temp.display(), error))?;
        if written != text {
            let _ = fs::remove_file(&temp);
            return Err(crate::messages::tr(
                "scrittura incompleta: impostazioni non salvate",
                "incomplete write: settings not saved",
            ));
        }
        fs::rename(&temp, &path).map_err(|error| {
            let _ = fs::remove_file(&temp);
            format!("{}: {}", path.display(), error)
        })
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
        // alcuni editor salvano con BOM: senza toglierlo il parsing fallisce
        // e l'app ripartirebbe dai valori di fabbrica perdendo le impostazioni.
        let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
        if let Ok(settings) = serde_json::from_str::<Settings>(text) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_settings_without_bind_listen_on_all_interfaces() {
        let json =
            r##"{"name":"AC","token":"abc","accent":"#ff5a1f","banner":"","api_port":8082}"##;
        let settings: InviteSettings = serde_json::from_str(json).expect("json valido");
        assert_eq!(settings.api_bind, "0.0.0.0");
        assert!(settings.expires_at.is_none());
    }

    #[test]
    fn settings_file_with_bom_is_read() {
        let dir = std::env::temp_dir().join(format!("acsc-state-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("cartella temporanea");
        let path = dir.join("settings.json");
        fs::write(&path, "\u{feff}{\"language\":\"en\"}").expect("scrittura");

        let settings = load_settings(&dir.join("ACServerControl.exe"));
        assert_eq!(settings.language, "en");

        let _ = fs::remove_dir_all(&dir);
    }
}
