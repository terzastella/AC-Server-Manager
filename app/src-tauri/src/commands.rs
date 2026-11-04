use crate::cm;
use crate::messages;
use crate::ini;
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
        _ => return Err(messages::tr("lingua non supportata", "unsupported language")),
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

#[tauri::command]
pub fn list_profiles(state: State<AppState>) -> crate::profiles::ProfileIndex {
    state.profiles.lock().unwrap().clone()
}

#[tauri::command]
pub fn create_profile(
    state: State<AppState>,
    name: String,
) -> Result<crate::profiles::ProfileIndex, String> {
    if server_running(&state) {
        return Err(messages::tr("ferma il server prima di creare un profilo", "stop the server before creating a profile"));
    }
    let root = state.server_dir();
    let mut index = state.profiles.lock().unwrap();
    let active = index.active.clone();
    let from = index
        .profiles
        .iter()
        .find(|entry| entry.id == active)
        .cloned()
        .or_else(|| index.profiles.first().cloned())
        .ok_or_else(|| "nessun profilo disponibile".to_string())?;
    let entry = crate::profiles::create(&root, &mut index, &name, &from)?;
    index.active = entry.id.clone();
    crate::profiles::save(&root, &index)?;
    Ok(index.clone())
}

#[tauri::command]
pub fn import_profile(
    state: State<AppState>,
    path: String,
    name: String,
) -> Result<crate::profiles::ProfileIndex, String> {
    if server_running(&state) {
        return Err(messages::tr("ferma il server prima di importare un profilo", "stop the server before importing a profile"));
    }
    let folder = PathBuf::from(path.trim());
    if !folder.is_dir() {
        return Err(messages::tr("cartella non trovata", "folder not found"));
    }
    let cfg = if folder.join("cfg").join("server_cfg.ini").is_file() {
        folder.join("cfg")
    } else if folder.join("server_cfg.ini").is_file() {
        folder.clone()
    } else {
        return Err(messages::tr("nella cartella non c'è cfg/server_cfg.ini", "cfg/server_cfg.ini not found in that folder"));
    };
    let root = state.server_dir();
    let mut index = state.profiles.lock().unwrap();
    let entry = crate::profiles::import_from(&root, &mut index, &cfg, &name)?;
    index.active = entry.id.clone();
    crate::profiles::save(&root, &index)?;
    Ok(index.clone())
}

#[tauri::command]
pub fn duplicate_profile(
    state: State<AppState>,
    id: String,
) -> Result<crate::profiles::ProfileIndex, String> {
    if server_running(&state) {
        return Err(messages::tr("ferma il server prima di duplicare un profilo", "stop the server before duplicating a profile"));
    }
    let root = state.server_dir();
    let mut index = state.profiles.lock().unwrap();
    crate::profiles::duplicate(&root, &mut index, &id)?;
    Ok(index.clone())
}

#[tauri::command]
pub fn rename_profile(
    state: State<AppState>,
    id: String,
    name: String,
) -> Result<crate::profiles::ProfileIndex, String> {
    let root = state.server_dir();
    let mut index = state.profiles.lock().unwrap();
    crate::profiles::rename(&mut index, &id, &name)?;
    crate::profiles::save(&root, &index)?;
    Ok(index.clone())
}

#[tauri::command]
pub fn delete_profile(
    state: State<AppState>,
    id: String,
) -> Result<crate::profiles::ProfileIndex, String> {
    if server_running(&state) {
        return Err(messages::tr("ferma il server prima di eliminare un profilo", "stop the server before deleting a profile"));
    }
    let root = state.server_dir();
    let mut index = state.profiles.lock().unwrap();
    crate::profiles::delete(&root, &mut index, &id)?;
    crate::profiles::save(&root, &index)?;
    Ok(index.clone())
}

#[tauri::command]
pub fn set_active_profile(
    state: State<AppState>,
    id: String,
) -> Result<crate::profiles::ProfileIndex, String> {
    if server_running(&state) {
        return Err(messages::tr("ferma il server prima di cambiare profilo", "stop the server before switching profile"));
    }
    let root = state.server_dir();
    let mut index = state.profiles.lock().unwrap();
    if !index.profiles.iter().any(|entry| entry.id == id) {
        return Err(messages::tr(&format!("server {} non trovato", id), &format!("server {} not found", id)));
    }
    index.active = id;
    crate::profiles::save(&root, &index)?;
    Ok(index.clone())
}

// ---------------------------------------------------------------- server

#[derive(Serialize, Clone)]
pub struct ServerStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub uptime_secs: u64,
    pub server_dir: String,
    pub log_file: Option<String>,
}

fn status_of(state: &AppState) -> ServerStatus {
    let mut guard = state.child.lock().unwrap();
    let exited = match guard.as_mut() {
        Some(child) => matches!(child.try_wait(), Ok(Some(_))),
        None => false,
    };
    if exited {
        *guard = None;
        *state.started.lock().unwrap() = None;
    }
    let running = guard.is_some();
    let pid = guard.as_ref().map(|child| child.id());
    drop(guard);

    ServerStatus {
        running,
        pid,
        uptime_secs: state
            .started
            .lock()
            .unwrap()
            .map(|started| started.elapsed().as_secs())
            .unwrap_or(0),
        server_dir: state.server_dir().to_string_lossy().to_string(),
        log_file: state.current_log.lock().unwrap().clone(),
    }
}

#[tauri::command]
pub fn server_status(state: State<AppState>) -> ServerStatus {
    status_of(&state)
}

fn pump<R: std::io::Read + Send + 'static>(
    app: AppHandle,
    reader: R,
    log_path: PathBuf,
    stream: &'static str,
) {
    std::thread::spawn(move || {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .ok();
        let reader = BufReader::new(reader);
        for line in reader.lines().map_while(Result::ok) {
            if line.trim().is_empty() {
                continue;
            }
            let entry = LogEntry {
                ts: chrono::Local::now().format("%H:%M:%S").to_string(),
                stream: stream.to_string(),
                message: line,
            };
            if let Some(file) = file.as_mut() {
                let _ = writeln!(file, "[{}] {} {}", entry.ts, entry.stream, entry.message);
            }
            {
                let state = app.state::<AppState>();
                let mut logs = state.logs.lock().unwrap();
                logs.push(entry.clone());
                if logs.len() > 5000 {
                    let excess = logs.len() - 5000;
                    logs.drain(0..excess);
                }
            }
            let _ = app.emit("server-log", &entry);
        }
    });
}

/// PIDs dei processi `acServer.exe` avviati fuori da questa app. Serve perché
/// un server rimasto aperto tiene occupate 9600/8081 e fa andare in panic il
/// nuovo processo (`Error listening`), senza che l'interfaccia capisca niente.
fn external_server_pids() -> Vec<u32> {
    #[cfg(windows)]
    let mut command: Command = {
        let mut command = Command::new("tasklist");
        command.args(["/FI", "IMAGENAME eq acServer.exe", "/FO", "CSV", "/NH"]);
        command.creation_flags(CREATE_NO_WINDOW);
        command
    };
    #[cfg(not(windows))]
    let mut command: Command = {
        let mut command = Command::new("tasklist");
        command.args(["/FI", "IMAGENAME eq acServer.exe", "/FO", "CSV", "/NH"]);
        command
    };
    let Ok(output) = command.output() else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| line.to_lowercase().contains("acserver.exe"))
        .filter_map(|line| {
            let columns: Vec<&str> = line.split("\",\"").collect();
            columns
                .get(1)
                .and_then(|value| value.trim().trim_matches('"').parse::<u32>().ok())
        })
        .collect()
}

