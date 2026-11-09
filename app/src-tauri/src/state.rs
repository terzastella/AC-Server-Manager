use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Child,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, OnceLock,
    },
    time::Instant,
};

/// Serializza le scritture di `settings.json`: due salvataggi ravvicinati non
/// devono condividere il file temporaneo.
static PERSIST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static PERSIST_SEQ: AtomicU64 = AtomicU64::new(0);

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

/// Di default l'API ascolta solo su questo PC: per far entrare gli amici si
/// sceglie l'indirizzo della VPN (o la scheda di rete) dalla pagina Invito,
/// invece di esporre l'API su tutte le interfacce senza chiedere.
fn default_bind() -> String {
    "127.0.0.1".into()
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
        let _guard = PERSIST_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let snapshot = self.settings.lock().unwrap().clone();
        let path = settings_path(&self.exe_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }

        // Manager e Launcher sono due processi: prima di scrivere si rilegge il
        // file e si toccano solo i campi di competenza, così nessuno cancella
        // le modifiche dell'altro.
        let _file_lock = SettingsLock::acquire(&path)?;
        let disk = match fs::read_to_string(&path) {
            Ok(text) => {
                let text = text
                    .strip_prefix('\u{feff}')
                    .map(str::to_string)
                    .unwrap_or(text);
                serde_json::from_str::<Settings>(&text).map_err(|error| {
                    crate::messages::tr(
                        &format!(
                            "settings.json non valido ({}): salvataggio annullato",
                            error
                        ),
                        &format!("invalid settings.json ({}): save aborted", error),
                    )
                })?
            }
            Err(error) if error.kind() == ErrorKind::NotFound => Settings::default(),
            Err(error) => return Err(format!("{}: {}", path.display(), error)),
        };
        let merged = merge_for_process(self.launcher_mode, disk, snapshot);

        let text = serde_json::to_string_pretty(&merged)
            .map_err(|error| format!("settings.json non serializzabile: {}", error))?;
        let seq = PERSIST_SEQ.fetch_add(1, Ordering::Relaxed);
        let temp = path.with_file_name(format!("settings.{}.{}.tmp", std::process::id(), seq));
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

/// Campi che appartengono al Launcher: il Manager non li deve sovrascrivere e
/// viceversa. `language` è condiviso, quindi vince chi salva.
fn merge_for_process(launcher: bool, disk: Settings, mine: Settings) -> Settings {
    let mut merged = disk;
    merged.language = mine.language;
    if launcher {
        merged.driver_name = mine.driver_name;
        merged.saved_invites = mine.saved_invites;
    } else {
        merged.server_dir = mine.server_dir;
        merged.game_dir = mine.game_dir;
        merged.cm_dir = mine.cm_dir;
        merged.theme = mine.theme;
        merged.invite = mine.invite;
    }
    merged
}

/// Lock tra processi per `settings.json`.
/// Su Windows è un mutex nominato: esclusione reale, rilasciata dal sistema
/// anche se il processo termina di colpo. Altrove si usa un file di lock.
#[cfg(windows)]
mod settings_lock {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};

    const WAIT_TIMEOUT: u32 = 0x0000_0102;
    const WAIT_FAILED: u32 = 0xFFFF_FFFF;

    pub struct SettingsLock(HANDLE);

    impl SettingsLock {
        pub fn acquire(_target: &std::path::Path) -> Result<Self, String> {
            let name: Vec<u16> = OsStr::new("Local\\ACServerControl-settings")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
            if handle.is_null() {
                return Err("mutex delle impostazioni non creato".into());
            }
            let wait = unsafe { WaitForSingleObject(handle, 2000) };
            if wait == WAIT_TIMEOUT {
                unsafe { CloseHandle(handle) };
                return Err(crate::messages::tr(
                    "impostazioni in uso da un'altra istanza",
                    "settings are being written by another instance",
                ));
            }
            if wait == WAIT_FAILED {
                unsafe { CloseHandle(handle) };
                return Err("attesa del mutex delle impostazioni fallita".into());
            }
            Ok(Self(handle))
        }
    }

    impl Drop for SettingsLock {
        fn drop(&mut self) {
            unsafe {
                ReleaseMutex(self.0);
                CloseHandle(self.0);
            }
        }
    }
}

#[cfg(not(windows))]
mod settings_lock {
    use std::fs;
    use std::io::ErrorKind;
    use std::path::{Path, PathBuf};

    pub struct SettingsLock {
        path: PathBuf,
        token: String,
    }

