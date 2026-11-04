use serde_json::json;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

use crate::commands;
use crate::messages;
use crate::slots::{apply_reservation, free_slot_count, Reservation};
use crate::state::AppState;

static WRITE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static RATE: OnceLock<Mutex<HashMap<String, Vec<Instant>>>> = OnceLock::new();

fn write_lock() -> &'static Mutex<()> {
    WRITE_LOCK.get_or_init(|| Mutex::new(()))
}

/// Finestra del limite per IP.
const RATE_WINDOW: Duration = Duration::from_secs(60);

/// Logica pura del limite (testabile con un istante esplicito).
fn rate_allowed(
    store: &Mutex<HashMap<String, Vec<Instant>>>,
    peer: &str,
    max: usize,
    now: Instant,
) -> bool {
    let mut guard = store.lock().unwrap();
    let hits = guard.entry(peer.to_string()).or_default();
    hits.retain(|at| now.duration_since(*at) < RATE_WINDOW);
    if hits.len() >= max {
        return false;
    }
    hits.push(now);
    true
}

/// Limite per IP. Il launcher chiede una volta per invito salvato (fino a 8)
/// a ogni aggiornamento: con 10/minuto bastava un refresh per ricevere 429.
fn rate_ok(peer: &str, max: usize) -> bool {
    let store = RATE.get_or_init(|| Mutex::new(HashMap::new()));
    rate_allowed(store, peer, max, Instant::now())
}

/// Confronto del token: un token vuoto non deve mai combaciare.
fn token_matches(expected: &str, provided: &str) -> bool {
    !expected.is_empty() && expected == provided
}

fn token_ok(state: &AppState, provided: &str) -> bool {
    let expected = state.settings.lock().unwrap().invite.token.clone();
    token_matches(&expected, provided)
}

fn server_payload(state: &AppState) -> serde_json::Value {
    let sections = commands::config_sections(state);
    let name = commands::section_value(&sections, "SERVER", "NAME").unwrap_or_default();
    let track = commands::section_value(&sections, "SERVER", "TRACK").unwrap_or_default();
    let max_clients = commands::section_value(&sections, "SERVER", "MAX_CLIENTS")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let port = commands::section_value(&sections, "SERVER", "UDP_PORT")
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(9600);
    let entry_text =
        std::fs::read_to_string(state.cfg_dir().join("entry_list.ini")).unwrap_or_default();
    let library = commands::content_library_impl(state);

    let cars: Vec<serde_json::Value> = commands::used_cars(&sections)
        .into_iter()
        .map(|id| {
            let car = library
                .cars
                .iter()
                .find(|car| car.id.eq_ignore_ascii_case(&id));
            let skins: Vec<String> = car
                .map(|car| car.skins.iter().map(|skin| skin.name.clone()).collect())
                .unwrap_or_default();
            json!({
                "id": id,
                "name": car.map(|car| car.name.clone()).unwrap_or_else(|| id.clone()),
                "skins": skins,
            })
        })
        .collect();

    let running = {
        let mut guard = state.child.lock().unwrap();
        match guard.as_mut() {
            Some(child) => matches!(child.try_wait(), Ok(None)),
            None => false,
        }
    };

    json!({
        "name": name,
        "track": track,
        "max_clients": max_clients,
        "port": port,
        "free_slots": free_slot_count(&entry_text),
        "running": running,
        "cars": cars,
    })
}

/// Legge e valida il corpo di `POST /api/reserve` (senza toccare i file).
fn parse_reserve(body: &serde_json::Value) -> Result<Reservation, String> {
    let field = |name: &str| {
        body.get(name)
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim()
            .to_string()
    };
    let driver = field("driver");
    let car = field("car");
    let skin = field("skin");
    let guid = field("guid");

    if driver.is_empty() || car.is_empty() {
        return Err(messages::tr(
            "nome pilota e auto sono obbligatori",
            "driver name and car are required",
        ));
    }
    if guid.is_empty() || !guid.chars().all(|c| c.is_ascii_digit()) {
        return Err(messages::tr(
            "Steam non rilevato: apri Steam e riprova, senza GUID lo slot non è utilizzabile",
            "Steam not detected: open Steam and retry, the slot is unusable without a GUID",
        ));
    }
    Ok(Reservation {
        driver,
        car,
        skin,
        guid,
    })
}

