use crate::ini;
use serde::Serialize;
use std::{fs, path::{Path, PathBuf}};

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
            let track_root = Path::new(ac_root).join("content").join("tracks").join(&track).join("ui");
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

pub fn import_server_preset(source: &Path, destination_root: &Path) -> Result<PathBuf, String> {
    if !source.is_dir() {
        return Err(format!("preset non trovato: {}", source.display()));
    }
    let canonical = source
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let lowered = canonical.to_string_lossy().to_lowercase();
    if !lowered.contains("\\server\\presets\\") && !lowered.contains("/server/presets/") {
        return Err("il preset non proviene da una cartella server\\presets".into());
    }

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
