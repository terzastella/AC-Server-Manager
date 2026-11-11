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

#[derive(Serialize)]
pub struct ExternalServer {
    pub pid: u32,
    pub path: String,
}

/// Percorso dell'eseguibile per PID, quando Windows lo rende leggibile.
fn server_paths() -> std::collections::HashMap<u32, String> {
    let mut paths: std::collections::HashMap<u32, String> = std::collections::HashMap::new();
    #[cfg(windows)]
    {
        let mut command = Command::new("powershell");
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-Process acServer -ErrorAction SilentlyContinue | ForEach-Object { \"$($_.Id)|$($_.Path)\" }",
        ]);
        command.creation_flags(CREATE_NO_WINDOW);
        if let Ok(output) = command.output() {
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                if let Some((pid, path)) = line.split_once('|') {
                    if let Ok(pid) = pid.trim().parse::<u32>() {
                        paths.insert(pid, path.trim().to_string());
                    }
                }
            }
        }
    }
    paths
}

/// Processi `acServer.exe` avviati fuori da questa app, con il percorso
/// dell'eseguibile: l'interfaccia li mostra prima di terminare qualcosa.
#[tauri::command]
pub fn external_servers() -> Vec<ExternalServer> {
    let mut paths = server_paths();
    external_server_pids()
        .into_iter()
        .map(|pid| ExternalServer {
            pid,
            path: paths.remove(&pid).unwrap_or_default(),
        })
        .collect()
}

/// Un processo è davvero acServer.exe solo se il percorso è assoluto e il nome
/// del file coincide esattamente (case-insensitive). Un semplice suffisso
/// accetterebbe anche `C:\x\notacServer.exe`.
fn is_acserver_exe(path: &str) -> bool {
    let path = std::path::Path::new(path);
    path.is_absolute()
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.eq_ignore_ascii_case("acServer.exe"))
            .unwrap_or(false)
}

/// Verifica e termina **lo stesso handle** del processo scelto: sparisce la
/// finestra in cui il PID poteva essere riusato tra controllo e `taskkill`.
#[cfg(windows)]
fn terminate_verified(pid: u32) -> Result<(), String> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, TerminateProcess,
        PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
    };

    let handle = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE,
            0,
            pid,
        )
    };
    if handle.is_null() {
        return Err(messages::tr(
            &format!("impossibile aprire il processo {}", pid),
            &format!("cannot open process {}", pid),
        ));
    }

    let mut buffer = [0u16; 1024];
    let mut size = buffer.len() as u32;
    let read = unsafe { QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut size) };
    if read == 0 {
        unsafe { CloseHandle(handle) };
        return Err(messages::tr(
            &format!("impossibile leggere l'eseguibile del PID {}", pid),
            &format!("cannot read the executable of PID {}", pid),
        ));
    }

    let path = String::from_utf16_lossy(&buffer[..size as usize]);
    if !is_acserver_exe(&path) {
        unsafe { CloseHandle(handle) };
        return Err(messages::tr(
            &format!("rifiutato: il PID {} non è acServer.exe", pid),
            &format!("refused: PID {} is not acServer.exe", pid),
        ));
    }

    let terminated = unsafe { TerminateProcess(handle, 1) };
    unsafe { CloseHandle(handle) };
    if terminated == 0 {
        return Err(messages::tr(
            &format!("impossibile terminare il PID {}", pid),
            &format!("cannot terminate PID {}", pid),
        ));
    }
    Ok(())
}

