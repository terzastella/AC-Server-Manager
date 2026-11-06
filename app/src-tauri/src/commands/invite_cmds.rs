use super::*;

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
        return Err(messages::tr(
            "la porta API deve essere almeno 1024",
            "the API port must be at least 1024",
        ));
    }
    let accent = invite.accent.trim();
    let valid = accent.len() == 7
        && accent.starts_with('#')
        && accent[1..].chars().all(|c| c.is_ascii_hexdigit());
    if !valid {
        return Err(messages::tr(
            "colore accento non valido: usa #rrggbb",
            "invalid accent colour: use #rrggbb",
        ));
    }
    if invite.api_bind.parse::<std::net::IpAddr>().is_err() {
        return Err(messages::tr(
            "indirizzo di ascolto dell'API non valido",
            "invalid API bind address",
        ));
    }
    Ok(())
}

#[tauri::command]
pub fn save_invite_settings(
    state: State<AppState>,
    invite: InviteSettings,
) -> Result<InviteSettings, String> {
    validate_invite(&invite)?;
    // la generazione può fallire: meglio un errore che un invito con token vuoto
    let token = if invite.token.trim().is_empty() {
        crate::invite::new_token()?
    } else {
        invite.token.trim().to_string()
    };
    let mut settings = state.settings.lock().unwrap();
    settings.invite = InviteSettings { token, ..invite };
    let saved = settings.invite.clone();
    drop(settings);
    state.persist();
    Ok(saved)
}

#[tauri::command]
pub fn regenerate_invite_token(state: State<AppState>) -> Result<InviteSettings, String> {
    let token = crate::invite::new_token()?;
    let mut settings = state.settings.lock().unwrap();
    settings.invite.token = token;
    let saved = settings.invite.clone();
    drop(settings);
    state.persist();
    Ok(saved)
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
        settings
            .saved_invites
            .insert(0, crate::state::SavedInvite { link: link.clone() });
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
        return Err(messages::tr(
            "indirizzo del server non valido",
            "invalid server address",
        ));
    }
    let url = crate::invite::api_url(&host, api_port, "/api/server");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .build()
        .map_err(|error| error.to_string())?;
    let response = client
        .get(url)
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|error| format!("server non raggiungibile: {}", error))?;
    if response.status().as_u16() == 403 {
        return Err(messages::tr(
            "token non valido: chiedi un link aggiornato",
            "invalid token: ask for a fresh link",
        ));
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
        return Err(messages::tr(
            "indirizzo del server non valido",
            "invalid server address",
        ));
    }
    let url = crate::invite::api_url(&host, api_port, "/api/reserve");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .build()
        .map_err(|error| error.to_string())?;
    let response = client
        .post(url)
        .bearer_auth(&token)
        .json(&serde_json::json!({
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
        return Err(messages::tr(
            "token non valido: chiedi un link aggiornato",
            "invalid token: ask for a fresh link",
        ));
    }
    serde_json::from_str(&text).map_err(|error| format!("risposta non valida: {}", error))
}
