use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use crate::cm::lzf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CmSetting {
    pub key: String,
    pub value: String,
    pub group: String,
    pub kind: String,
    pub encrypted: bool,
}

const LZF_FLAG: u8 = 11;
const DEFLATE_FLAG: u8 = 0;

pub fn decode_bytes(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if bytes[0] == LZF_FLAG {
        return lzf::decompress(&bytes[1..]);
    }
    if bytes[0] == DEFLATE_FLAG {
        return Err("Values.data in deflate: non supportato".into());
    }
    if bytes
        .iter()
        .any(|byte| *byte < 0x20 && *byte != b'\t' && *byte != b'\n' && *byte != b'\r')
    {
        return Err("Values.data compresso in deflate raw: non supportato".into());
    }
    Ok(bytes.to_vec())
}

fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            out.push(character);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

#[allow(dead_code)]
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

pub fn parse(bytes: &[u8]) -> Result<Vec<(String, String)>, String> {
    let decoded = decode_bytes(bytes)?;
    let text = String::from_utf8(decoded).map_err(|error| format!("UTF-8 non valido: {}", error))?;

    let newline = text.find('\n').ok_or("header Values.data mancante")?;
    let header = &text[..newline];
    if !header.starts_with("version: ") {
        return Err(format!("header sconosciuto: {}", header));
    }
    let version = header.trim_start_matches("version: ").trim();
    if version != "2" {
        return Err(format!("versione Values.data non supportata: {}", version));
    }

    let mut entries = Vec::new();
    for line in text[newline + 1..].split('\n') {
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('\t') else {
            continue;
        };
        entries.push((unescape(key), unescape(value)));
    }
    Ok(entries)
}

pub fn read_file(path: &Path) -> Result<Vec<(String, String)>, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {}", path.display(), error))?;
    parse(&bytes)
}

#[allow(dead_code)]
pub fn encode(entries: &[(String, String)]) -> Vec<u8> {
    let mut text = String::from("version: 2\n");
    for (key, value) in entries {
        text.push_str(&escape(key));
        text.push('\t');
        text.push_str(&escape(value));
        text.push('\n');
    }
    let mut out = Vec::with_capacity(text.len() + 1);
    out.push(LZF_FLAG);
    out.extend_from_slice(&lzf::compress(text.as_bytes()));
    out
}

fn group_of(key: &str) -> String {
    key.split(['.', ':'])
        .next()
        .unwrap_or(key)
        .trim()
        .to_string()
}

fn kind_of(value: &str) -> &'static str {
    if value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false") {
        return "bool";
    }
    if value == "0" || value == "1" {
        return "bool";
    }
    if value.parse::<i64>().is_ok() || value.parse::<f64>().is_ok() {
        return "number";
    }
    if (value.starts_with('{') || value.starts_with('['))
        && serde_json::from_str::<serde_json::Value>(value).is_ok()
    {
        return "json";
    }
    "string"
}

fn looks_encrypted(key: &str, value: &str) -> bool {
    let lower = key.to_lowercase();
    let sensitive = ["password", "token", "secret", "credential", "apikey", "api_key", "auth"]
        .iter()
        .any(|needle| lower.contains(needle));
    sensitive
        && value.len() >= 32
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '+' | '/' | '='))
}

pub fn settings_from(entries: &[(String, String)]) -> Vec<CmSetting> {
    entries
        .iter()
        .map(|(key, value)| CmSetting {
            key: key.clone(),
            value: value.clone(),
            group: group_of(key),
            kind: kind_of(value).to_string(),
            encrypted: looks_encrypted(key, value),
        })
        .collect()
}

pub fn find_value(entries: &[(String, String)], key: &str) -> Option<String> {
    entries
        .iter()
        .find(|(candidate, _)| candidate == key)
        .map(|(_, value)| value.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_with_escapes_and_unknown_keys() {
        let entries = vec![
            ("Settings.DriveSettings.PlayerName".to_string(), "Driver".to_string()),
            ("chiave\\strana".to_string(), "valore\tcon\ntab".to_string()),
            ("sconosciuta".to_string(), "{\"a\":1}".to_string()),
        ];
        let encoded = encode(&entries);
        assert_eq!(encoded[0], LZF_FLAG);
        let decoded = parse(&encoded).expect("parse");
        assert_eq!(decoded, entries);
    }

    #[test]
    fn rejects_unknown_version() {
        assert!(parse(b"version: 9\nA\tb\n").is_err());
    }

    #[test]
    fn classifies_settings() {
        let settings = settings_from(&[
            ("A".into(), "1".into()),
            ("B".into(), "{\"x\":1}".into()),
            ("Settings.OnlineSettings.ServerPassword".into(), "YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXo=".into()),
        ]);
        assert_eq!(settings[0].kind, "bool");
        assert_eq!(settings[1].kind, "json");
        assert!(settings[2].encrypted);
    }

    /// Reads the real Content Manager store of this machine. Run with
    /// `cargo test -- --ignored` after installing Content Manager.
    #[test]
    #[ignore]
    fn reads_local_content_manager_values() {
        let local = std::env::var("LOCALAPPDATA").expect("LOCALAPPDATA");
        let path = Path::new(&local)
            .join("AcTools Content Manager")
            .join("Values.data");
        if !path.is_file() {
            return;
        }
        let entries = read_file(&path).expect("parse Values.data");
        assert!(entries.len() > 50, "chiavi lette: {}", entries.len());
        assert!(find_value(&entries, "_ac_root").is_some());
    }
}
