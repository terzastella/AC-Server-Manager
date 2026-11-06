use super::*;

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
            used: used_weather
                .iter()
                .any(|item| item.eq_ignore_ascii_case(&id)),
            id,
        });
    }

    let missing_cars = used_cars
        .iter()
        .filter(|id| safe_id(id))
        .filter(|id| !dir.join("content").join("cars").join(id.as_str()).is_dir())
        .cloned()
        .collect();
    let missing_tracks = if !used_track.is_empty()
        && safe_id(&used_track)
        && !dir
            .join("content")
            .join("tracks")
            .join(&used_track)
            .is_dir()
    {
        vec![used_track.clone()]
    } else {
        Vec::new()
    };
    let missing_weather = used_weather
        .iter()
        .filter(|id| {
            !dir.join("content")
                .join("weather")
                .join(id.as_str())
                .is_dir()
        })
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

/// Identificativo di contenuto usato nei percorsi: niente separatori, unità,
/// `..` o caratteri di controllo. Un `CARS=` scritto a mano non deve poter
/// far leggere o copiare fuori da `content/`.
fn safe_id(id: &str) -> bool {
    let id = id.trim();
    !id.is_empty()
        && id.len() <= 128
        && id != "."
        && id != ".."
        && !id.contains('/')
        && !id.contains('\\')
        && !id.contains(':')
        && !id.chars().any(char::is_control)
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
        return Err(messages::tr(
            &format!("cartella content non valida: {}", content.display()),
            &format!("invalid content folder: {}", content.display()),
        ));
    }

    {
        state.settings.lock().unwrap().game_dir = Some(game_dir.clone());
        state.persist()?;
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
            Err(error) => report
                .errors
                .push(format!("{}: {}", dir_name(&destination), error)),
        }
    };

    for id in used_cars(&sections) {
        if !safe_id(&id) {
            report.errors.push(messages::tr(
                &format!("identificativo auto non valido: {}", id),
                &format!("invalid car identifier: {}", id),
            ));
            continue;
        }
        copy_one(
            cars_root.join(&id),
            dir.join("content").join("cars").join(&id),
            &mut report,
        );
    }
    if let Some(track) = section_value(&sections, "SERVER", "TRACK") {
        if !track.is_empty() {
            if safe_id(&track) {
                copy_one(
                    tracks_root.join(&track),
                    dir.join("content").join("tracks").join(&track),
                    &mut report,
                );
            } else {
                report.errors.push(messages::tr(
                    &format!("identificativo pista non valido: {}", track),
                    &format!("invalid track identifier: {}", track),
                ));
            }
        }
    }
    for id in used_weather(&sections) {
        if !safe_id(&id) {
            report.errors.push(messages::tr(
                &format!("identificativo meteo non valido: {}", id),
                &format!("invalid weather identifier: {}", id),
            ));
            continue;
        }
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
                if let Err(error) = state.persist() {
                    eprintln!("impostazioni non salvate: {}", error);
                }
            }
        }
        detected.map(PathBuf::from)
    };

    let sections = config_sections(state);
    let used = used_cars(&sections);
    let used_track = section_value(&sections, "SERVER", "TRACK").unwrap_or_default();

    let server_cars = collect_dirs(&server_root.join("cars"));
    let game_cars = game_root
        .as_ref()
        .map(|root| collect_dirs(&root.join("content").join("cars")))
        .unwrap_or_default();

    let mut car_ids: Vec<String> = server_cars
        .keys()
        .chain(game_cars.keys())
        .cloned()
        .collect();
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
                    if value.is_empty() {
                        name.clone()
                    } else {
                        value
                    }
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
        skins.sort_by_key(|item| item.display.to_lowercase());

        let badge = first_existing(&[
            game_path
                .as_ref()
                .map(|p| p.join("ui").join("badge.png"))
                .unwrap_or_default(),
            server_path
                .as_ref()
                .map(|p| p.join("ui").join("badge.png"))
                .unwrap_or_default(),
        ]);
        let preview = skins
            .iter()
            .find_map(|skin| skin.preview.clone())
            .or_else(|| badge.clone())
            .or_else(|| {
                first_existing(&[
                    game_path
                        .as_ref()
                        .map(|p| p.join("ui").join("preview.jpg"))
                        .unwrap_or_default(),
                    server_path
                        .as_ref()
                        .map(|p| p.join("ui").join("preview.jpg"))
                        .unwrap_or_default(),
                ])
            });

        cars.push(CarEntry {
            used: used.iter().any(|item| item.eq_ignore_ascii_case(&id)),
            name: if json_text(&ui, "name").is_empty() {
                id.clone()
            } else {
                json_text(&ui, "name")
            },
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
    let mut track_ids: Vec<String> = server_tracks
        .keys()
        .chain(game_tracks.keys())
        .cloned()
        .collect();
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
            game_path
                .as_ref()
                .map(|p| p.join("ui").join("preview.png"))
                .unwrap_or_default(),
            game_path
                .as_ref()
                .map(|p| p.join("ui").join("preview.jpg"))
                .unwrap_or_default(),
            server_path
                .as_ref()
                .map(|p| p.join("ui").join("preview.png"))
                .unwrap_or_default(),
        ]);
        let outline = first_existing(&[
            game_path
                .as_ref()
                .map(|p| p.join("ui").join("outline.png"))
                .unwrap_or_default(),
            server_path
                .as_ref()
                .map(|p| p.join("ui").join("outline.png"))
                .unwrap_or_default(),
        ]);

        tracks.push(TrackEntry {
            used: id.eq_ignore_ascii_case(&used_track),
            name: if json_text(&ui, "name").is_empty() {
                id.clone()
            } else {
                json_text(&ui, "name")
            },
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
        return Err(messages::tr(
            "immagine fuori dalle cartelle content",
            "image outside the content folders",
        ));
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
            entry_stamp: Mutex::new(None),
        };

        let library = content_library_impl(&state);
        println!(
            "cars={} tracks={} con preview={}",
            library.cars.len(),
            library.tracks.len(),
            library
                .cars
                .iter()
                .filter(|car| car.preview.is_some())
                .count()
        );
        assert!(library.cars.len() > 100);
        assert!(library.tracks.len() > 10);
        assert!(library.cars.iter().any(|car| car.preview.is_some()));
    }
}

// ---------------------------------------------------------------- network

#[cfg(test)]
mod safe_id_tests {
    use super::safe_id;

    #[test]
    fn rejects_path_escape() {
        assert!(safe_id("bmw_z4_gt3"));
        assert!(safe_id("ks_barcelona"));
        assert!(safe_id("v1.2"));
        assert!(!safe_id(".."));
        assert!(!safe_id("../cfg"));
        assert!(!safe_id(r"..\cfg"));
        assert!(!safe_id(r"C:\Windows"));
        assert!(!safe_id("cars/../cfg"));
        assert!(!safe_id(""));
    }
}