fn reserve(state: &AppState, body: &serde_json::Value) -> Result<usize, String> {
    let reservation = parse_reserve(body)?;
    let car = reservation.car.clone();
    let sections = commands::config_sections(state);
    let available = commands::used_cars(&sections);
    if !available.iter().any(|id| id.eq_ignore_ascii_case(&car)) {
        return Err(messages::tr(
            &format!("auto non presente in CARS=: {}", car),
            &format!("car not in CARS=: {}", car),
        ));
    }
    let cap = commands::section_value(&sections, "SERVER", "MAX_CLIENTS")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);

    let _guard = write_lock().lock().unwrap();
    let path = state.cfg_dir().join("entry_list.ini");
    let base = std::fs::read_to_string(&path)
        .map_err(|error| format!("entry_list.ini non leggibile: {}", error))?;
    let (text, slot) = apply_reservation(&base, cap, &reservation)?;
    commands::backup_cfg(state)?;
    std::fs::write(&path, text).map_err(|error| error.to_string())?;
    Ok(slot)
}

/// Risposta HTTP completa, costruita senza socket (testabile).
fn response_text(status: u16, body: &serde_json::Value) -> String {
    let reason = match status {
        200 => "OK",
        403 => "Forbidden",
        429 => "Too Many Requests",
        _ => "Bad Request",
    };
    let text = body.to_string();
    format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{}",
        status,
        reason,
        text.len(),
        text
    )
}

fn respond(stream: &mut TcpStream, status: u16, body: serde_json::Value) {
    let _ = stream.write_all(response_text(status, &body).as_bytes());
}

fn handle(app: &AppHandle, mut stream: TcpStream) {
    let peer = stream
        .peer_addr()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_default();
    let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("").to_string();

    let mut length = 0usize;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).unwrap_or(0) == 0 {
            break;
        }
        if header.trim().is_empty() {
            break;
        }
        if let Some((key, value)) = header.split_once(':') {
            if key.eq_ignore_ascii_case("content-length") {
                length = value.trim().parse().unwrap_or(0);
            }
        }
    }
    let mut body = vec![0u8; length.min(8192)];
    if length > 8192 {
        return respond(
            &mut stream,
            400,
            json!({"ok": false, "error": messages::tr("richiesta troppo grande", "request too large")}),
        );
    }
    if length > 0 && reader.read_exact(&mut body).is_err() {
        return;
    }

    let (path, query) = target.split_once('?').unwrap_or((target.as_str(), ""));
    let params: HashMap<String, String> = query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (key.to_lowercase(), value.to_string())
        })
        .collect();

    let state = app.state::<AppState>();

    match (method.as_str(), path) {
        ("GET", "/api/server") => {
            if !rate_ok(&peer, 120) {
                return respond(
                    &mut stream,
                    429,
                    json!({"ok": false, "error": messages::tr("troppe richieste", "too many requests")}),
                );
            }
            if !token_ok(
                &state,
                params
                    .get("token")
                    .map(|value| value.as_str())
                    .unwrap_or(""),
            ) {
                return respond(
                    &mut stream,
                    403,
                    json!({"ok": false, "error": messages::tr("token non valido", "invalid token")}),
                );
            }
            respond(&mut stream, 200, server_payload(&state));
        }
        ("POST", "/api/reserve") => {
            if !rate_ok(&peer, 20) {
                return respond(
                    &mut stream,
                    429,
                    json!({"ok": false, "error": messages::tr("troppe richieste", "too many requests")}),
                );
            }
            let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap_or(json!({}));
            let token = parsed
                .get("token")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            if !token_ok(&state, token) {
                return respond(
                    &mut stream,
                    403,
                    json!({"ok": false, "error": messages::tr("token non valido", "invalid token")}),
                );
            }
            match reserve(&state, &parsed) {
                Ok(slot) => {
                    let _ = app.emit("entry-list-changed", slot);
                    respond(&mut stream, 200, json!({"ok": true, "slot": slot}))
                }
                Err(error) => respond(&mut stream, 200, json!({"ok": false, "error": error})),
            }
        }
        _ => respond(
            &mut stream,
            400,
            json!({"ok": false, "error": messages::tr("endpoint sconosciuto", "unknown endpoint")}),
        ),
    }
}

