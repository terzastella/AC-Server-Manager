use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Entry {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Section {
    pub name: String,
    pub entries: Vec<Entry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Doc {
    pub sections: Vec<Section>,
    pub newline: String,
}

/// Parses an INI file keeping section and key order. Comments are ignored here
/// but preserved on disk by `apply`.
pub fn parse(text: &str) -> Doc {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut sections: Vec<Section> = Vec::new();
    let mut current: Option<usize> = None;

    for raw in text.lines() {
        let line = raw.trim_end_matches('\r');
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() > 2 {
            let name = trimmed[1..trimmed.len() - 1].trim().to_string();
            sections.push(Section {
                name,
                entries: Vec::new(),
            });
            current = Some(sections.len() - 1);
        } else if let (Some(index), Some(eq)) = (current, line.find('=')) {
            sections[index].entries.push(Entry {
                key: line[..eq].trim().to_string(),
                value: line[eq + 1..].trim().to_string(),
            });
        }
    }

    Doc {
        sections,
        newline: newline.to_string(),
    }
}

/// Rewrites the INI applying the model while keeping the original layout,
/// comments and any key the UI does not know about.
pub fn apply(base: &str, sections: &[Section]) -> String {
    let newline = if base.contains("\r\n") { "\r\n" } else { "\n" };

    let mut model: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for section in sections {
        model.insert(
            section.name.to_uppercase(),
            section
                .entries
                .iter()
                .map(|entry| (entry.key.trim().to_uppercase(), entry.value.clone()))
                .collect(),
        );
    }

    let mut used: HashMap<String, Vec<bool>> = model
        .iter()
        .map(|(name, entries)| (name.clone(), vec![false; entries.len()]))
        .collect();

    let mut out = String::new();
    let mut current = String::new();
    let mut seen: Vec<String> = Vec::new();

    for raw in base.split_inclusive('\n') {
        let line = raw.trim_end_matches('\n').trim_end_matches('\r');
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            append_missing(&mut out, &current, &model, &mut used, newline);
            current = trimmed[1..trimmed.len() - 1].trim().to_uppercase();
            if !seen.contains(&current) {
                seen.push(current.clone());
            }
            out.push_str(raw);
        } else if let Some(eq) = line.find('=') {
            let key = line[..eq].trim().to_uppercase();
            let mut replaced = false;
            if let Some(entries) = model.get(&current) {
                if let Some(pos) = entries.iter().position(|(k, _)| *k == key) {
                    if let Some(flags) = used.get_mut(&current) {
                        flags[pos] = true;
                    }
                    out.push_str(&line[..eq]);
                    out.push('=');
                    out.push_str(&entries[pos].1);
                    out.push_str(newline);
                    replaced = true;
                }
            }
            if !replaced {
                out.push_str(raw);
                if !raw.ends_with('\n') {
                    out.push_str(newline);
                }
            }
        } else {
            out.push_str(raw);
            if !raw.ends_with('\n') {
                out.push_str(newline);
            }
        }
    }
    append_missing(&mut out, &current, &model, &mut used, newline);

    for section in sections {
        let upper = section.name.to_uppercase();
        if seen.contains(&upper) {
            continue;
        }
        if !out.is_empty() && !out.ends_with('\n') {
            out.push_str(newline);
        }
        out.push_str(&format!("[{}]{}", section.name, newline));
        for entry in &section.entries {
            out.push_str(&format!("{}={}{}", entry.key, entry.value, newline));
        }
    }

    out
}

fn append_missing(
    out: &mut String,
    section: &str,
    model: &HashMap<String, Vec<(String, String)>>,
    used: &mut HashMap<String, Vec<bool>>,
    newline: &str,
) {
    if section.is_empty() {
        return;
    }
    let (Some(entries), Some(flags)) = (model.get(section), used.get_mut(section)) else {
        return;
    };
    for (index, (key, value)) in entries.iter().enumerate() {
        if flags[index] {
            continue;
        }
        if !out.is_empty() && !out.ends_with('\n') {
            out.push_str(newline);
        }
        out.push_str(&format!("{}={}{}", key, value, newline));
        flags[index] = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_keeps_comments_and_unknown_keys() {
        let base = "; comment\r\n[SERVER]\r\nNAME=Old\r\nUNKNOWN=42\r\n\r\n[RACE]\r\nLAPS=5\r\n";
        let doc = parse(base);
        assert_eq!(doc.sections.len(), 2);
        assert_eq!(doc.sections[0].entries[0].value, "Old");

        let mut sections = doc.sections.clone();
        sections[0].entries[0].value = "New".into();
        let output = apply(base, &sections);
        assert!(output.contains("; comment"));
        assert!(output.contains("NAME=New"));
        assert!(output.contains("UNKNOWN=42"));
        assert!(output.contains("LAPS=5"));
        assert!(output.contains("\r\n"));
    }

    #[test]
    fn adds_missing_key_and_section() {
        let base = "[SERVER]\nNAME=Old\n";
        let sections = vec![
            Section {
                name: "SERVER".into(),
                entries: vec![
                    Entry { key: "NAME".into(), value: "New".into() },
                    Entry { key: "PASSWORD".into(), value: "x".into() },
                ],
            },
            Section {
                name: "RACE".into(),
                entries: vec![Entry { key: "LAPS".into(), value: "8".into() }],
            },
        ];
        let output = apply(base, &sections);
        assert!(output.contains("PASSWORD=x"));
        assert!(output.contains("[RACE]"));
    }
}
