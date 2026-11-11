//! Primitive condivise sul filesystem: riconoscimento dei link e scrittura
//! atomica usata da configurazioni, profili e launcher. Una sola implementazione
//! evita differenze di comportamento tra Manager e Launcher.

use crate::messages;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

static WRITE_SEQ: AtomicU64 = AtomicU64::new(0);

/// Un percorso è un link simbolico o una junction: su Windows entrambi sono
/// reparse point, altrove è un symlink. `Path::is_dir()`/`is_file()` li
/// seguirebbero, quindi non bastano per decidere se un percorso è fidato.
pub(crate) fn is_link(file_type: &fs::FileType) -> bool {
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

/// Scrittura atomica: temporaneo creato in esclusiva, contenuto scritto e
/// sincronizzato su disco, rilettura di controllo, poi rename. Se qualcosa
/// fallisce l'originale resta intatto e il temporaneo viene rimosso.
pub(crate) fn write_text_atomic(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("cfg");
    // nome univoco per singola scrittura: due salvataggi non si pestano
    let seq = WRITE_SEQ.fetch_add(1, Ordering::Relaxed);
    let temp = path.with_file_name(format!("{}.{}.{}.tmp", file_name, std::process::id(), seq));
    if let Err(error) = write_and_verify(&temp, text) {
        let _ = fs::remove_file(&temp);
        return Err(error);
    }
    fs::rename(&temp, path).map_err(|error| {
        let _ = fs::remove_file(&temp);
        format!("{}: {}", path.display(), error)
    })
}

/// Scrive il temporaneo (in esclusiva), lo sincronizza e rilegge il contenuto
/// prima del rename.
fn write_and_verify(temp: &Path, text: &str) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp)
        .map_err(|error| format!("{}: {}", temp.display(), error))?;
    file.write_all(text.as_bytes())
        .map_err(|error| format!("{}: {}", temp.display(), error))?;
    // i byte devono essere sul disco prima che il rename renda visibile il file
    file.sync_all()
        .map_err(|error| format!("{}: {}", temp.display(), error))?;
    drop(file);
    let written =
        fs::read_to_string(temp).map_err(|error| format!("{}: {}", temp.display(), error))?;
    if written != text {
        return Err(messages::tr(
            "scrittura incompleta: il file originale non è stato modificato",
            "incomplete write: the original file was left untouched",
        ));
    }
    Ok(())
}