/// Avvia l'API su `requested`, provando le porte successive se occupata.
pub fn start(app: AppHandle, requested: u16) -> Result<u16, String> {
    let mut last_error = String::new();
    for port in requested..requested.saturating_add(20) {
        match TcpListener::bind(("0.0.0.0", port)) {
            Ok(listener) => {
                std::thread::spawn(move || {
                    for stream in listener.incoming() {
                        if let Ok(stream) = stream {
                            let app = app.clone();
                            std::thread::spawn(move || handle(&app, stream));
                        }
                    }
                });
                return Ok(port);
            }
            Err(error) => last_error = error.to_string(),
        }
    }
    Err(messages::tr(
        &format!("nessuna porta API libera: {}", last_error),
        &format!("no free API port: {}", last_error),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn store() -> Mutex<HashMap<String, Vec<Instant>>> {
        Mutex::new(HashMap::new())
    }

    #[test]
    fn token_never_matches_when_empty() {
        assert!(!token_matches("", ""));
        assert!(!token_matches("", "abc"));
        assert!(!token_matches("abc", ""));
        assert!(!token_matches("abc", "abd"));
        assert!(token_matches("abc", "abc"));
    }

    #[test]
    fn reservation_requires_driver_and_car() {
        let error = parse_reserve(&json!({"car": "bmw_z4_gt3", "guid": "76561198000000000"}))
            .expect_err("driver mancante");
        assert!(error.to_lowercase().contains("pilota") || error.to_lowercase().contains("driver"));

        let error = parse_reserve(&json!({"driver": "Player", "guid": "76561198000000000"}))
            .expect_err("car mancante");
        assert!(error.to_lowercase().contains("auto") || error.to_lowercase().contains("car"));
    }

    #[test]
    fn reservation_requires_numeric_steam_guid() {
        let body = json!({"driver": "Player", "car": "bmw_z4_gt3", "guid": ""});
        assert!(parse_reserve(&body).is_err());

        let body = json!({"driver": "Player", "car": "bmw_z4_gt3", "guid": "abc123"});
        assert!(parse_reserve(&body).is_err());

        let body = json!({"driver": "Player", "car": "bmw_z4_gt3", "guid": "76561198000000000"});
        let reservation = parse_reserve(&body).expect("payload valido");
        assert_eq!(reservation.driver, "Player");
        assert_eq!(reservation.car, "bmw_z4_gt3");
        assert!(reservation.skin.is_empty());
    }

    #[test]
    fn reservation_trims_values() {
        let body = json!({
            "driver": "  Player  ",
            "car": " bmw_z4_gt3 ",
            "skin": " one ",
            "guid": " 76561198000000000 ",
        });
        let reservation = parse_reserve(&body).expect("payload valido");
        assert_eq!(reservation.driver, "Player");
        assert_eq!(reservation.car, "bmw_z4_gt3");
        assert_eq!(reservation.skin, "one");
        assert_eq!(reservation.guid, "76561198000000000");
    }

    #[test]
    fn rate_limit_allows_then_blocks_then_recovers() {
        let store = store();
        let start = Instant::now();

        for _ in 0..3 {
            assert!(rate_allowed(&store, "10.0.0.1", 3, start));
        }
        assert!(!rate_allowed(&store, "10.0.0.1", 3, start));
        // un altro IP ha il suo contatore
        assert!(rate_allowed(&store, "10.0.0.2", 3, start));
        // dopo la finestra si riparte
        assert!(rate_allowed(
            &store,
            "10.0.0.1",
            3,
            start + RATE_WINDOW + Duration::from_secs(1)
        ));
    }

    #[test]
    fn response_has_status_and_content_length() {
        let text = response_text(403, &json!({"ok": false, "error": "nope"}));
        assert!(text.starts_with("HTTP/1.1 403 Forbidden\r\n"));
        assert!(text.contains("Content-Type: application/json"));
        let body = text.split("\r\n\r\n").nth(1).unwrap();
        assert!(text.contains(&format!("Content-Length: {}", body.len())));
        assert!(body.contains("nope"));

        let too_many = response_text(429, &json!({"ok": false}));
        assert!(too_many.starts_with("HTTP/1.1 429 Too Many Requests"));
    }
}
