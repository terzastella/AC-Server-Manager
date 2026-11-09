use crate::ini;
use serde::Serialize;
use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

/// Esito del controllo di contenimento: "non esiste" e "fuori" sono errori diversi e
/// i comandi mostrano messaggi diversi.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfineError {
    Missing,
    Outside,
}

impl fmt::Display for ConfineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfineError::Missing => write!(formatter, "percorso non trovato"),
            ConfineError::Outside => write!(formatter, "percorso fuori dalla cartella consentita"),
        }
    }
}

/// Canonicalizza `candidate` e verifica che stia **dentro** `base`, mai uguale.
/// Il confronto è su percorsi canonici, quindi regge anche junction e symlink.
pub fn canonical_inside(candidate: &Path, base: &Path) -> Result<PathBuf, ConfineError> {
    let canonical_base = base.canonicalize().map_err(|_| ConfineError::Missing)?;
    let canonical = candidate
        .canonicalize()
        .map_err(|_| ConfineError::Missing)?;
    if canonical == canonical_base || !canonical.starts_with(&canonical_base) {
        return Err(ConfineError::Outside);
    }
    Ok(canonical)
}

/// Un preset importabile: il percorso canonico deve stare in una cartella
/// `...\server\presets\<nome>`. Non serve un confronto testuale sul path.
pub fn canonical_server_preset(source: &Path) -> Result<PathBuf, ConfineError> {
    let canonical = source.canonicalize().map_err(|_| ConfineError::Missing)?;
    if !canonical.is_dir() {
        return Err(ConfineError::Missing);
    }
    let presets = canonical.parent().ok_or(ConfineError::Outside)?;
    let server = presets.parent().ok_or(ConfineError::Outside)?;
    let presets_ok = presets
        .file_name()
        .map(|name| name.eq_ignore_ascii_case("presets"))
        .unwrap_or(false);
    let server_ok = server
        .file_name()
        .map(|name| name.eq_ignore_ascii_case("server"))
        .unwrap_or(false);
    if !presets_ok || !server_ok {
        return Err(ConfineError::Outside);
    }
    Ok(canonical)
}

#[derive(Clone, Debug, Serialize)]
pub struct CmServerPreset {
    pub name: String,
    pub path: String,
    pub server_name: String,
    pub track: String,
    pub cars: Vec<String>,
    pub max_clients: String,
    pub password: bool,
    pub files: Vec<String>,
    pub has_wrapper: bool,
    pub preview: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CmUserPreset {
    pub category: String,
    pub name: String,
    pub path: String,
    pub size: u64,
}

fn section_value(sections: &[ini::Section], section: &str, key: &str) -> Option<String> {
    sections
        .iter()
        .find(|item| item.name.eq_ignore_ascii_case(section))
        .and_then(|item| {
            item.entries
                .iter()
                .find(|entry| entry.key.eq_ignore_ascii_case(key))
                .map(|entry| entry.value.clone())
        })
}

fn dir_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

pub fn scan_server_presets(ac_root: &str) -> Vec<CmServerPreset> {
    let root = Path::new(ac_root).join("server").join("presets");
    let mut presets = Vec::new();
    let Ok(entries) = fs::read_dir(&root) else {
        return presets;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let config_path = path.join("server_cfg.ini");
        let config = fs::read_to_string(&config_path).unwrap_or_default();
        let sections = ini::parse(&config).sections;
        let files = fs::read_dir(&path)
            .map(|list| {
                list.flatten()
                    .map(|item| item.file_name().to_string_lossy().to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let track = section_value(&sections, "SERVER", "TRACK").unwrap_or_default();
        let preview = if track.is_empty() {
            None
        } else {
            let track_root = Path::new(ac_root)
                .join("content")
                .join("tracks")
                .join(&track)
                .join("ui");
            ["preview.png", "preview.jpg"]
                .iter()
                .map(|name| track_root.join(name))
                .find(|candidate| candidate.is_file())
                .map(|candidate| candidate.to_string_lossy().to_string())
        };

        presets.push(CmServerPreset {
            name: dir_name(&path),
            path: path.to_string_lossy().to_string(),
            server_name: section_value(&sections, "SERVER", "NAME").unwrap_or_default(),
            track,
            cars: section_value(&sections, "SERVER", "CARS")
                .map(|value| {
                    value
                        .split(';')
                        .map(|item| item.trim().to_string())
                        .filter(|item| !item.is_empty())
                        .collect()
                })
                .unwrap_or_default(),
            max_clients: section_value(&sections, "SERVER", "MAX_CLIENTS").unwrap_or_default(),
            password: section_value(&sections, "SERVER", "PASSWORD")
                .map(|value| !value.is_empty())
                .unwrap_or(false),
            has_wrapper: path.join("cm_wrapper_params.json").is_file(),
            files,
            preview,
        });
    }

    presets.sort_by(|a, b| a.name.cmp(&b.name));
    presets
}

pub fn scan_user_presets(data_dir: &str) -> Vec<CmUserPreset> {
    let root = Path::new(data_dir).join("Presets");
    let mut presets = Vec::new();
    collect_presets(&root, &root, 0, &mut presets);
    presets.sort_by(|a, b| a.category.cmp(&b.category).then(a.name.cmp(&b.name)));
    presets
}

fn collect_presets(root: &Path, current: &Path, depth: usize, out: &mut Vec<CmUserPreset>) {
    if depth > 4 {
        return;
    }
    let Ok(entries) = fs::read_dir(current) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_presets(root, &path, depth + 1, out);
            continue;
        }
        if !path
            .extension()
            .map(|ext| ext.eq_ignore_ascii_case("cmpreset"))
            .unwrap_or(false)
        {
            continue;
        }
        let category = path
            .parent()
            .and_then(|parent| parent.strip_prefix(root).ok())
            .map(|relative| relative.to_string_lossy().to_string())
            .unwrap_or_default();
        out.push(CmUserPreset {
            category,
            name: dir_name(&path),
            size: entry.metadata().map(|meta| meta.len()).unwrap_or(0),
            path: path.to_string_lossy().to_string(),
        });
    }
}

/// Un link simbolico o una junction: copiarne il contenuto uscirebbe dal preset.
fn is_link(file_type: &fs::FileType) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileTypeExt;
        file_type.is_symlink_dir() || file_type.is_symlink_file()
    }
    #[cfg(not(windows))]
    {
        file_type.is_symlink()
    }
}

