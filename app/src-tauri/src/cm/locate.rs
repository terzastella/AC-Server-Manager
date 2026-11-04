use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Debug, Serialize)]
pub struct CmLocation {
    pub cm_exe: Option<String>,
    pub data_dir: Option<String>,
    pub values_file: Option<String>,
    pub ac_root: Option<String>,
    pub version: Option<String>,
    pub running: bool,
}

fn candidate_dirs(custom: Option<&str>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(custom) = custom {
        if !custom.trim().is_empty() {
            dirs.push(PathBuf::from(custom));
        }
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        dirs.push(PathBuf::from(local).join("AcTools Content Manager"));
    }
    if let Ok(roaming) = std::env::var("APPDATA") {
        dirs.push(PathBuf::from(roaming).join("AcTools Content Manager"));
    }
    dirs
}

pub fn is_running() -> bool {
    let mut command = Command::new("tasklist");
    command.args(["/FI", "IMAGENAME eq Content Manager.exe", "/NH"]);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    command
        .output()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .to_lowercase()
                .contains("content manager.exe")
        })
        .unwrap_or(false)
}

pub fn locate(custom: Option<&str>) -> CmLocation {
    let mut location = CmLocation {
        cm_exe: None,
        data_dir: None,
        values_file: None,
        ac_root: None,
        version: None,
        running: is_running(),
    };

    for dir in candidate_dirs(custom) {
        let values = dir.join("Values.data");
        if !values.is_file() {
            continue;
        }
        location.data_dir = Some(dir.to_string_lossy().to_string());
        location.values_file = Some(values.to_string_lossy().to_string());
        if let Ok(entries) = crate::cm::storage::read_file(&values) {
            location.ac_root = crate::cm::storage::find_value(&entries, "_ac_root");
            location.version =
                crate::cm::storage::find_value(&entries, "AppUpdater:PreviousVersion");
            if let Some(exe) =
                crate::cm::storage::find_value(&entries, "CustomUriSchemeHelper.RegisteredLocation")
            {
                if Path::new(&exe).is_file() {
                    location.cm_exe = Some(exe);
                }
            }
        }
        break;
    }

    location
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn locates_local_content_manager() {
        let location = locate(None);
        assert!(
            location.values_file.is_some(),
            "Values.data non trovato: {:?}",
            location
        );
        assert!(
            location.ac_root.is_some(),
            "AC root non letta: {:?}",
            location
        );
        assert!(!location.running);
    }
}