#[tauri::command]
pub fn stop_external_server() -> Result<usize, String> {
    let pids = external_server_pids();
    for pid in &pids {
        let mut command = Command::new("taskkill");
        command.args(["/PID", &pid.to_string(), "/F"]);
        #[cfg(windows)]
        command.creation_flags(CREATE_NO_WINDOW);
        let _ = command.output();
    }
    Ok(pids.len())
}

#[tauri::command]
pub fn start_server(app: AppHandle, state: State<AppState>) -> Result<ServerStatus, String> {
    {
        let mut guard = state.child.lock().unwrap();
        let exited = match guard.as_mut() {
            Some(child) => matches!(child.try_wait(), Ok(Some(_))),
            None => false,
        };
        if exited {
            *guard = None;
            *state.started.lock().unwrap() = None;
        }
        if guard.is_some() {
            drop(guard);
            return Ok(status_of(&state));
        }
    }

    let profile = state.active_profile();
    let dir = state.active_profile_dir();
    let exe = state.server_dir().join("acServer.exe");
    if !exe.is_file() {
        return Err(messages::tr("acServer.exe non trovato nella cartella server", "acServer.exe not found in the server folder"));
    }
    if !profile.legacy {
        crate::profiles::ensure_content_link(&state.server_dir(), &profile)?;
    }

    let external = external_server_pids();
    if !external.is_empty() {
        let list = external
            .iter()
            .map(|pid| pid.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "c'è già un server AC in esecuzione (PID {}), avviato fuori da questa app: fermalo prima di avviarne un altro.",
            list
        ));
    }
    let logs = state.server_dir().join("logs");
    fs::create_dir_all(&logs).map_err(|error| error.to_string())?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let tag = if profile.slug.is_empty() { "default".to_string() } else { profile.slug.clone() };
    let log_path = logs.join(format!("acserver-{}-{}.log", tag, stamp));
    fs::write(&log_path, "").map_err(|error| error.to_string())?;

    let mut command = Command::new(&exe);
    command
        .current_dir(&dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    let mut child = command
        .spawn()
        .map_err(|error| format!("avvio fallito: {}", error))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    *state.child.lock().unwrap() = Some(child);
    *state.started.lock().unwrap() = Some(Instant::now());
    *state.current_log.lock().unwrap() = Some(log_path.to_string_lossy().to_string());

    if let Some(stdout) = stdout {
        pump(app.clone(), stdout, log_path.clone(), "out");
    }
    if let Some(stderr) = stderr {
        pump(app, stderr, log_path, "err");
    }

    Ok(status_of(&state))
}

#[tauri::command]
pub fn stop_server(state: State<AppState>) -> ServerStatus {
    {
        let mut guard = state.child.lock().unwrap();
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
    *state.started.lock().unwrap() = None;
    status_of(&state)
}

#[tauri::command]
pub fn restart_server(app: AppHandle, state: State<AppState>) -> Result<ServerStatus, String> {
    {
        let mut guard = state.child.lock().unwrap();
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
    *state.started.lock().unwrap() = None;
    std::thread::sleep(std::time::Duration::from_millis(600));
    start_server(app, state)
}

// ---------------------------------------------------------------- logs

#[tauri::command]
pub fn get_logs(state: State<AppState>) -> Vec<LogEntry> {
    state.logs.lock().unwrap().clone()
}

#[tauri::command]
pub fn clear_logs(state: State<AppState>) {
    state.logs.lock().unwrap().clear();
}

#[derive(Serialize)]
pub struct LogFile {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified: u64,
}

#[tauri::command]
pub fn list_logs(state: State<AppState>) -> Vec<LogFile> {
    let dir = state.server_dir().join("logs");
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path
                .extension()
                .map(|ext| ext.eq_ignore_ascii_case("log"))
                .unwrap_or(false)
            {
                continue;
            }
            let meta = entry.metadata().ok();
            files.push(LogFile {
                name: dir_name(&path),
                path: path.to_string_lossy().to_string(),
                size: meta.as_ref().map(|item| item.len()).unwrap_or(0),
                modified: meta
                    .and_then(|item| item.modified().ok())
                    .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|duration| duration.as_secs())
                    .unwrap_or(0),
            });
        }
    }
    files.sort_by(|a, b| b.modified.cmp(&a.modified));
    files
}

#[tauri::command]
pub fn tail_log(state: State<AppState>, file: String, lines: usize) -> Result<String, String> {
    let logs_dir = state.server_dir().join("logs");
    let target = PathBuf::from(&file)
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let logs_dir = logs_dir.canonicalize().unwrap_or(logs_dir);
    if !target.starts_with(&logs_dir) {
        return Err(messages::tr("file fuori dalla cartella logs", "file outside the logs folder"));
    }
    let text = read_text(&target)?;
    let all: Vec<&str> = text.lines().collect();
    let start = all.len().saturating_sub(lines.max(1));
    Ok(all[start..].join("\n"))
}

// ---------------------------------------------------------------- config

pub(crate) fn backup_cfg(state: &AppState) -> Result<String, String> {
    let dir = state.server_dir();
    let profile = state.active_profile();
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let tag = if profile.slug.is_empty() { "default".to_string() } else { profile.slug.clone() };
    let destination = dir.join("backups").join(format!("cfg-{}-{}", tag, stamp));
    fs::create_dir_all(&destination).map_err(|error| error.to_string())?;
    let source = state.cfg_dir();
    for name in ["server_cfg.ini", "entry_list.ini", "welcome.txt"] {
        let from = source.join(name);
        if from.is_file() {
            fs::copy(&from, destination.join(name)).map_err(|error| error.to_string())?;
        }
    }
    Ok(destination.to_string_lossy().to_string())
}

#[tauri::command]
pub fn backup_now(state: State<AppState>) -> Result<String, String> {
    backup_cfg(&state)
}

#[tauri::command]
pub fn read_config(state: State<AppState>) -> Result<ini::Doc, String> {
    read_text(&state.cfg_dir().join("server_cfg.ini")).map(|text| ini::parse(&text))
}

