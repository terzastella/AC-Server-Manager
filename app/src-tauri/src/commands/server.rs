use super::*;

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
        return Err(messages::tr(
            "acServer.exe non trovato nella cartella server",
            "acServer.exe not found in the server folder",
        ));
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
        return Err(messages::tr(
            &format!(
                "c'è già un server AC in esecuzione (PID {}), avviato fuori da questa app: fermalo prima di avviarne un altro.",
                list
            ),
            &format!(
                "an AC server is already running (PID {}), started outside this app: stop it before starting another one.",
                list
            ),
        ));
    }
    let logs = state.server_dir().join("logs");
    fs::create_dir_all(&logs).map_err(|error| error.to_string())?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let tag = if profile.slug.is_empty() {
        "default".to_string()
    } else {
        profile.slug.clone()
    };
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
        return Err(messages::tr(
            "file fuori dalla cartella logs",
            "file outside the logs folder",
        ));
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
    let tag = if profile.slug.is_empty() {
        "default".to_string()
    } else {
        profile.slug.clone()
    };
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