    impl SettingsLock {
        pub fn acquire(target: &Path) -> Result<Self, String> {
            let path = target.with_file_name("settings.lock");
            let token = format!(
                "{}:{}",
                std::process::id(),
                PERSIST_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            );
            for _ in 0..100 {
                match fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)
                {
                    Ok(mut file) => {
                        use std::io::Write as _;
                        let _ = file.write_all(token.as_bytes());
                        return Ok(Self { path, token });
                    }
                    Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                        let stale = fs::metadata(&path)
                            .and_then(|meta| meta.modified())
                            .ok()
                            .and_then(|time| time.elapsed().ok())
                            .map(|age| age.as_secs() > 10)
                            .unwrap_or(false);
                        if stale {
                            let tomb = path.with_file_name(format!(
                                "settings.lock.stale.{}",
                                super::PERSIST_SEQ
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                            ));
                            let _ = fs::rename(&path, &tomb);
                            let _ = fs::remove_file(&tomb);
                            continue;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(20));
                    }
                    Err(error) => return Err(format!("{}: {}", path.display(), error)),
                }
            }
            Err(crate::messages::tr(
                "impostazioni in uso da un'altra istanza",
                "settings are being written by another instance",
            ))
        }
    }

    impl Drop for SettingsLock {
        fn drop(&mut self) {
            if fs::read_to_string(&self.path)
                .map(|content| content == self.token)
                .unwrap_or(false)
            {
                let _ = fs::remove_file(&self.path);
            }
        }
    }
}

use settings_lock::SettingsLock;

pub fn settings_path(exe: &Path) -> PathBuf {
    exe.parent()
        .unwrap_or_else(|| Path::new("."))
        .join("settings.json")
}

pub fn load_settings(exe: &Path) -> Settings {
    let path = settings_path(exe);
    match fs::read_to_string(&path) {
        Ok(text) => {
            // alcuni editor salvano con BOM: senza toglierlo il parsing fallisce
            let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
            match serde_json::from_str::<Settings>(text) {
                Ok(mut settings) => {
                    // un token vuoto (CSPRNG fallito al primo avvio) renderebbe
                    // inutilizzabile ogni invito: si riprova qui
                    if settings.invite.token.trim().is_empty() {
                        match crate::invite::new_token() {
                            Ok(token) => settings.invite.token = token,
                            Err(error) => eprintln!("token d'invito non generato: {}", error),
                        }
                    }
                    settings
                }
                Err(error) => {
                    // il file resta recuperabile: non si sovrascrive al primo salvataggio
                    let backup = path.with_extension(format!(
                        "json.invalid-{}",
                        chrono::Local::now().format("%Y%m%d-%H%M%S")
                    ));
                    let _ = fs::rename(&path, &backup);
                    eprintln!(
                        "settings.json non valido ({}): copia in {}",
                        error,
                        backup.display()
                    );
                    Settings::default()
                }
            }
        }
        Err(error) => {
            if error.kind() != ErrorKind::NotFound {
                eprintln!("settings.json non leggibile: {}", error);
            }
            Settings::default()
        }
    }
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
    fn old_settings_without_bind_stay_local() {
        let json =
            r##"{"name":"AC","token":"abc","accent":"#ff5a1f","banner":"","api_port":8082}"##;
        let settings: InviteSettings = serde_json::from_str(json).expect("json valido");
        assert_eq!(settings.api_bind, "127.0.0.1");
        assert!(settings.expires_at.is_none());
    }

    #[cfg(windows)]
    #[test]
    fn settings_lock_is_released() {
        let dir = std::env::temp_dir().join(format!("acsc-lock-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("cartella");
        let target = dir.join("settings.json");
        {
            let _lock = SettingsLock::acquire(&target).expect("primo lock");
        }
        let _lock = SettingsLock::acquire(&target).expect("lock dopo il rilascio");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn manager_persist_keeps_launcher_fields() {
        let disk = Settings {
            driver_name: "Luca".into(),
            saved_invites: vec![SavedInvite {
                link: "acservercontrol://join?x".into(),
            }],
            ..Settings::default()
        };
        let mine = Settings {
            theme: "light".into(),
            server_dir: Some("C:\\srv".into()),
            ..Settings::default()
        };

        let merged = merge_for_process(false, disk, mine);
        assert_eq!(merged.driver_name, "Luca");
        assert_eq!(merged.saved_invites.len(), 1);
        assert_eq!(merged.theme, "light");
        assert_eq!(merged.server_dir.as_deref(), Some("C:\\srv"));
    }

    #[test]
    fn launcher_persist_keeps_manager_fields() {
        let disk = Settings {
            theme: "light".into(),
            server_dir: Some("C:\\srv".into()),
            ..Settings::default()
        };
        let mine = Settings {
            driver_name: "Luca".into(),
            saved_invites: vec![SavedInvite {
                link: "acservercontrol://join?x".into(),
            }],
            ..Settings::default()
        };

        let merged = merge_for_process(true, disk, mine);
        assert_eq!(merged.theme, "light");
        assert_eq!(merged.server_dir.as_deref(), Some("C:\\srv"));
        assert_eq!(merged.driver_name, "Luca");
        assert_eq!(merged.saved_invites.len(), 1);
    }

    #[test]
    fn invalid_settings_file_is_preserved() {
        let dir = std::env::temp_dir().join(format!("acsc-bad-settings-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("cartella");
        fs::write(dir.join("settings.json"), "{ non json").expect("scrittura");

        let settings = load_settings(&dir.join("ACServerControl.exe"));
        assert_eq!(settings.language, "it");

        let names: Vec<String> = fs::read_dir(&dir)
            .expect("dir")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect();
        assert!(
            names
                .iter()
                .any(|name| name.starts_with("settings.json.invalid-")),
            "{:?}",
            names
        );

        let _ = fs::remove_dir_all(&dir);
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