#[tauri::command]
pub fn save_config(
    state: State<AppState>,
    sections: Vec<ini::Section>,
) -> Result<String, String> {
    let path = state.cfg_dir().join("server_cfg.ini");
    let base = read_text(&path)?;
    let backup = backup_cfg(&state).unwrap_or_default();
    let text = ini::apply(&base, &sections);
    write_text(&path, &text)?;
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

#[tauri::command]
pub fn save_entry_list(state: State<AppState>, slots: Vec<CarSlot>) -> Result<String, String> {
    let path = state.cfg_dir().join("entry_list.ini");
    let base = read_text(&path).unwrap_or_default();
    let newline = if base.contains("\r\n") { "\r\n" } else { "\n" };
    let backup = backup_cfg(&state).unwrap_or_default();

    let mut out = String::new();
    for line in base.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            break;
        }
        if !trimmed.is_empty() {
            out.push_str(line.trim_end_matches('\r'));
            out.push_str(newline);
        }
    }
    for (index, slot) in slots.iter().enumerate() {
        out.push_str(&format!("[CAR_{}]{}", index, newline));
        out.push_str(&format!("DRIVERNAME={}{}", slot.driver_name, newline));
        out.push_str(&format!("TEAM={}{}", slot.team, newline));
        out.push_str(&format!("MODEL={}{}", slot.model, newline));
        out.push_str(&format!("SKIN={}{}", slot.skin, newline));
        out.push_str(&format!("SPECTATOR_MODE={}{}", slot.spectator_mode, newline));
        out.push_str(&format!("GUID={}{}", slot.guid, newline));
        out.push_str(&format!("BALLAST={}{}", slot.ballast, newline));
        out.push_str(&format!("RESTRICTOR={}{}", slot.restrictor, newline));
        out.push_str(newline);
    }
    write_text(&path, &out)?;
    Ok(backup)
}

#[tauri::command]
pub fn read_welcome(state: State<AppState>) -> Result<String, String> {
    read_text(&state.cfg_dir().join("welcome.txt"))
}

#[tauri::command]
pub fn save_welcome(state: State<AppState>, text: String) -> Result<(), String> {
    write_text(&state.cfg_dir().join("welcome.txt"), &text)
}

// ---------------------------------------------------------------- content

#[derive(Serialize)]
pub struct CarInfo {
    pub id: String,
    pub name: String,
    pub brand: String,
    pub skins: Vec<String>,
    pub used: bool,
}

#[derive(Serialize)]
pub struct TrackInfo {
    pub id: String,
    pub name: String,
    pub used: bool,
}

#[derive(Serialize)]
pub struct WeatherInfo {
    pub id: String,
    pub used: bool,
}

#[derive(Serialize)]
pub struct ContentReport {
    pub cars: Vec<CarInfo>,
    pub tracks: Vec<TrackInfo>,
    pub weather: Vec<WeatherInfo>,
    pub used_cars: Vec<String>,
    pub used_tracks: Vec<String>,
    pub missing_cars: Vec<String>,
    pub missing_tracks: Vec<String>,
    pub missing_weather: Vec<String>,
    pub game_dir: Option<String>,
}

fn build_report(state: &AppState) -> ContentReport {
    let dir = state.server_dir();
    let sections = config_sections(state);
    let used_cars = used_cars(&sections);
    let used_track = section_value(&sections, "SERVER", "TRACK").unwrap_or_default();
    let used_weather = used_weather(&sections);

    let mut cars = Vec::new();
    for path in subdirs(&dir.join("content").join("cars")) {
        let id = dir_name(&path);
        let ui = read_ui_json(&path, "ui_car.json");
        let skins = subdirs(&path.join("skins"))
            .iter()
            .map(|item| dir_name(item))
            .collect();
        cars.push(CarInfo {
            used: used_cars.iter().any(|item| item.eq_ignore_ascii_case(&id)),
            name: ui
                .get("name")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .to_string(),
            brand: ui
                .get("brand")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .to_string(),
            id,
            skins,
        });
    }

    let mut tracks = Vec::new();
    for path in subdirs(&dir.join("content").join("tracks")) {
        let id = dir_name(&path);
        let ui = read_ui_json(&path, "ui_track.json");
        tracks.push(TrackInfo {
            used: id.eq_ignore_ascii_case(&used_track),
            name: ui
                .get("name")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .to_string(),
            id,
        });
    }

    let mut weather = Vec::new();
    for path in subdirs(&dir.join("content").join("weather")) {
        let id = dir_name(&path);
        if !path.join("weather.ini").is_file() {
            continue;
        }
        weather.push(WeatherInfo {
            used: used_weather.iter().any(|item| item.eq_ignore_ascii_case(&id)),
            id,
        });
    }

    let missing_cars = used_cars
        .iter()
        .filter(|id| !dir.join("content").join("cars").join(id.as_str()).is_dir())
        .cloned()
        .collect();
    let missing_tracks = if !used_track.is_empty()
        && !dir.join("content").join("tracks").join(&used_track).is_dir()
    {
        vec![used_track.clone()]
    } else {
        Vec::new()
    };
    let missing_weather = used_weather
        .iter()
        .filter(|id| !dir.join("content").join("weather").join(id.as_str()).is_dir())
        .cloned()
        .collect();

    ContentReport {
        cars,
        tracks,
        weather,
        used_cars,
        used_tracks: vec![used_track],
        missing_cars,
        missing_tracks,
        missing_weather,
        game_dir: state.settings.lock().unwrap().game_dir.clone(),
    }
}

#[tauri::command]
pub fn scan_content(state: State<AppState>) -> ContentReport {
    build_report(&state)
}

fn copy_tree(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

#[derive(Serialize)]
pub struct CopyReport {
    pub copied: Vec<String>,
    pub skipped: Vec<String>,
    pub errors: Vec<String>,
}

#[tauri::command]
pub fn copy_content(state: State<AppState>, game_dir: String) -> Result<CopyReport, String> {
    let game = PathBuf::from(&game_dir);
    let content = if game.join("content").is_dir() {
        game.join("content")
    } else {
        game.clone()
    };
    let cars_root = content.join("cars");
    let tracks_root = content.join("tracks");
    let weather_root = content.join("weather");
    if !cars_root.is_dir() || !tracks_root.is_dir() {
        return Err(format!(
            "cartella content non valida: {}",
            content.display()
        ));
    }

    {
        state.settings.lock().unwrap().game_dir = Some(game_dir.clone());
        state.persist();
    }
    invalidate_library(&state);

    let sections = config_sections(&state);
    let dir = state.server_dir();
    let mut report = CopyReport {
        copied: Vec::new(),
        skipped: Vec::new(),
        errors: Vec::new(),
    };

    let copy_one = |source: PathBuf, destination: PathBuf, report: &mut CopyReport| {
        if destination.exists() {
            report.skipped.push(dir_name(&destination));
            return;
        }
        match copy_tree(&source, &destination) {
            Ok(()) => report.copied.push(dir_name(&destination)),
            Err(error) => report.errors.push(format!(
                "{}: {}",
                dir_name(&destination),
                error
            )),
        }
    };

    for id in used_cars(&sections) {
        copy_one(
            cars_root.join(&id),
            dir.join("content").join("cars").join(&id),
            &mut report,
        );
    }
    if let Some(track) = section_value(&sections, "SERVER", "TRACK") {
        if !track.is_empty() {
            copy_one(
                tracks_root.join(&track),
                dir.join("content").join("tracks").join(&track),
                &mut report,
            );
        }
    }
    for id in used_weather(&sections) {
        copy_one(
            weather_root.join(&id),
            dir.join("content").join("weather").join(&id),
            &mut report,
        );
    }

    Ok(report)
}

// ---------------------------------------------------------------- content library (visual browser)

#[derive(Serialize, Clone)]
pub struct SkinEntry {
    pub name: String,
    pub display: String,
    pub preview: Option<String>,
    pub team: String,
    pub country: String,
    pub number: String,
}

#[derive(Serialize, Clone)]
pub struct CarEntry {
    pub id: String,
    pub name: String,
    pub brand: String,
    pub class: String,
    pub tags: Vec<String>,
    pub specs: serde_json::Value,
    pub preview: Option<String>,
    pub badge: Option<String>,
    pub skins: Vec<SkinEntry>,
    pub in_server: bool,
    pub in_game: bool,
    pub used: bool,
    pub server_path: Option<String>,
    pub game_path: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct TrackEntry {
    pub id: String,
    pub name: String,
    pub country: String,
    pub city: String,
    pub length: String,
    pub tags: Vec<String>,
    pub preview: Option<String>,
    pub outline: Option<String>,
    pub in_server: bool,
    pub in_game: bool,
    pub used: bool,
    pub server_path: Option<String>,
    pub game_path: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct ContentLibrary {
    pub cars: Vec<CarEntry>,
    pub tracks: Vec<TrackEntry>,
    pub game_dir: Option<String>,
}

fn read_json(path: &Path) -> serde_json::Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(serde_json::Value::Null)
}

fn collect_dirs(root: &Path) -> std::collections::BTreeMap<String, PathBuf> {
    let mut map = std::collections::BTreeMap::new();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                map.insert(dir_name(&path), path);
            }
        }
    }
    map
}

