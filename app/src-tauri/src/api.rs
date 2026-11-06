use serde_json::json;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

use crate::commands;
use crate::messages;
use crate::slots::{apply_reservation, free_slot_count, Reservation};
use crate::state::AppState;

static WRITE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static RATE: OnceLock<Mutex<HashMap<String, Vec<Instant>>>> = OnceLock::new();

/// Connessioni contemporanee massime: oltre, il client viene chiuso subito.
const MAX_CONNECTIONS: usize = 32;
static ACTIVE: AtomicUsize = AtomicUsize::new(0);

/// Tetto complessivo per riga di richiesta + header (anti slowloris).
const MAX_HEADER_BYTES: usize = 8 * 1024;
/// Tetto per il corpo delle richieste.
const MAX_BODY_BYTES: usize = 8 * 1024;
/// Timeout di lettura/scrittura per singola connessione.
const IO_TIMEOUT: Duration = Duration::from_secs(5);

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
/// a ogni aggiornamento.
fn rate_ok(peer: &str, max: usize) -> bool {
    let store = RATE.get_or_init(|| Mutex::new(HashMap::new()));
    rate_allowed(store, peer, max, Instant::now())
}

/// Confronto a tempo costante: il tempo impiegato non dipende da quanti
/// caratteri combaciano. La lunghezza del token è pubblica (43 caratteri
/// base64url), quindi uscire subito su lunghezza diversa non rivela altro.
fn token_matches(expected: &str, provided: &str) -> bool {
    let expected = expected.as_bytes();
    let provided = provided.as_bytes();
    if expected.is_empty() || expected.len() != provided.len() {
        return false;
    }
    let mut diff = 0u8;
    for (left, right) in expected.iter().zip(provided.iter()) {
        diff |= left ^ right;
    }
    diff == 0
}

/// Verifica token + scadenza. `now` esplicito per i test.
fn invite_valid(expected: &str, provided: &str, expires_at: Option<i64>, now: i64) -> bool {
    if let Some(expires) = expires_at {
        if now >= expires {
            return false;
        }
    }
    token_matches(expected, provided)
}

fn token_ok(state: &AppState, provided: &str) -> bool {
    let guard = state.settings.lock().unwrap();
    invite_valid(
        &guard.invite.token,
        provided,
        guard.invite.expires_at,
        chrono::Local::now().timestamp(),
    )
}

/// SteamID64 di un account individuale: 17 cifre e base + 32 bit di account.
fn valid_steam_id64(guid: &str) -> bool {
    const BASE: u64 = 76_561_197_960_265_728;
    const MAX_OFFSET: u64 = u32::MAX as u64;
    guid.len() == 17
        && guid.chars().all(|c| c.is_ascii_digit())
        && guid
            .parse::<u64>()
            .map(|value| (BASE..=BASE + MAX_OFFSET).contains(&value))
            .unwrap_or(false)
}

// --------------------------------------------------------------- richiesta

#[derive(Debug)]
struct Request {
    method: String,
    path: String,
    query: HashMap<String, String>,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

#[derive(Debug, PartialEq)]
enum RequestError {
    Malformed,
    TooLarge,
}

fn split_target(target: &str) -> (String, HashMap<String, String>) {
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let params = query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (key.to_lowercase(), value.to_string())
        })
        .collect();
    (path.to_string(), params)
}