fn copy_tree(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        let file_type = entry.file_type()?;
        // Un link dentro il preset potrebbe copiare contenuto fuori dalla cartella:
        // canonicalizzare la radice non basta, il controllo va fatto su ogni voce.
        // Su Windows anche le junction sono reparse point e vanno rifiutate.
        if is_link(&file_type) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!(
                    "link o junction non consentiti nel preset: {}",
                    entry.path().display()
                ),
            ));
        }
        if file_type.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

pub fn import_server_preset(source: &Path, destination_root: &Path) -> Result<PathBuf, String> {
    let canonical = canonical_server_preset(source).map_err(|error| match error {
        ConfineError::Missing => format!("preset non trovato: {}", source.display()),
        ConfineError::Outside => {
            "il preset non proviene da una cartella server\\presets".to_string()
        }
    })?;

    let name = dir_name(&canonical);
    let mut destination = destination_root.join(&name);
    if destination.exists() {
        destination = destination_root.join(format!("{}-cm", name));
    }
    if destination.exists() {
        destination = destination_root.join(format!(
            "{}-cm-{}",
            name,
            chrono::Local::now().format("%Y%m%d-%H%M%S")
        ));
    }

    copy_tree(&canonical, &destination).map_err(|error| error.to_string())?;
    Ok(destination)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sandbox(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("acsc-cm-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("server").join("presets").join("buono")).expect("preset");
        fs::create_dir_all(root.join("server").join("altro")).expect("altro");
        fs::write(
            root.join("server")
                .join("presets")
                .join("buono")
                .join("server_cfg.ini"),
            "[SERVER]\n",
        )
        .expect("file");
        root
    }

    #[test]
    fn a_preset_inside_server_presets_is_accepted() {
        let root = sandbox("ok");
        let source = root.join("server").join("presets").join("buono");

        let canonical = canonical_server_preset(&source).expect("preset valido");

        assert!(canonical.ends_with("buono"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_folder_outside_server_presets_is_rejected() {
        let root = sandbox("outside");
        let source = root.join("server").join("altro");

        assert_eq!(canonical_server_preset(&source), Err(ConfineError::Outside));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_missing_folder_is_reported_as_missing() {
        let root = sandbox("missing");
        let source = root.join("server").join("presets").join("manca");

        assert_eq!(canonical_server_preset(&source), Err(ConfineError::Missing));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn canonical_inside_rejects_the_base_and_the_escapes() {
        let root = sandbox("inside");
        let base = root.join("server").join("presets");

        assert!(canonical_inside(&base.join("buono"), &base).is_ok());
        assert_eq!(canonical_inside(&base, &base), Err(ConfineError::Outside));
        assert_eq!(
            canonical_inside(&root.join("server").join("altro"), &base),
            Err(ConfineError::Outside)
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(windows)]
    #[test]
    fn a_junction_inside_a_preset_is_refused() {
        let root = sandbox("junction");
        let source = root.join("server").join("presets").join("buono");
        let outside = root.join("server").join("altro");
        fs::write(outside.join("segreto.txt"), "fuori").expect("file fuori");
        // Una junction dentro il preset punta fuori dalla cartella: copiarla
        // porterebbe contenuto estraneo dentro la configurazione.
        junction::create(&outside, source.join("link")).expect("junction creata");

        let error = import_server_preset(&source, &root.join("dest"))
            .expect_err("la junction deve essere rifiutata");
        assert!(
            error.to_lowercase().contains("link") || error.to_lowercase().contains("junction"),
            "{}",
            error
        );
        assert!(!root.join("dest").join("link").exists());

        let _ = fs::remove_dir_all(&root);
    }
}