fn first_existing(candidates: &[PathBuf]) -> Option<String> {
    candidates
        .iter()
        .find(|path| path.is_file())
        .map(|path| path.to_string_lossy().to_string())
}

fn json_text(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|item| item.as_str())
        .unwrap_or("")
        .to_string()
}

fn json_tags(value: &serde_json::Value) -> Vec<String> {
    value
        .get("tags")
        .and_then(|item| item.as_array())
        .map(|list| {
            list.iter()
                .filter_map(|tag| tag.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

#[tauri::command]
pub fn content_library(state: State<AppState>) -> ContentLibrary {
    content_library_impl(&state)
}

/// La libreria costa ~0,8 s di disco con ~470 auto e ~3.000 skin. Le richieste
/// dell'API (una per invito del launcher) e la finestra di iscrizione la
/// chiedono in parallelo: si tiene in cache per 30 s e si ricalcola solo se
/// scade o se i contenuti cambiano.
const LIBRARY_TTL: std::time::Duration = std::time::Duration::from_secs(30);

pub(crate) fn invalidate_library(state: &AppState) {
    *state.library_cache.lock().unwrap() = None;
}

pub(crate) fn content_library_impl(state: &AppState) -> ContentLibrary {
    let cached = {
        let cache = state.library_cache.lock().unwrap();
        cache
            .as_ref()
            .filter(|(at, _)| at.elapsed() < LIBRARY_TTL)
            .map(|(_, library)| library.clone())
    };
    if let Some(mut library) = cached {
        // Le auto/pista "in uso" dipendono da server_cfg.ini, che cambia senza
        // invalidare i contenuti: si ricalcolano al volo, costa nulla.
        let sections = config_sections(state);
        let used = used_cars(&sections);
        let used_track = section_value(&sections, "SERVER", "TRACK").unwrap_or_default();
        for car in library.cars.iter_mut() {
            car.used = used.iter().any(|id| id.eq_ignore_ascii_case(&car.id));
        }
        for track in library.tracks.iter_mut() {
            track.used = track.id.eq_ignore_ascii_case(&used_track);
        }
        return library;
    }

    let library = build_library(state);
    *state.library_cache.lock().unwrap() = Some((Instant::now(), library.clone()));
    library
}

fn build_library(state: &AppState) -> ContentLibrary {
    let server_root = state.server_dir().join("content");
    let game_root = {
        let configured = state.settings.lock().unwrap().game_dir.clone();
        let detected = configured
            .filter(|dir| Path::new(dir.as_str()).join("content").is_dir())
            .or_else(|| {
                crate::cm::locate::locate(None)
                    .ac_root
                    .filter(|dir| Path::new(dir.as_str()).join("content").is_dir())
            });
        if let Some(dir) = &detected {
            let mut settings = state.settings.lock().unwrap();
            if settings.game_dir.as_deref() != Some(dir.as_str()) {
                settings.game_dir = Some(dir.clone());
                drop(settings);
                state.persist();
            }
        }
        detected.map(PathBuf::from)
    };

    let sections = config_sections(&state);
    let used = used_cars(&sections);
    let used_track = section_value(&sections, "SERVER", "TRACK").unwrap_or_default();

    let server_cars = collect_dirs(&server_root.join("cars"));
    let game_cars = game_root
        .as_ref()
        .map(|root| collect_dirs(&root.join("content").join("cars")))
        .unwrap_or_default();

    let mut car_ids: Vec<String> = server_cars.keys().chain(game_cars.keys()).cloned().collect();
    car_ids.sort();
    car_ids.dedup();

    let mut cars = Vec::new();
    for id in car_ids {
        let server_path = server_cars.get(&id).cloned();
        let game_path = game_cars.get(&id).cloned();
        let ui = game_path
            .as_ref()
            .map(|path| read_json(&path.join("ui").join("ui_car.json")))
            .filter(|value| !value.is_null())
            .or_else(|| {
                server_path
                    .as_ref()
                    .map(|path| read_json(&path.join("ui").join("ui_car.json")))
            })
            .unwrap_or(serde_json::Value::Null);

        let mut skin_map: std::collections::BTreeMap<String, SkinEntry> =
            std::collections::BTreeMap::new();
        for (root, _is_game) in [(&game_path, true), (&server_path, false)] {
            let Some(root) = root.as_ref() else { continue };
            for skin_dir in subdirs(&root.join("skins")) {
                let name = dir_name(&skin_dir);
                let skin_json = read_json(&skin_dir.join("ui_skin.json"));
                let display = {
                    let value = json_text(&skin_json, "skinname");
                    if value.is_empty() { name.clone() } else { value }
                };
                let preview = first_existing(&[
                    skin_dir.join("preview.jpg"),
                    skin_dir.join("preview.png"),
                    skin_dir.join("livery.png"),
                ]);
                let existing = skin_map.get(&name).cloned();

                let merged = SkinEntry {
                    name: name.clone(),
                    display: existing
                        .as_ref()
                        .map(|entry| entry.display.clone())
                        .filter(|value| !value.is_empty())
                        .unwrap_or(display),
                    preview: existing
                        .as_ref()
                        .and_then(|entry| entry.preview.clone())
                        .or(preview),
                    team: existing
                        .as_ref()
                        .map(|entry| entry.team.clone())
                        .filter(|value| !value.is_empty())
                        .unwrap_or_else(|| json_text(&skin_json, "team")),
                    country: existing
                        .as_ref()
                        .map(|entry| entry.country.clone())
                        .filter(|value| !value.is_empty())
                        .unwrap_or_else(|| json_text(&skin_json, "country")),
                    number: existing
                        .as_ref()
                        .map(|entry| entry.number.clone())
                        .filter(|value| !value.is_empty())
                        .unwrap_or_else(|| json_text(&skin_json, "number")),
                };
                skin_map.insert(name, merged);
            }
        }
        let mut skins: Vec<SkinEntry> = skin_map.into_values().collect();
        skins.sort_by(|a, b| a.display.to_lowercase().cmp(&b.display.to_lowercase()));

        let badge = first_existing(&[
            game_path.as_ref().map(|p| p.join("ui").join("badge.png")).unwrap_or_default(),
            server_path.as_ref().map(|p| p.join("ui").join("badge.png")).unwrap_or_default(),
        ]);
        let preview = skins
            .iter()
            .find_map(|skin| skin.preview.clone())
            .or_else(|| badge.clone())
            .or_else(|| {
                first_existing(&[
                    game_path.as_ref().map(|p| p.join("ui").join("preview.jpg")).unwrap_or_default(),
                    server_path.as_ref().map(|p| p.join("ui").join("preview.jpg")).unwrap_or_default(),
                ])
            });

        cars.push(CarEntry {
            used: used.iter().any(|item| item.eq_ignore_ascii_case(&id)),
            name: if json_text(&ui, "name").is_empty() { id.clone() } else { json_text(&ui, "name") },
            brand: json_text(&ui, "brand"),
            class: json_text(&ui, "class"),
            tags: json_tags(&ui),
            specs: ui.get("specs").cloned().unwrap_or(serde_json::Value::Null),
            preview,
            badge,
            skins,
            in_server: server_path.is_some(),
            in_game: game_path.is_some(),
            server_path: server_path.map(|p| p.to_string_lossy().to_string()),
            game_path: game_path.map(|p| p.to_string_lossy().to_string()),
            id,
        });
    }

    let server_tracks = collect_dirs(&server_root.join("tracks"));
    let game_tracks = game_root
        .as_ref()
        .map(|root| collect_dirs(&root.join("content").join("tracks")))
        .unwrap_or_default();
    let mut track_ids: Vec<String> =
        server_tracks.keys().chain(game_tracks.keys()).cloned().collect();
    track_ids.sort();
    track_ids.dedup();

    let mut tracks = Vec::new();
    for id in track_ids {
        let server_path = server_tracks.get(&id).cloned();
        let game_path = game_tracks.get(&id).cloned();
        let ui = game_path
            .as_ref()
            .map(|path| read_json(&path.join("ui").join("ui_track.json")))
            .filter(|value| !value.is_null())
            .or_else(|| {
                server_path
                    .as_ref()
                    .map(|path| read_json(&path.join("ui").join("ui_track.json")))
            })
            .unwrap_or(serde_json::Value::Null);

        let preview = first_existing(&[
            game_path.as_ref().map(|p| p.join("ui").join("preview.png")).unwrap_or_default(),
            game_path.as_ref().map(|p| p.join("ui").join("preview.jpg")).unwrap_or_default(),
            server_path.as_ref().map(|p| p.join("ui").join("preview.png")).unwrap_or_default(),
        ]);
        let outline = first_existing(&[
            game_path.as_ref().map(|p| p.join("ui").join("outline.png")).unwrap_or_default(),
            server_path.as_ref().map(|p| p.join("ui").join("outline.png")).unwrap_or_default(),
        ]);

        tracks.push(TrackEntry {
            used: id.eq_ignore_ascii_case(&used_track),
            name: if json_text(&ui, "name").is_empty() { id.clone() } else { json_text(&ui, "name") },
            country: json_text(&ui, "country"),
            city: json_text(&ui, "city"),
            length: json_text(&ui, "length"),
            tags: json_tags(&ui),
            preview,
            outline,
            in_server: server_path.is_some(),
            in_game: game_path.is_some(),
            server_path: server_path.map(|p| p.to_string_lossy().to_string()),
            game_path: game_path.map(|p| p.to_string_lossy().to_string()),
            id,
        });
    }

    ContentLibrary {
        cars,
        tracks,
        game_dir: game_root.map(|path| path.to_string_lossy().to_string()),
    }
}

#[tauri::command]
pub fn content_image(state: State<AppState>, path: String, width: u32) -> Result<String, String> {
    let requested = PathBuf::from(&path)
        .canonicalize()
        .map_err(|error| format!("immagine non trovata: {}", error))?;

    let server_content = state.server_dir().join("content").canonicalize().ok();
    let game_content = state
        .settings
        .lock()
        .unwrap()
        .game_dir
        .clone()
        .map(PathBuf::from)
        .and_then(|dir| dir.join("content").canonicalize().ok());

    let allowed = server_content
        .as_ref()
        .map(|root| requested.starts_with(root))
        .unwrap_or(false)
        || game_content
            .as_ref()
            .map(|root| requested.starts_with(root))
            .unwrap_or(false);
    if !allowed {
        return Err(messages::tr("immagine fuori dalle cartelle content", "image outside the content folders"));
    }

    let width = width.clamp(64, 2000);
    let key = format!("{}|{}", requested.to_string_lossy(), width);
    if let Some(hit) = state.image_cache.lock().unwrap().get(&key) {
        return Ok(hit.clone());
    }

    let encoded = render_thumbnail(&requested, width)?;
    let mut cache = state.image_cache.lock().unwrap();
    if cache.len() > 600 {
        cache.clear();
    }
    cache.insert(key, encoded.clone());
    Ok(encoded)
}

fn render_thumbnail(path: &Path, width: u32) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let image = image::load_from_memory(&bytes)
        .map_err(|error| format!("formato immagine non supportato: {}", error))?;
    let thumb = image.thumbnail(width, u32::MAX);
    let mut buffer = std::io::Cursor::new(Vec::new());
    thumb
        .write_to(&mut buffer, image::ImageFormat::Jpeg)
        .map_err(|error| error.to_string())?;
    Ok(format!(
        "data:image/jpeg;base64,{}",
        BASE64.encode(buffer.into_inner())
    ))
}

#[cfg(test)]
mod content_library_tests {
    use super::*;
    use crate::state::{AppState, Settings};
    use std::sync::Mutex;

    #[test]
    #[ignore]
    fn renders_local_preview() {
        let game = crate::cm::locate::locate(None)
            .ac_root
            .map(PathBuf::from)
            .unwrap_or_default();
        let sample = game
            .join("content")
            .join("cars")
            .join("bmw_m3_e30")
            .join("skins")
            .join("0")
            .join("preview.jpg");
        if !sample.is_file() {
            return;
        }
        let encoded = render_thumbnail(&sample, 480).expect("thumbnail");
        assert!(encoded.starts_with("data:image/jpeg;base64,"));
        assert!(encoded.len() > 2000);
    }

    #[test]
    #[ignore]
    fn scans_project_content() {
        let root = std::path::Path::new("..")
            .join("..")
            .canonicalize()
            .expect("project root");
        let game = crate::cm::locate::locate(None).ac_root;
        let state = AppState {
            exe_path: root.join("ACServerControl.exe"),
            settings: Mutex::new(Settings {
                server_dir: Some(root.to_string_lossy().to_string()),
                game_dir: game,
                cm_dir: None,
                theme: "dark".into(),
                language: "it".into(),
                invite: Default::default(),
                driver_name: String::new(),
                saved_invites: Vec::new(),
            }),
            profiles: Mutex::new(crate::profiles::load(&root)),
            server_dir: Mutex::new(root),
            child: Mutex::new(None),
            logs: Mutex::new(Vec::new()),
            started: Mutex::new(None),
            current_log: Mutex::new(None),
            image_cache: Mutex::new(std::collections::HashMap::new()),
            library_cache: Mutex::new(None),
            join_uri: Mutex::new(None),
            launcher_mode: false,
        };

        let library = content_library_impl(&state);
        println!(
            "cars={} tracks={} con preview={}",
            library.cars.len(),
            library.tracks.len(),
            library.cars.iter().filter(|car| car.preview.is_some()).count()
        );
        assert!(library.cars.len() > 100);
        assert!(library.tracks.len() > 10);
        assert!(library.cars.iter().any(|car| car.preview.is_some()));
    }
}

// ---------------------------------------------------------------- network

#[derive(Serialize)]
pub struct Adapter {
    pub name: String,
    pub ip: String,
    pub kind: String,
}

#[tauri::command]
pub fn list_adapters() -> Vec<Adapter> {
    let mut adapters = Vec::new();
    if let Ok(list) = if_addrs::get_if_addrs() {
        for item in list {
            let if_addrs::IfAddr::V4(v4) = item.addr else {
                continue;
            };
            if v4.ip.is_loopback() {
                continue;
            }
            let octets = v4.ip.octets();
            let kind = if octets[0] == 25 {
                "Hamachi"
            } else if octets[0] == 26 {
                "Radmin VPN"
            } else if octets[0] == 100 && (64..=127).contains(&octets[1]) {
                "Tailscale"
            } else {
                "LAN"
            };
            adapters.push(Adapter {
                name: item.name,
                ip: v4.ip.to_string(),
                kind: kind.to_string(),
            });
        }
    }
    adapters.sort_by_key(|item| match item.kind.as_str() {
        "Hamachi" => 0,
        "Radmin VPN" => 1,
        "Tailscale" => 2,
        _ => 3,
    });
    adapters
}

#[tauri::command]
pub fn write_share_files(
    state: State<AppState>,
    ip: String,
    http_port: u16,
) -> Result<Vec<String>, String> {
    let dir = state.server_dir();
    let sections = config_sections(&state);
    let name = section_value(&sections, "SERVER", "NAME").unwrap_or_else(|| "AC Server".into());
    let password = section_value(&sections, "SERVER", "PASSWORD").unwrap_or_default();
    let track = section_value(&sections, "SERVER", "TRACK").unwrap_or_default();
    let cars = used_cars(&sections).join(", ");
    let link = format!(
        "acmanager://race/online/join?ip={}&httpPort={}",
        ip, http_port
    );

    let url_path = dir.join("entra.url");
    write_text(
        &url_path,
        &format!("[InternetShortcut]\r\nURL={}\r\n", link),
    )?;

    let local_link = format!(
        "acmanager://race/online/join?ip=127.0.0.1&httpPort={}",
        http_port
    );
    let local_path = dir.join("entra-locale.url");
    write_text(
        &local_path,
        &format!("[InternetShortcut]\r\nURL={}\r\n", local_link),
    )?;

    let info = format!(
        "Server      : {}\nIP          : {}\nPista       : {}\nAuto        : {}\nPassword    : {}\nPorta gioco : UDP 9600\nPorta HTTP  : {}\n\nLink per Content Manager:\n{}\n\nLauncher originale: ONLINE > LAN, oppure IP {} porta 9600\n",
        name,
        ip,
        track,
        cars,
        if password.is_empty() { "(nessuna)" } else { "(impostata)" },
        http_port,
        link,
        ip
    );
    let info_path = dir.join("server-info.txt");
    write_text(&info_path, &info)?;

    Ok(vec![
        url_path.to_string_lossy().to_string(),
        local_path.to_string_lossy().to_string(),
        info_path.to_string_lossy().to_string(),
    ])
}

#[derive(Serialize)]
pub struct PortState {
    pub port: u16,
    pub tcp_free: bool,
    pub udp_free: bool,
}

/// Porte locali in ascolto lette da `netstat -ano`, separate fra TCP e UDP.
/// Su Windows il bind di prova riesce anche quando un altro processo ascolta
/// su `0.0.0.0`, quindi non basta provare a legare la porta.
fn listening_ports() -> Option<(HashSet<u16>, HashSet<u16>)> {
    #[cfg(windows)]
    let mut command: Command = {
        let mut command = Command::new("netstat");
        command.args(["-ano"]);
        command.creation_flags(CREATE_NO_WINDOW);
        command
    };
    #[cfg(not(windows))]
    let mut command: Command = {
        let mut command = Command::new("netstat");
        command.args(["-ano"]);
        command
    };
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(parse_netstat(&String::from_utf8_lossy(&output.stdout)))
}

fn parse_netstat(text: &str) -> (HashSet<u16>, HashSet<u16>) {
    let mut tcp = HashSet::new();
    let mut udp = HashSet::new();
    for line in text.lines() {
        let columns: Vec<&str> = line.split_whitespace().collect();
        if columns.len() < 4 {
            continue;
        }
        let Some((_, port)) = columns[1].rsplit_once(':') else {
            continue;
        };
        let Ok(port) = port.parse::<u16>() else {
            continue;
        };
        // Solo il socket in ascolto del server: le connessioni dei client hanno
        // la porta locale effimera e non devono marcare la porta come occupata.
        if columns[0].eq_ignore_ascii_case("TCP") && columns[3].eq_ignore_ascii_case("LISTENING") {
            tcp.insert(port);
        } else if columns[0].eq_ignore_ascii_case("UDP") {
            udp.insert(port);
        }
    }
    (tcp, udp)
}

#[tauri::command]
pub fn check_ports(ports: Vec<u16>) -> Vec<PortState> {
    let table = listening_ports();
    ports
        .into_iter()
        .map(|port| {
            let (tcp_free, udp_free) = match &table {
                Some((tcp, udp)) => (!tcp.contains(&port), !udp.contains(&port)),
                None => (
                    std::net::TcpListener::bind(("0.0.0.0", port)).is_ok(),
                    std::net::UdpSocket::bind(("0.0.0.0", port)).is_ok(),
                ),
            };
            PortState {
                port,
                tcp_free,
                udp_free,
            }
        })
        .collect()
}

#[cfg(test)]
mod port_tests {
    use super::parse_netstat;

    const SAMPLE: &str = "\r\nAttività connessioni\r\n\r\n  Proto  Indirizzo locale        Indirizzo esterno       Stato           PID\r\n  TCP    0.0.0.0:9600           0.0.0.0:0              LISTENING       29144\r\n  TCP    127.0.0.1:51044          127.0.0.1:8081         TIME_WAIT       0\r\n  TCP    [::]:8081              [::]:0                 LISTENING       29144\r\n  UDP    0.0.0.0:9600           *:*                                    29144\r\n  TCP    127.0.0.1:443           0.0.0.0:0              LISTENING       4\r\n";

    #[test]
    fn marks_listening_ports_only() {
        let (tcp, udp) = parse_netstat(SAMPLE);
        assert!(tcp.contains(&9600));
        assert!(tcp.contains(&8081));
        assert!(udp.contains(&9600));
        assert!(!tcp.contains(&51044));
        assert!(!udp.contains(&8081));
    }
}

#[tauri::command]
pub async fn server_info(port: u16) -> Result<serde_json::Value, String> {
    let url = format!("http://127.0.0.1:{}/INFO", port);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .map_err(|error| error.to_string())?;
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let text = response.text().await.map_err(|error| error.to_string())?;
    serde_json::from_str(&text).map_err(|error| format!("{} (risposta: {})", error, text))
}

#[tauri::command]
pub fn open_firewall(state: State<AppState>) -> Result<(), String> {
    let script = state.server_dir().join("apri-porte-firewall.ps1");
    if !script.is_file() {
        return Err(messages::tr("apri-porte-firewall.ps1 non trovato", "apri-porte-firewall.ps1 not found"));
    }
    let path = script.to_string_lossy().replace('\'', "''");
    let command = format!(
        "Start-Process powershell -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File','{}' -Verb RunAs",
        path
    );
    let mut process = Command::new("powershell");
    process.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &command]);
    #[cfg(windows)]
    process.creation_flags(CREATE_NO_WINDOW);
    process.spawn().map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err(messages::tr("percorso vuoto", "empty path"));
    }
    #[cfg(windows)]
    {
        let mut process = Command::new("cmd");
        process.args(["/C", "start", "", path.as_str()]);
        process.creation_flags(CREATE_NO_WINDOW);
        process.spawn().map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// Apre solo URI http/https/acmanager, sempre tra virgolette: niente injection
/// di metacaratteri della shell.
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    let value = url.trim();
    let allowed = ["acmanager://", "http://", "https://"]
        .iter()
        .any(|prefix| value.to_lowercase().starts_with(prefix));
    if !allowed {
        return Err(messages::tr("URL non consentito", "URL not allowed"));
    }
    if value.chars().any(|c| c.is_control() || c == '"' || c == '<' || c == '>') {
        return Err(messages::tr("URL non valido", "invalid URL"));
    }
    #[cfg(windows)]
    {
        // `raw_arg` tiene le virgolette: senza, `Command` non quota l'URL e cmd
        // spezza il link sul `&` (`httpPort=...` diventa un comando a sé).
        let mut process = Command::new("cmd");
        process.arg("/C");
        process.raw_arg(format!("start \"\" \"{}\"", value));
        process.creation_flags(CREATE_NO_WINDOW);
        process.spawn().map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// `true` se Windows ha un gestore registrato per `acmanager://` (Content Manager).
#[tauri::command]
pub fn cm_protocol_ready() -> bool {
    #[cfg(windows)]
    {
        let mut process = Command::new("reg");
        process.args([
            "query",
            r"HKCU\Software\Classes\acmanager\shell\open\command",
            "/ve",
        ]);
        process.creation_flags(CREATE_NO_WINDOW);
        process
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        false
    }
}

// ---------------------------------------------------------------- content manager

#[derive(Serialize)]
pub struct CmScan {
    pub location: cm::CmLocation,
    pub settings: Vec<cm::CmSetting>,
    pub server_presets: Vec<cm::CmServerPreset>,
    pub user_presets: Vec<cm::CmUserPreset>,
    pub total_keys: usize,
    pub error: Option<String>,
}

#[tauri::command]
pub fn cm_scan(state: State<AppState>) -> CmScan {
    let custom = state.settings.lock().unwrap().cm_dir.clone();
    let location = cm::locate::locate(custom.as_deref());

    let mut error = None;
    let mut entries = Vec::new();
    match &location.values_file {
        Some(values) => match cm::storage::read_file(Path::new(values)) {
            Ok(parsed) => entries = parsed,
            Err(message) => error = Some(message),
        },
        None => error = Some("Values.data di Content Manager non trovato".into()),
    }

    let settings = cm::storage::settings_from(&entries);
    let server_presets = location
        .ac_root
        .as_deref()
        .map(cm::presets::scan_server_presets)
        .unwrap_or_default();
    let user_presets = location
        .data_dir
        .as_deref()
        .map(cm::presets::scan_user_presets)
        .unwrap_or_default();

    CmScan {
        location,
        total_keys: settings.len(),
        settings,
        server_presets,
        user_presets,
        error,
    }
}

#[tauri::command]
pub fn cm_import_server_preset(
    state: State<AppState>,
    source: String,
) -> Result<String, String> {
    let destination_root = state.server_dir().join("presets");
    fs::create_dir_all(&destination_root).map_err(|error| error.to_string())?;
    let destination = cm::presets::import_server_preset(Path::new(&source), &destination_root)?;
    Ok(destination.to_string_lossy().to_string())
}

#[tauri::command]
pub fn cm_save_snapshot(
    state: State<AppState>,
    settings: Vec<cm::CmSetting>,
    source: Option<String>,
) -> Result<String, String> {
    let payload = serde_json::json!({
        "generated_at": chrono::Local::now().to_rfc3339(),
        "source": source,
        "settings": settings,
    });
    let path = state.server_dir().join("cm-import.json");
    let text = serde_json::to_string_pretty(&payload).map_err(|error| error.to_string())?;
    write_text(&path, &text)?;
    Ok(path.to_string_lossy().to_string())
}

// ---------------------------------------------------------------- presets

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
            character.is_alphanumeric() || *character == '_' || *character == '-' || *character == ' '
        })
        .collect();
    let safe = safe.trim().to_string();
    if safe.is_empty() {
        return Err(messages::tr("nome preset non valido", "invalid preset name"));
    }
    let destination = state.server_dir().join("presets").join(&safe);
    fs::create_dir_all(&destination).map_err(|error| error.to_string())?;
    let source = state.cfg_dir();
    for file in ["server_cfg.ini", "entry_list.ini", "welcome.txt"] {
        let from = source.join(file);
        if from.is_file() {
            fs::copy(&from, destination.join(file)).map_err(|error| error.to_string())?;
        }
    }
    Ok(destination.to_string_lossy().to_string())
}

