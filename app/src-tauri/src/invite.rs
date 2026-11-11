use crate::messages;
use serde::{Deserialize, Serialize};

/// Parametri trasportati dal link `acservercontrol://join?...`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct JoinArgs {
    pub ip: String,
    pub http_port: u16,
    pub api_port: u16,
    pub token: String,
    pub name: String,
    pub accent: String,
    pub banner: String,
}

/// Token casuale da un CSPRNG: 32 byte in base64 url-safe senza padding.
/// L'errore viene propagato: meglio un messaggio in interfaccia che un
/// invito salvato con un token vuoto che nessuno può usare.
pub fn new_token() -> Result<String, String> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|error| {
        messages::tr(
            &format!("generatore casuale non disponibile: {}", error),
            &format!("random generator unavailable: {}", error),
        )
    })?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{:02X}", byte)),
        }
    }
    out
}

fn decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && bytes[index + 1].is_ascii_hexdigit()
            && bytes[index + 2].is_ascii_hexdigit()
        {
            let hex = &value[index + 1..index + 3];
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                out.push(byte);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

/// Accetta IPv4, IPv6 e hostname semplici: niente metacaratteri di shell o URL.
pub fn valid_host(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 253
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == ':' || c == '-' || c == '_')
}

/// URL dell'API: un IPv6 va racchiuso tra parentesi quadre, altrimenti il
/// primo `:` viene letto come separatore di porta.
pub fn api_url(host: &str, port: u16, path: &str) -> String {
    let host = host.trim();
    let authority = if host.contains(':') && !host.starts_with('[') {
        format!("[{}]", host)
    } else {
        host.to_string()
    };
    format!("http://{}:{}{}", authority, port, path)
}

pub fn build_link(ip: &str, http_port: u16, invite: &crate::state::InviteSettings) -> String {
    format!(
        "acservercontrol://join?ip={}&httpPort={}&apiPort={}&token={}&n={}&c={}&b={}",
        encode(ip),
        http_port,
        invite.api_port,
        encode(&invite.token),
        encode(&invite.name),
        encode(&invite.accent),
        encode(&invite.banner)
    )
}

pub fn parse_join_uri(uri: &str) -> Result<JoinArgs, String> {
    let trimmed = uri.trim();
    let lower = trimmed.to_lowercase();
    if !lower.starts_with("acservercontrol://") {
        return Err(messages::tr(
            "link non riconosciuto: serve acservercontrol://join?...",
            "unrecognised link: acservercontrol://join?... required",
        ));
    }
    let query = trimmed
        .split_once('?')
        .map(|(_, query)| query)
        .ok_or_else(|| "link senza parametri".to_string())?;

    let mut map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        map.insert(decode(key).to_lowercase(), decode(value));
    }

    let ip = map
        .get("ip")
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .ok_or_else(|| "parametro ip mancante".to_string())?;
    if !valid_host(&ip) {
        return Err(messages::tr(
            "parametro ip non valido",
            "invalid ip parameter",
        ));
    }
    let http_port = map
        .get("httpport")
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| "parametro httpPort mancante o non valido".to_string())?;
    let api_port = map
        .get("apiport")
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| "parametro apiPort mancante o non valido".to_string())?;
    let token = map
        .get("token")
        .filter(|value| !value.is_empty())
        .cloned()
        .ok_or_else(|| "parametro token mancante".to_string())?;

    Ok(JoinArgs {
        ip,
        http_port,
        api_port,
        token,
        name: map.get("n").cloned().unwrap_or_default(),
        accent: map
            .get("c")
            .cloned()
            .unwrap_or_else(|| "#ff5a1f".to_string()),
        banner: map.get("b").cloned().unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::InviteSettings;

    #[test]
    fn token_is_random_base64url() {
        let token = new_token().expect("generatore disponibile");
        // 32 byte -> 43 caratteri base64url senza padding
        assert_eq!(token.len(), 43);
        assert!(token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }

    #[test]
    fn tokens_do_not_repeat() {
        let first = new_token().expect("generatore disponibile");
        let second = new_token().expect("generatore disponibile");
        assert_ne!(first, second);
        assert!(!first.is_empty());
    }

    #[test]
    fn api_url_brackets_ipv6() {
        assert_eq!(
            api_url("192.168.1.10", 8082, "/api/server"),
            "http://192.168.1.10:8082/api/server"
        );
        assert_eq!(
            api_url("::1", 8082, "/api/server"),
            "http://[::1]:8082/api/server"
        );
        assert_eq!(
            api_url("2001:db8::5", 9000, "/x"),
            "http://[2001:db8::5]:9000/x"
        );
        assert_eq!(api_url("[::1]", 8082, "/x"), "http://[::1]:8082/x");
        assert_eq!(
            api_url("vpn.example.com", 8082, "/x"),
            "http://vpn.example.com:8082/x"
        );
    }

    #[test]
    fn link_carries_all_params() {
        let invite = InviteSettings {
            token: "abc".into(),
            name: "GT3 Night".into(),
            accent: "#00aaff".into(),
            banner: "https://x/y.jpg".into(),
            api_port: 8082,
            api_bind: "0.0.0.0".into(),
            expires_at: None,
        };
        let link = build_link("203.0.113.10", 8081, &invite);
        assert_eq!(
            link,
            "acservercontrol://join?ip=203.0.113.10&httpPort=8081&apiPort=8082&token=abc&n=GT3%20Night&c=%2300aaff&b=https%3A%2F%2Fx%2Fy.jpg"
        );
    }

    #[test]
    fn link_roundtrip() {
        let invite = InviteSettings {
            token: "abc123".into(),
            name: "Ciao & bello".into(),
            accent: "#ff5a1f".into(),
            banner: String::new(),
            api_port: 8082,
            api_bind: "0.0.0.0".into(),
            expires_at: None,
        };
        let link = build_link("203.0.113.10", 8081, &invite);
        let args = parse_join_uri(&link).expect("link valido");
        assert_eq!(args.ip, "203.0.113.10");
        assert_eq!(args.http_port, 8081);
        assert_eq!(args.token, "abc123");
        assert_eq!(args.name, "Ciao & bello");
        assert_eq!(args.api_port, 8082);
    }

    #[test]
    fn missing_params_are_reported() {
        assert!(parse_join_uri("acservercontrol://join?ip=1.2.3.4").is_err());
        assert!(parse_join_uri("https://example.com").is_err());
    }
}