/// Legge una richiesta HTTP con limiti espliciti su riga, header e corpo.
fn read_request<R: BufRead>(reader: &mut R) -> Result<Request, RequestError> {
    let mut head = (&mut *reader).take(MAX_HEADER_BYTES as u64);

    let mut request_line = String::new();
    if head.read_line(&mut request_line).unwrap_or(0) == 0 {
        return Err(RequestError::Malformed);
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("").to_string();
    if method.is_empty() || target.is_empty() {
        return Err(RequestError::Malformed);
    }

    let mut headers: HashMap<String, String> = HashMap::new();
    loop {
        let mut line = String::new();
        match head.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => return Err(RequestError::Malformed),
        }
        if line.trim().is_empty() {
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    // se gli header superano il tetto, `take` ha già troncato: il corpo non è attendibile
    if head.limit() == 0 {
        return Err(RequestError::TooLarge);
    }

    let length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    if length > MAX_BODY_BYTES {
        return Err(RequestError::TooLarge);
    }
    let mut body = vec![0u8; length];
    if length > 0 && reader.read_exact(&mut body).is_err() {
        return Err(RequestError::Malformed);
    }

    let (path, query) = split_target(&target);
    Ok(Request {
        method,
        path,
        query,
        headers,
        body,
    })
}

/// Token dall'header `Authorization: Bearer`, con fallback sul parametro
/// (GET) o sul campo `token` del corpo (POST) per i client più vecchi.
fn provided_token(request: &Request, body: Option<&serde_json::Value>) -> String {
    if let Some(value) = request.headers.get("authorization") {
        for prefix in ["Bearer ", "bearer "] {
            if let Some(rest) = value.strip_prefix(prefix) {
                return rest.trim().to_string();
            }
        }
    }
    if let Some(value) = request.query.get("token") {
        return value.clone();
    }
    body.and_then(|value| value.get("token"))
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string()
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
    if !valid_steam_id64(&guid) {
        return Err(messages::tr(
            "SteamID64 non valido: apri Steam e riprova, senza GUID lo slot non è utilizzabile",
            "invalid SteamID64: open Steam and retry, the slot is unusable without a GUID",
        ));
    }
    Ok(Reservation {
        driver,
        car,
        skin,
        guid,
    })
}

/// La livrea, quando indicata, deve appartenere all'auto scelta.
fn skin_exists(state: &AppState, car: &str, skin: &str) -> bool {
    if skin.trim().is_empty() {
        return true;
    }
    let library = commands::content_library_impl(state);
    library
        .cars
        .iter()
        .find(|item| item.id.eq_ignore_ascii_case(car))
        .map(|item| {
            item.skins
                .iter()
                .any(|entry| entry.name.eq_ignore_ascii_case(skin.trim()))
        })
        .unwrap_or(false)
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
    if !skin_exists(state, &car, &reservation.skin) {
        return Err(messages::tr(
            &format!("livrea non trovata per l'auto {}", car),
            &format!("skin not found for car {}", car),
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
    commands::write_text_atomic(&path, &text)?;
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
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{}",
        status,
        reason,
        text.len(),
        text
    )
}

fn respond(stream: &mut TcpStream, status: u16, body: serde_json::Value) {
    let _ = stream.write_all(response_text(status, &body).as_bytes());
}

/// Decrementa il contatore delle connessioni anche in caso di uscita anticipata.
struct ConnectionGuard;

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Operazioni che la connessione deve poter fare: implementato dallo stato
/// dell'app e da un finto backend nei test.
trait ApiBackend {
    fn authorized(&self, token: &str) -> bool;
    fn payload(&self) -> serde_json::Value;
    fn reserve(&self, body: &serde_json::Value) -> Result<usize, String>;
    fn slot_reserved(&self, slot: usize);
}

fn serve_connection(mut stream: TcpStream, backend: &dyn ApiBackend) {
    let _guard = ConnectionGuard;
    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(IO_TIMEOUT));

    let peer = stream
        .peer_addr()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_default();
    let mut reader = match stream.try_clone() {
        Ok(clone) => BufReader::new(clone),
        Err(_) => return,
    };

    let request = match read_request(&mut reader) {
        Ok(request) => request,
        Err(RequestError::TooLarge) => {
            return respond(
                &mut stream,
                400,
                json!({"ok": false, "error": messages::tr("richiesta troppo grande", "request too large")}),
            )
        }
        Err(RequestError::Malformed) => return,
    };

    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/api/server") => {
            if !rate_ok(&peer, 120) {
                return respond(
                    &mut stream,
                    429,
                    json!({"ok": false, "error": messages::tr("troppe richieste", "too many requests")}),
                );
            }
            if !backend.authorized(&provided_token(&request, None)) {
                return respond(
                    &mut stream,
                    403,
                    json!({"ok": false, "error": messages::tr("token non valido", "invalid token")}),
                );
            }
            respond(&mut stream, 200, backend.payload());
        }
        ("POST", "/api/reserve") => {
            if !rate_ok(&peer, 20) {
                return respond(
                    &mut stream,
                    429,
                    json!({"ok": false, "error": messages::tr("troppe richieste", "too many requests")}),
                );
            }
            let parsed: serde_json::Value =
                serde_json::from_slice(&request.body).unwrap_or(json!({}));
            if !backend.authorized(&provided_token(&request, Some(&parsed))) {
                return respond(
                    &mut stream,
                    403,
                    json!({"ok": false, "error": messages::tr("token non valido", "invalid token")}),
                );
            }
            match backend.reserve(&parsed) {
                Ok(slot) => {
                    backend.slot_reserved(slot);
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

struct AppBackend<'a>(&'a AppHandle);

impl ApiBackend for AppBackend<'_> {
    fn authorized(&self, token: &str) -> bool {
        let state = self.0.state::<AppState>();
        token_ok(&state, token)
    }

    fn payload(&self) -> serde_json::Value {
        let state = self.0.state::<AppState>();
        server_payload(&state)
    }

    fn reserve(&self, body: &serde_json::Value) -> Result<usize, String> {
        let state = self.0.state::<AppState>();
        reserve(&state, body)
    }

    fn slot_reserved(&self, slot: usize) {
        let _ = self.0.emit("entry-list-changed", slot);
    }
}

fn handle(app: &AppHandle, stream: TcpStream) {
    serve_connection(stream, &AppBackend(app));
}

/// Indirizzo di ascolto valido, senza scappatoie: un valore corrotto non deve
/// aprire l'API su tutte le interfacce di nascosto.
fn parse_bind(bind: &str) -> Result<IpAddr, String> {
    bind.trim().parse::<IpAddr>().map_err(|_| {
        messages::tr(
            &format!("indirizzo di ascolto non valido: {}", bind.trim()),
            &format!("invalid listen address: {}", bind.trim()),
        )
    })
}

/// Avvia l'API su `bind` (default tutte le interfacce), provando le porte
/// successive se occupata.
pub fn start(app: AppHandle, requested: u16, bind: &str) -> Result<u16, String> {
    let ip = parse_bind(bind)?;
    let mut last_error = String::new();
    for port in requested..requested.saturating_add(20) {
        match TcpListener::bind((ip, port)) {
            Ok(listener) => {
                std::thread::spawn(move || {
                    for stream in listener.incoming() {
                        let Ok(stream) = stream else { continue };
                        if ACTIVE.fetch_add(1, Ordering::SeqCst) >= MAX_CONNECTIONS {
                            ACTIVE.fetch_sub(1, Ordering::SeqCst);
                            continue;
                        }
                        let app = app.clone();
                        std::thread::spawn(move || handle(&app, stream));
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
    use std::io::Cursor;

    fn store() -> Mutex<HashMap<String, Vec<Instant>>> {
        Mutex::new(HashMap::new())
    }

    fn reader(raw: &str) -> Cursor<Vec<u8>> {
        Cursor::new(raw.as_bytes().to_vec())
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
    fn bind_must_parse() {
        assert_eq!(
            parse_bind("192.168.1.5").unwrap().to_string(),
            "192.168.1.5"
        );
        assert_eq!(parse_bind(" 0.0.0.0 ").unwrap().to_string(), "0.0.0.0");
        assert_eq!(parse_bind("::1").unwrap().to_string(), "::1");
        // un valore corrotto non deve diventare 0.0.0.0
        assert!(parse_bind("").is_err());
        assert!(parse_bind("192.168.1").is_err());
        assert!(parse_bind("localhost").is_err());
    }

    #[test]
    fn invite_expires() {
        let now = 1_000;
        assert!(invite_valid("abc", "abc", None, now));
        assert!(invite_valid("abc", "abc", Some(now + 1), now));
        assert!(!invite_valid("abc", "abc", Some(now), now));
        assert!(!invite_valid("abc", "abc", Some(now - 1), now));
    }

    #[test]
    fn steam_id64_must_be_a_real_account() {
        assert!(valid_steam_id64("76561198000000000"));
        assert!(valid_steam_id64("76561197960265728"));
        // 16 cifre, fuori range, non numerico
        assert!(!valid_steam_id64("7656119800000000"));
        assert!(!valid_steam_id64("12345678901234567"));
        assert!(!valid_steam_id64("7656119800000000a"));
        assert!(!valid_steam_id64(""));
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

    #[test]
    fn parses_request_with_query_and_headers() {
        let raw = "GET /api/server?token=abc&x=1 HTTP/1.1\r\nHost: x\r\nAuthorization: Bearer abc\r\n\r\n";
        let request = read_request(&mut reader(raw)).expect("richiesta valida");
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/api/server");
        assert_eq!(request.query.get("token").map(String::as_str), Some("abc"));
        assert_eq!(
            request.headers.get("authorization").map(String::as_str),
            Some("Bearer abc")
        );
        assert!(request.body.is_empty());
    }

    #[test]
    fn parses_body() {
        let raw = "POST /api/reserve HTTP/1.1\r\nContent-Length: 7\r\n\r\n{\"a\":1}";
        let request = read_request(&mut reader(raw)).expect("richiesta valida");
        assert_eq!(request.body, b"{\"a\":1}");
    }

    #[test]
    fn rejects_oversized_headers() {
        let mut raw = String::from("GET /api/server HTTP/1.1\r\n");
        for index in 0..400 {
            raw.push_str(&format!("X-Pad-{}: {}\r\n", index, "x".repeat(64)));
        }
        raw.push_str("\r\n");
        assert_eq!(
            read_request(&mut reader(&raw)).unwrap_err(),
            RequestError::TooLarge
        );
    }

    #[test]
    fn rejects_oversized_body_before_reading_it() {
        let raw = format!(
            "POST /api/reserve HTTP/1.1\r\nContent-Length: {}\r\n\r\n",
            MAX_BODY_BYTES + 1
        );
        assert_eq!(
            read_request(&mut reader(&raw)).unwrap_err(),
            RequestError::TooLarge
        );
    }

    #[test]
    fn rejects_malformed_request_line() {
        assert_eq!(
            read_request(&mut reader("\r\n")).unwrap_err(),
            RequestError::Malformed
        );
        assert_eq!(
            read_request(&mut reader("GET\r\n\r\n")).unwrap_err(),
            RequestError::Malformed
        );
    }

    #[test]
    fn token_prefers_authorization_header() {
        let raw = "GET /api/server?token=vecchio HTTP/1.1\r\nAuthorization: Bearer nuovo\r\n\r\n";
        let request = read_request(&mut reader(raw)).expect("richiesta valida");
        assert_eq!(provided_token(&request, None), "nuovo");
    }

    #[test]
    fn token_falls_back_to_query_then_body() {
        let request = read_request(&mut reader("GET /api/server?token=query HTTP/1.1\r\n\r\n"))
            .expect("richiesta valida");
        assert_eq!(provided_token(&request, None), "query");

        let request = read_request(&mut reader("POST /api/reserve HTTP/1.1\r\n\r\n"))
            .expect("richiesta valida");
        assert_eq!(
            provided_token(&request, Some(&json!({"token": "corpo"}))),
            "corpo"
        );
    }
    struct FakeBackend {
        token: String,
    }

    impl ApiBackend for FakeBackend {
        fn authorized(&self, token: &str) -> bool {
            token == self.token
        }

        fn payload(&self) -> serde_json::Value {
            json!({"name": "Test", "free_slots": 1})
        }

        fn reserve(&self, _body: &serde_json::Value) -> Result<usize, String> {
            Ok(2)
        }

        fn slot_reserved(&self, _slot: usize) {}
    }

    fn tcp_request(port: u16, raw: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connessione");
        stream.write_all(raw.as_bytes()).expect("scrittura");
        let mut response = String::new();
        stream.read_to_string(&mut response).expect("lettura");
        response
    }

    #[test]
    fn serves_requests_over_tcp() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("porta libera");
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let backend = FakeBackend {
                token: "abc".into(),
            };
            for _ in 0..3 {
                let (stream, _) = listener.accept().expect("connessione");
                serve_connection(stream, &backend);
            }
        });

        // senza token: 403
        let response = tcp_request(port, "GET /api/server HTTP/1.1\r\n\r\n");
        assert!(response.starts_with("HTTP/1.1 403"), "{}", response);

        // token nell'header: 200 con payload
        let response = tcp_request(
            port,
            "GET /api/server HTTP/1.1\r\nAuthorization: Bearer abc\r\n\r\n",
        );
        assert!(response.starts_with("HTTP/1.1 200"), "{}", response);
        assert!(response.contains("Test"));

        // prenotazione con token nel corpo
        let body = "{\"token\":\"abc\"}";
        let response = tcp_request(
            port,
            &format!(
                "POST /api/reserve HTTP/1.1\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            ),
        );
        assert!(response.contains("\"slot\":2"), "{}", response);

        server.join().expect("server");
    }
}