#[tauri::command]
pub fn load_preset(state: State<AppState>, name: String) -> Result<(), String> {
    let directory = state.server_dir().join("presets").join(&name);
    if !directory.is_dir() {
        return Err(messages::tr(&format!("preset {} non trovato", name), &format!("preset {} not found", name)));
    }
    backup_cfg(&state).ok();
    let destination = state.cfg_dir();
    for file in ["server_cfg.ini", "entry_list.ini", "welcome.txt"] {
        let from = directory.join(file);
        if from.is_file() {
            fs::copy(&from, destination.join(file)).map_err(|error| error.to_string())?;
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
    backups.sort_by(|a, b| b.modified.cmp(&a.modified));
    backups
}

// ---------------------------------------------------------------- invito / launcher

use crate::invite::{build_link, parse_join_uri, JoinArgs};
use crate::protocol;
use crate::state::InviteSettings;

#[tauri::command]
pub fn invite_settings(state: State<AppState>) -> InviteSettings {
    state.settings.lock().unwrap().invite.clone()
}

fn validate_invite(invite: &InviteSettings) -> Result<(), String> {
    if invite.api_port < 1024 {
        return Err(messages::tr("la porta API deve essere almeno 1024", "the API port must be at least 1024"));
    }
    let accent = invite.accent.trim();
    let valid = accent.len() == 7
        && accent.starts_with('#')
        && accent[1..].chars().all(|c| c.is_ascii_hexdigit());
    if !valid {
        return Err(messages::tr("colore accento non valido: usa #rrggbb", "invalid accent colour: use #rrggbb"));
    }
    Ok(())
}

#[tauri::command]
pub fn save_invite_settings(
    state: State<AppState>,
    invite: InviteSettings,
) -> Result<InviteSettings, String> {
    validate_invite(&invite)?;
    let mut settings = state.settings.lock().unwrap();
    settings.invite = InviteSettings {
        token: if invite.token.trim().is_empty() {
            crate::invite::new_token()
        } else {
            invite.token.trim().to_string()
        },
        ..invite
    };
    let saved = settings.invite.clone();
    drop(settings);
    state.persist();
    Ok(saved)
}

#[tauri::command]
pub fn regenerate_invite_token(state: State<AppState>) -> InviteSettings {
    let mut settings = state.settings.lock().unwrap();
    settings.invite.token = crate::invite::new_token();
    let saved = settings.invite.clone();
    drop(settings);
    state.persist();
    saved
}

#[tauri::command]
pub fn invite_link(state: State<AppState>, ip: String, http_port: u16) -> Result<String, String> {
    if ip.trim().is_empty() {
        return Err(messages::tr("IP non disponibile", "no IP available"));
    }
    let invite = state.settings.lock().unwrap().invite.clone();
    Ok(build_link(&ip, http_port, &invite))
}

#[tauri::command]
pub fn register_invite_protocol(state: State<AppState>) -> Result<bool, String> {
    protocol::register_protocol(&state.exe_path)?;
    Ok(protocol::protocol_registered(&state.exe_path))
}

#[tauri::command]
pub fn invite_protocol_status(state: State<AppState>) -> bool {
    protocol::protocol_registered(&state.exe_path)
}

#[tauri::command]
pub fn join_context(state: State<AppState>) -> Option<JoinArgs> {
    state
        .join_uri
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|uri| parse_join_uri(uri).ok())
}

#[derive(Serialize)]
pub struct LauncherState {
    pub launcher_mode: bool,
    pub driver_name: String,
    pub invites: Vec<crate::state::SavedInvite>,
}

#[tauri::command]
pub fn launcher_state(state: State<AppState>) -> LauncherState {
    let settings = state.settings.lock().unwrap();
    LauncherState {
        launcher_mode: state.launcher_mode,
        driver_name: settings.driver_name.clone(),
        invites: settings.saved_invites.clone(),
    }
}

#[tauri::command]
pub fn add_invite(state: State<AppState>, link: String) -> Result<LauncherState, String> {
    parse_join_uri(&link)?;
    let mut settings = state.settings.lock().unwrap();
    if !settings
        .saved_invites
        .iter()
        .any(|invite| invite.link == link)
    {
        settings.saved_invites.insert(
            0,
            crate::state::SavedInvite {
                link: link.clone(),
            },
        );
        settings.saved_invites.truncate(20);
    }
    let snapshot = LauncherState {
        launcher_mode: state.launcher_mode,
        driver_name: settings.driver_name.clone(),
        invites: settings.saved_invites.clone(),
    };
    drop(settings);
    state.persist();
    Ok(snapshot)
}

#[tauri::command]
pub fn remove_invite(state: State<AppState>, link: String) -> LauncherState {
    let mut settings = state.settings.lock().unwrap();
    settings.saved_invites.retain(|invite| invite.link != link);
    let snapshot = LauncherState {
        launcher_mode: state.launcher_mode,
        driver_name: settings.driver_name.clone(),
        invites: settings.saved_invites.clone(),
    };
    drop(settings);
    state.persist();
    snapshot
}

#[tauri::command]
pub fn set_driver_name(state: State<AppState>, name: String) -> String {
    let trimmed: String = name.trim().chars().take(40).collect();
    let cleaned: String = trimmed
        .chars()
        .filter(|c| !c.is_control() && !"[];=\r\n".contains(*c))
        .collect();
    {
        let mut settings = state.settings.lock().unwrap();
        settings.driver_name = cleaned.clone();
    }
    state.persist();
    cleaned
}

#[tauri::command]
pub fn open_join_link(state: State<AppState>, link: String) -> Result<(), String> {
    parse_join_uri(&link)?;
    *state.join_uri.lock().unwrap() = Some(link);
    Ok(())
}

#[tauri::command]
pub fn launcher_library(state: State<AppState>) -> ContentLibrary {
    content_library_impl(&state)
}

#[tauri::command]
pub async fn remote_server_info(
    host: String,
    api_port: u16,
    token: String,
) -> Result<serde_json::Value, String> {
    if !crate::invite::valid_host(&host) {
        return Err(messages::tr("indirizzo del server non valido", "invalid server address"));
    }
    let url = format!("http://{}:{}/api/server?token={}", host, api_port, token);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .build()
        .map_err(|error| error.to_string())?;
    let response = client.get(url).send().await.map_err(|error| {
        format!("server non raggiungibile: {}", error)
    })?;
    if response.status().as_u16() == 403 {
        return Err(messages::tr("token non valido: chiedi un link aggiornato", "invalid token: ask for a fresh link"));
    }
    let text = response.text().await.map_err(|error| error.to_string())?;
    serde_json::from_str(&text).map_err(|error| format!("risposta non valida: {}", error))
}

#[tauri::command]
pub async fn remote_reserve(
    host: String,
    api_port: u16,
    token: String,
    driver: String,
    car: String,
    skin: String,
    guid: String,
) -> Result<serde_json::Value, String> {
    if !crate::invite::valid_host(&host) {
        return Err(messages::tr("indirizzo del server non valido", "invalid server address"));
    }
    let url = format!("http://{}:{}/api/reserve", host, api_port);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .build()
        .map_err(|error| error.to_string())?;
    let response = client
        .post(url)
        .json(&serde_json::json!({
            "token": token,
            "driver": driver,
            "car": car,
            "skin": skin,
            "guid": guid,
        }))
        .send()
        .await
        .map_err(|error| format!("prenotazione non riuscita: {}", error))?;
    let status = response.status().as_u16();
    let text = response.text().await.map_err(|error| error.to_string())?;
    if status == 403 {
        return Err(messages::tr("token non valido: chiedi un link aggiornato", "invalid token: ask for a fresh link"));
    }
    serde_json::from_str(&text).map_err(|error| format!("risposta non valida: {}", error))
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
        return Err(messages::tr("indirizzo del server non valido", "invalid server address"));
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

    let cfg_dir = crate::assists::ac_cfg_dir().ok_or_else(|| {
        "cartella Documenti\\Assetto Corsa non trovata".to_string()
    })?;
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
        &[
            ("MODEL", car),
            ("SKIN", skin),
            ("DRIVER_NAME", name),
        ],
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

#[derive(Serialize)]
pub struct AssistsState {
    pub path: Option<String>,
    pub exists: bool,
    pub values: crate::assists::Assists,
}

fn assists_snapshot() -> AssistsState {
    let path = crate::assists::config_path();
    let exists = path.as_deref().map(Path::is_file).unwrap_or(false);
    let values = path
        .as_deref()
        .map(crate::assists::load)
        .unwrap_or_default();
    AssistsState {
        path: path.map(|value| value.display().to_string()),
        exists,
        values,
    }
}

#[tauri::command]
pub fn assists_state() -> AssistsState {
    assists_snapshot()
}

#[tauri::command]
pub fn save_assists(values: crate::assists::Assists) -> Result<AssistsState, String> {
    let path = crate::assists::config_path()
        .ok_or_else(|| "cartella Documenti di Windows non trovata".to_string())?;
    crate::assists::save(&path, &values)?;
    Ok(assists_snapshot())
}