use crate::ini::{self, Entry, Section};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Aiuti di guida letti da `Documenti\Assetto Corsa\cfg\assists.ini`.
/// I valori numerici seguono i range della schermata Assists di AC.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Assists {
    pub ideal_line: bool,
    pub auto_blip: bool,
    pub auto_clutch: bool,
    pub auto_shifter: bool,
    /// 0 = Off, 1 = Fabbrica, 2 = On
    pub traction_control: u8,
    /// 0 = Off, 1 = Fabbrica, 2 = On
    pub abs: u8,
    /// 0..100 %
    pub stability_control: u8,
    /// 0..100 %
    pub damage: u8,
    /// 0 = Off, 1..3 = moltiplicatore
    pub tyre_wear: u8,
    /// 1..5 = moltiplicatore
    pub slipstream: u8,
    pub fuel_rate: bool,
    pub tyre_blankets: bool,
}

impl Default for Assists {
    fn default() -> Self {
        // Valori del preset "Custom" di Assetto Corsa.
        Self {
            ideal_line: false,
            auto_blip: false,
            auto_clutch: false,
            auto_shifter: false,
            traction_control: 0,
            abs: 1,
            stability_control: 0,
            damage: 0,
            tyre_wear: 1,
            slipstream: 1,
            fuel_rate: false,
            tyre_blankets: false,
        }
    }
}

/// Chiavi del file, nell'ordine in cui vengono scritte.
#[allow(clippy::type_complexity)]
const KEYS: [(&str, fn(&Assists) -> u8); 12] = [
    ("IDEAL_LINE", |a| a.ideal_line as u8),
    ("AUTO_BLIP", |a| a.auto_blip as u8),
    ("STABILITY_CONTROL", |a| a.stability_control),
    ("AUTO_SHIFTER", |a| a.auto_shifter as u8),
    ("ABS", |a| a.abs),
    ("TRACTION_CONTROL", |a| a.traction_control),
    ("AUTO_CLUTCH", |a| a.auto_clutch as u8),
    ("DAMAGE", |a| a.damage),
    ("FUEL_RATE", |a| a.fuel_rate as u8),
    ("TYRE_WEAR", |a| a.tyre_wear),
    ("TYRE_BLANKETS", |a| a.tyre_blankets as u8),
    ("SLIPSTREAM", |a| a.slipstream),
];

/// Cartella `...\Assetto Corsa\cfg` (Documenti, anche con OneDrive spostato).
pub fn ac_cfg_dir() -> Option<PathBuf> {
    let home = std::env::var_os("USERPROFILE").map(PathBuf::from)?;
    let mut documents = vec![home.join("Documents")];
    for var in ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"] {
        if let Some(dir) = std::env::var_os(var) {
            documents.push(PathBuf::from(dir).join("Documents"));
        }
    }

    let fallback = documents
        .first()
        .map(|dir| dir.join("Assetto Corsa").join("cfg"));
    for dir in documents {
        let cfg = dir.join("Assetto Corsa").join("cfg");
        if cfg.is_dir() {
            return Some(cfg);
        }
    }
    fallback
}

/// Percorso di `assists.ini`. Prova `Documenti` dell'utente e le varianti
/// OneDrive, nel caso la cartella sia stata spostata.
pub fn config_path() -> Option<PathBuf> {
    ac_cfg_dir().map(|cfg| cfg.join("assists.ini"))
}

fn number(value: &str, min: u8, max: u8) -> u8 {
    value
        .trim()
        .parse::<i32>()
        .map(|value| value.clamp(min as i32, max as i32) as u8)
        .unwrap_or(min)
}

/// Legge i valori dalla sezione `[ASSISTS]`, lasciando i predefiniti per le
/// chiavi assenti.
pub fn read_values(text: &str) -> Assists {
    let mut values = Assists::default();
    let doc = ini::parse(text);
    let Some(section) = doc
        .sections
        .iter()
        .find(|section| section.name.eq_ignore_ascii_case("ASSISTS"))
    else {
        return values;
    };

    for entry in &section.entries {
        let raw = entry.value.trim();
        let enabled = raw == "1";
        match entry.key.to_uppercase().as_str() {
            "IDEAL_LINE" => values.ideal_line = enabled,
            "AUTO_BLIP" => values.auto_blip = enabled,
            "AUTO_CLUTCH" => values.auto_clutch = enabled,
            "AUTO_SHIFTER" => values.auto_shifter = enabled,
            "TRACTION_CONTROL" => values.traction_control = number(raw, 0, 2),
            "ABS" => values.abs = number(raw, 0, 2),
            "STABILITY_CONTROL" => values.stability_control = number(raw, 0, 100),
            "DAMAGE" => values.damage = number(raw, 0, 100),
            "TYRE_WEAR" => values.tyre_wear = number(raw, 0, 3),
            "SLIPSTREAM" => values.slipstream = number(raw, 1, 5),
            "FUEL_RATE" => values.fuel_rate = enabled,
            "TYRE_BLANKETS" => values.tyre_blankets = enabled,
            _ => {}
        }
    }
    values
}