/// Termina un solo processo, quello scelto dall'utente, dopo aver verificato
/// che sia ancora `acServer.exe`: il PID da solo può essere stato riusato.
#[tauri::command]
pub fn stop_external_server(pid: u32) -> Result<u32, String> {
    if !external_server_pids().contains(&pid) {
        return Err(messages::tr(
            "il processo non è più attivo",
            "the process is no longer running",
        ));
    }

    #[cfg(windows)]
    {
        terminate_verified(pid)?;
        Ok(1)
    }

    #[cfg(not(windows))]
    {
        // fuori da Windows non c'è l'handle: si verifica il percorso, e se non
        // è leggibile si rifiuta invece di uccidere un PID riusato
        match server_paths().remove(&pid) {
            Some(path) if is_acserver_exe(&path) => {}
            Some(_) => {
                return Err(messages::tr(
                    &format!("rifiutato: il PID {} non è acServer.exe", pid),
                    &format!("refused: PID {} is not acServer.exe", pid),
                ))
            }
            None => {
                return Err(messages::tr(
                    &format!(
                        "rifiutato: impossibile verificare l'eseguibile del PID {}",
                        pid
                    ),
                    &format!("refused: cannot verify the executable of PID {}", pid),
                ))
            }
        }
        let mut command = Command::new("taskkill");
        command.args(["/PID", &pid.to_string(), "/F"]);
        match command.output() {
            Ok(output) if output.status.success() => Ok(1),
            Ok(output) => {
                let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
                Err(messages::tr(
                    &format!("impossibile terminare il PID {}: {}", pid, detail),
                    &format!("cannot terminate PID {}: {}", pid, detail),
                ))
            }
            Err(error) => Err(messages::tr(
                &format!("taskkill non avviato: {}", error),
                &format!("taskkill failed to start: {}", error),
            )),
        }
    }
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
    files.sort_by_key(|item| std::cmp::Reverse(item.modified));
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

/// File di configurazione che finiscono nel backup del profilo attivo.
const CFG_FILES: [&str; 3] = ["server_cfg.ini", "entry_list.ini", "welcome.txt"];

/// Copia i file di configurazione in una cartella di backup nuova. Rifiuta le
/// sorgenti che non sono file regolari (link o reparse point) e, se una copia
/// fallisce, elimina la cartella: mai un backup a metà.
fn copy_cfg_files(source: &Path, destination: &Path) -> Result<(), String> {
    // prima si controlla tutto, poi si scrive
    let mut present = Vec::new();
    for name in CFG_FILES {
        let from = source.join(name);
        match fs::symlink_metadata(&from) {
            Ok(metadata) => {
                let file_type = metadata.file_type();
                if crate::fsx::is_link(&file_type) || !file_type.is_file() {
                    return Err(messages::tr(
                        &format!("{} non è un file regolare: backup annullato", name),
                        &format!("{} is not a regular file: backup cancelled", name),
                    ));
                }
                present.push((name, from));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("{}: {}", from.display(), error)),
        }
    }
    for (name, from) in present {
        let to = destination.join(name);
        let copied = fs::copy(&from, &to).and_then(|written| {
            let expected = fs::metadata(&from)?.len();
            if written != expected {
                return Err(std::io::Error::other("copia incompleta"));
            }
            Ok(())
        });
        if let Err(error) = copied {
            let _ = fs::remove_dir_all(destination);
            return Err(format!("{}: {}", to.display(), error));
        }
    }
    Ok(())
}

pub(crate) fn backup_cfg(state: &AppState) -> Result<String, String> {
    let dir = state.server_dir();
    let profile = state.active_profile();
    // millisecondi + eventuale suffisso: due backup ravvicinati non si fondono
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S-%3f").to_string();
    let tag = if profile.slug.is_empty() {
        "default".to_string()
    } else {
        profile.slug.clone()
    };
    let root = dir.join("backups");
    fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    let mut destination = root.join(format!("cfg-{}-{}", tag, stamp));
    let mut attempt = 1;
    loop {
        match fs::create_dir(&destination) {
            Ok(()) => break,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists && attempt < 100 => {
                destination = root.join(format!("cfg-{}-{}-{}", tag, stamp, attempt));
                attempt += 1;
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    let source = state.cfg_dir();
    // i file vengono controllati e copiati insieme: se uno non è regolare o una
    // copia non riesce la cartella di backup viene rimossa
    copy_cfg_files(&source, &destination)?;
    Ok(destination.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_refuses_unknown_pid() {
        let error = stop_external_server(999_999).expect_err("PID inesistente rifiutato");
        assert!(!error.is_empty());
    }

    #[test]
    fn only_real_acserver_exe_is_accepted() {
        assert!(is_acserver_exe(r"C:\AC\server\acServer.exe"));
        assert!(is_acserver_exe(r"C:\AC\server\ACSERVER.EXE"));
        // suffisso simile ma nome diverso, oppure percorso relativo
        assert!(!is_acserver_exe(r"C:\AC\server\notacServer.exe"));
        assert!(!is_acserver_exe(r"C:\AC\server\acServer.exe.bak"));
        assert!(!is_acserver_exe("acServer.exe"));
        assert!(!is_acserver_exe(""));
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("acsc-backup-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("cartella");
        dir
    }

    #[test]
    fn backup_copies_the_regular_files_only() {
        let base = temp_dir("regular");
        let source = base.join("cfg");
        fs::create_dir_all(&source).expect("cfg");
        fs::write(source.join("server_cfg.ini"), "[SERVER]\nNAME=x\n").expect("cfg");
        fs::write(source.join("entry_list.ini"), "[CAR_0]\n").expect("entry");
        let destination = base.join("backup");
        fs::create_dir_all(&destination).expect("backup");

        copy_cfg_files(&source, &destination).expect("copia");

        assert_eq!(
            fs::read_to_string(destination.join("server_cfg.ini")).expect("letto"),
            "[SERVER]\nNAME=x\n"
        );
        assert!(destination.join("entry_list.ini").is_file());
        // welcome.txt non esiste nella sorgente: non deve comparire nel backup
        assert!(!destination.join("welcome.txt").exists());
        let _ = fs::remove_dir_all(&base);
    }

    #[cfg(windows)]
    #[test]
    fn a_link_source_stops_the_backup_before_anything_is_copied() {
        let base = temp_dir("link");
        let source = base.join("cfg");
        fs::create_dir_all(&source).expect("cfg");
        fs::write(source.join("server_cfg.ini"), "[SERVER]\n").expect("cfg");
        let outside = base.join("altrove");
        fs::create_dir_all(&outside).expect("fuori");
        // welcome.txt è una junction: il controllo è fail-closed e precede ogni copia
        junction::create(&outside, source.join("welcome.txt")).expect("junction");
        let destination = base.join("backup");
        fs::create_dir_all(&destination).expect("backup");

        assert!(copy_cfg_files(&source, &destination).is_err());
        assert_eq!(
            fs::read_dir(&destination).expect("dir").count(),
            0,
            "nessun file deve essere copiato prima del controllo"
        );
        let _ = fs::remove_dir(source.join("welcome.txt"));
        let _ = fs::remove_dir_all(&base);
    }
}
