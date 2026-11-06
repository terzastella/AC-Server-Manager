mod api;
mod assists;
mod cm;
mod commands;
mod ini;
mod invite;
mod messages;
mod profiles;
mod protocol;
mod slots;
mod state;

use state::{AppState, LogEntry};
use std::sync::Mutex;
use std::time::Instant;
use tauri::Manager;

fn join_argument() -> Option<String> {
    std::env::args()
        .skip(1)
        .find(|argument| argument.to_lowercase().starts_with("acservercontrol://"))
}

fn launcher_build(exe: &std::path::Path) -> bool {
    let name = exe
        .file_stem()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    name.contains("launcher")
        || exe
            .parent()
            .map(|dir| dir.join("launcher.flag").is_file())
            .unwrap_or(false)
}

/// Una sola istanza per modalità: due manager aperti significano due server di
/// API e due copie di `settings.json` che si sovrascrivono. Il link di invito
/// resta escluso, così può sempre aprire la sua finestra di iscrizione.
#[cfg(windows)]
mod single_instance {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::sync::OnceLock;

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateMutexW(
            attributes: *const core::ffi::c_void,
            owner: i32,
            name: *const u16,
        ) -> *mut core::ffi::c_void;
        fn GetLastError() -> u32;
    }

    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(class: *const u16, title: *const u16) -> *mut core::ffi::c_void;
        fn ShowWindow(window: *mut core::ffi::c_void, command: i32) -> i32;
        fn SetForegroundWindow(window: *mut core::ffi::c_void) -> i32;
    }

    const ERROR_ALREADY_EXISTS: u32 = 183;
    const SW_RESTORE: i32 = 9;

    static MUTEX: OnceLock<usize> = OnceLock::new();

    fn wide(value: &str) -> Vec<u16> {
        OsStr::new(value).encode_wide().chain(Some(0)).collect()
    }

    pub fn already_running(mode: &str) -> bool {
        let name = wide(&format!("Local\\acservercontrol-{}", mode));
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return false;
        }
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            return true;
        }
        // Il mutex deve restare aperto per tutta la vita del processo.
        let _ = MUTEX.set(handle as usize);
        false
    }

    pub fn focus(title: &str) {
        let title = wide(title);
        let window = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
        if window.is_null() {
            return;
        }
        unsafe {
            ShowWindow(window, SW_RESTORE);
            SetForegroundWindow(window);
        }
    }
}

pub fn run() {
    let exe = std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let launcher_mode = launcher_build(&exe);

    #[cfg(windows)]
    if join_argument().is_none()
        && single_instance::already_running(if launcher_mode { "launcher" } else { "manager" })
    {
        single_instance::focus(if launcher_mode {
            "AC Launcher"
        } else {
            "AC Server Control"
        });
        return;
    }

    let panic_path = exe
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("panic.log");
    std::panic::set_hook(Box::new(move |info| {
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&panic_path)
        {
            let _ = writeln!(file, "{}", info);
        }
    }));

    let detected = state::detect_server_dir(&exe);
    let mut settings = state::load_settings(&exe);
    messages::set(&settings.language);
    if settings.server_dir.is_none() {
        if let Some(dir) = &detected {
            settings.server_dir = Some(dir.to_string_lossy().to_string());
        }
    }
    let server_dir = settings
        .server_dir
        .as_ref()
        .map(std::path::PathBuf::from)
        .filter(|path| path.join("acServer.exe").is_file())
        .or(detected)
        .unwrap_or_default();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            exe_path: exe.clone(),
            settings: Mutex::new(settings),
            server_dir: Mutex::new(server_dir.clone()),
            child: Mutex::new(None),
            logs: Mutex::new(Vec::<LogEntry>::new()),
            started: Mutex::new(None::<Instant>),
            current_log: Mutex::new(None::<String>),
            image_cache: Mutex::new(std::collections::HashMap::new()),
            library_cache: Mutex::new(None),
            join_uri: Mutex::new(join_argument()),
            launcher_mode,
            profiles: Mutex::new(state::load_profiles(&server_dir)),
            entry_stamp: Mutex::new(None),
        })
        .setup(|app| {
            if let Ok(exe) = std::env::current_exe() {
                let _ = protocol::register_protocol(&exe);
            }
            let state = app.state::<AppState>();
            if !state.launcher_mode {
                let (port, bind) = {
                    let settings = state.settings.lock().unwrap();
                    (settings.invite.api_port, settings.invite.api_bind.clone())
                };
                match api::start(app.handle().clone(), port, &bind) {
                    Ok(bound) => {
                        if bound != port {
                            // il link d'invito deve puntare alla porta davvero aperta
                            state.settings.lock().unwrap().invite.api_port = bound;
                            if let Err(error) = state.persist() {
                                eprintln!("porta API non salvata: {}", error);
                            }
                            eprintln!("porta API {} occupata: uso {}", port, bound);
                        }
                    }
                    Err(error) => eprintln!("API invito non avviata: {}", error),
                }
            }
            if join_argument().is_some() {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_title("AC Server Control — Entra");
                    let _ = window.set_size(tauri::LogicalSize::new(1180.0, 820.0));
                    let _ = window.center();
                }
            } else if state.launcher_mode {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_title("AC Launcher");
                    if let Ok(icon) =
                        tauri::image::Image::from_bytes(include_bytes!("../icons/launcher.png"))
                    {
                        let _ = window.set_icon(icon);
                    }
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::get_settings,
            commands::save_settings,
            commands::set_language,
            commands::set_server_dir,
            commands::list_profiles,
            commands::create_profile,
            commands::import_profile,
            commands::duplicate_profile,
            commands::rename_profile,
            commands::delete_profile,
            commands::set_active_profile,
            commands::server_status,
            commands::start_server,
            commands::stop_server,
            commands::restart_server,
            commands::stop_external_server,
            commands::external_servers,
            commands::get_logs,
            commands::clear_logs,
            commands::list_logs,
            commands::tail_log,
            commands::read_config,
            commands::save_config,
            commands::read_entry_list,
            commands::save_entry_list,
            commands::read_welcome,
            commands::save_welcome,
            commands::scan_content,
            commands::content_library,
            commands::content_image,
            commands::copy_content,
            commands::list_adapters,
            commands::write_share_files,
            commands::check_ports,
            commands::server_info,
            commands::open_firewall,
            commands::list_presets,
            commands::save_preset,
            commands::load_preset,
            commands::list_backups,
            commands::backup_now,
            commands::open_path,
            commands::open_url,
            commands::launch_game,
            commands::steam_guid,
            commands::cm_protocol_ready,
            commands::cm_scan,
            commands::cm_import_server_preset,
            commands::cm_save_snapshot,
            commands::invite_settings,
            commands::save_invite_settings,
            commands::regenerate_invite_token,
            commands::invite_link,
            commands::register_invite_protocol,
            commands::invite_protocol_status,
            commands::join_context,
            commands::launcher_state,
            commands::add_invite,
            commands::remove_invite,
            commands::set_driver_name,
            commands::open_join_link,
            commands::launcher_library,
            commands::remote_server_info,
            commands::remote_reserve,
            commands::assists_state,
            commands::save_assists,
        ])
        .run(tauri::generate_context!())
        .expect("error while running AC Server Control");
}