/// Riscrive il file aggiornando solo le 12 chiavi note. Commenti, ordine e
/// chiavi sconosciute restano intatti (vedi `ini::apply`).
pub fn apply_values(base: &str, values: &Assists) -> Result<String, String> {
    let doc = ini::parse(base);
    let mut sections: Vec<Section> = doc
        .sections
        .iter()
        .filter(|section| !section.name.eq_ignore_ascii_case("ASSISTS"))
        .cloned()
        .collect();

    let mut entries: Vec<Entry> = doc
        .sections
        .iter()
        .find(|section| section.name.eq_ignore_ascii_case("ASSISTS"))
        .map(|section| section.entries.clone())
        .unwrap_or_default();

    for (key, value_of) in KEYS {
        let value = value_of(values).to_string();
        if let Some(entry) = entries
            .iter_mut()
            .find(|entry| entry.key.eq_ignore_ascii_case(key))
        {
            if entry.value.trim() != value {
                entry.value = value;
            }
        } else {
            entries.push(Entry {
                key: key.to_string(),
                value,
            });
        }
    }

    sections.push(Section {
        name: "ASSISTS".into(),
        entries,
    });
    ini::apply(base, &sections)
}

pub fn load(path: &Path) -> Assists {
    fs::read_to_string(path)
        .map(|text| read_values(&text))
        .unwrap_or_default()
}

pub fn save(path: &Path, values: &Assists) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    // un file illeggibile non è un file vuoto: sovrascriverlo perderebbe la
    // configurazione che non si è riusciti a leggere
    let base = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            return Err(crate::messages::tr(
                &format!(
                    "assists.ini non leggibile ({}): salvataggio annullato",
                    error
                ),
                &format!("assists.ini cannot be read ({}): save cancelled", error),
            ))
        }
    };
    // il backup deve riuscire, altrimenti non si tocca l'originale
    if path.is_file() {
        fs::copy(path, path.with_file_name("assists.ini.bak")).map_err(|error| {
            crate::messages::tr(
                &format!(
                    "backup di assists.ini non riuscito ({}): salvataggio annullato",
                    error
                ),
                &format!("assists.ini backup failed ({}): save cancelled", error),
            )
        })?;
    }
    let text = apply_values(&base, values)?;
    crate::fsx::write_text_atomic(path, &text)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "; config\r\n[ASSISTS]\r\nIDEAL_LINE=0\r\nAUTO_BLIP=1\r\nSTABILITY_CONTROL=50\r\nAUTO_SHIFTER=0\r\nABS=2\r\nTRACTION_CONTROL=1\r\nAUTO_CLUTCH=1\r\nVISUALDAMAGE=20\r\nDAMAGE=20\r\nFUEL_RATE=0\r\nTYRE_WEAR=2\r\nTYRE_BLANKETS=1\r\nSLIPSTREAM=3\r\n";

    #[test]
    fn reads_levels_and_booleans() {
        let values = read_values(SAMPLE);
        assert_eq!(values.stability_control, 50);
        assert_eq!(values.damage, 20);
        assert_eq!(values.abs, 2);
        assert_eq!(values.traction_control, 1);
        assert_eq!(values.tyre_wear, 2);
        assert_eq!(values.slipstream, 3);
        assert!(values.tyre_blankets);
        assert!(!values.fuel_rate);
        assert!(values.auto_blip);
        assert!(values.auto_clutch);
    }

    #[test]
    fn missing_keys_keep_defaults_and_values_are_clamped() {
        let values = read_values("[ASSISTS]\nIDEAL_LINE=1\nSTABILITY_CONTROL=250\nSLIPSTREAM=0\n");
        assert!(values.ideal_line);
        assert_eq!(values.stability_control, 100);
        assert_eq!(values.slipstream, 1);
        assert_eq!(values.abs, Assists::default().abs);
    }

    #[test]
    fn apply_keeps_comments_and_unknown_keys() {
        let mut values = read_values(SAMPLE);
        values.traction_control = 0;
        values.stability_control = 100;
        values.slipstream = 5;

        let output = apply_values(SAMPLE, &values).expect("apply");
        assert!(output.contains("; config"));
        assert!(output.contains("TRACTION_CONTROL=0"));
        assert!(output.contains("STABILITY_CONTROL=100"));
        assert!(output.contains("SLIPSTREAM=5"));
        assert!(output.contains("VISUALDAMAGE=20"));
        assert!(output.contains("\r\n"));
    }

    #[test]
    fn apply_creates_section_and_all_keys() {
        let values = Assists::default();
        let output = apply_values("", &values).expect("apply");
        assert!(output.contains("[ASSISTS]"));
        for (key, _) in KEYS {
            assert!(output.contains(&format!("{key}=")), "manca {key}");
        }
        assert_eq!(read_values(&output), values);
    }

    #[test]
    fn save_writes_backup_and_file() {
        let dir = std::env::temp_dir().join(format!("ac-assists-{}", std::process::id()));
        let path = dir.join("assists.ini");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(&path, SAMPLE).unwrap();

        let mut values = read_values(SAMPLE);
        values.damage = 100;
        save(&path, &values).unwrap();

        assert!(path.with_file_name("assists.ini.bak").is_file());
        assert!(fs::read_to_string(&path).unwrap().contains("DAMAGE=100"));
        assert_eq!(read_values(&fs::read_to_string(&path).unwrap()), values);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_refuses_an_unreadable_file_and_leaves_it_intact() {
        let dir =
            std::env::temp_dir().join(format!("acsc-assists-unreadable-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("cartella");
        let path = dir.join("assists.ini");
        let original = vec![0xFFu8, 0x00, 0xFE];
        fs::write(&path, &original).expect("file");
        let values = read_values(SAMPLE);

        assert!(save(&path, &values).is_err());
        // il file illeggibile non viene trattato come vuoto né sovrascritto
        assert_eq!(fs::read(&path).expect("letto"), original);
        let temporaries = fs::read_dir(&dir)
            .expect("dir")
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(temporaries, 0, "nessun temporaneo deve restare");
        let _ = fs::remove_dir_all(&dir);
    }
}
