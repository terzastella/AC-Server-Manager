//! Profili server: un indice in `profiles/index.json` e una cartella per profilo
//! (con `cfg/` propria e un junction `content` verso la radice condivisa).
//! Il profilo `default` è la cartella `cfg/` della radice: nessuna copia.

use crate::messages;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfileEntry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub legacy: bool,
    #[serde(default)]
    pub created: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProfileIndex {
    pub version: u32,
    pub active: String,
    pub profiles: Vec<ProfileEntry>,
}

fn now_iso() -> String {
    chrono::Local::now().to_rfc3339()
}

fn stamp() -> String {
    chrono::Local::now().format("%Y%m%d-%H%M%S").to_string()
}

fn default_profile() -> ProfileEntry {
    ProfileEntry {
        id: "default".into(),
        name: "Predefinito".into(),
        slug: String::new(),
        legacy: true,
        created: now_iso(),
    }
}

pub fn index_path(root: &Path) -> PathBuf {
    root.join("profiles").join("index.json")
}

fn valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
}

pub fn load(root: &Path) -> ProfileIndex {
    let path = index_path(root);
    match std::fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<ProfileIndex>(&text) {
            Ok(mut index) => {
                // Un indice modificato a mano o scritto da una versione diversa non
                // deve poter puntare fuori da `profiles/` né duplicare gli id.
                let mut seen = std::collections::HashSet::new();
                index.profiles.retain(|entry| {
                    let usable = entry.legacy || valid_slug(&entry.slug);
                    usable && seen.insert(entry.id.clone())
                });
                if !index.profiles.iter().any(|entry| entry.legacy) {
                    index.profiles.insert(0, default_profile());
                }
                index.active = index
                    .profiles
                    .iter()
                    .find(|entry| entry.id == index.active)
                    .map(|entry| entry.id.clone())
                    .unwrap_or_else(|| index.profiles[0].id.clone());
                return index;
            }
            Err(error) => {
                // Non sovrascrivere un indice esistente: se è corrotto lo si
                // conserva, così le cartelle dei profili restano recuperabili.
                let backup = path.with_extension(format!("json.corrupt-{}", stamp()));
                let _ = std::fs::rename(&path, &backup);
                eprintln!(
                    "indice profili non valido ({}): copia in {}",
                    error,
                    backup.display()
                );
            }
        },
        Err(error) => {
            if error.kind() != std::io::ErrorKind::NotFound {
                eprintln!("indice profili non leggibile: {}", error);
            }
        }
    }
    let index = ProfileIndex {
        version: 1,
        active: "default".into(),
        profiles: vec![default_profile()],
    };
    if root.is_dir() {
        let _ = save(root, &index);
    }
    index
}

pub fn save(root: &Path, index: &ProfileIndex) -> Result<(), String> {
    let dir = root.join("profiles");
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let path = index_path(root);
    let temp = path.with_file_name(format!("index.{}.tmp", std::process::id()));
    let text = serde_json::to_string_pretty(index).map_err(|error| error.to_string())?;
    std::fs::write(&temp, text).map_err(|error| error.to_string())?;
    std::fs::rename(&temp, &path).map_err(|error| error.to_string())
}

pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    for character in name.trim().chars() {
        if character.is_ascii_alphanumeric() {
            out.push(character.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

pub fn validate_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(messages::tr(
            "il nome del server non può essere vuoto",
            "server name cannot be empty",
        ));
    }
    if trimmed.chars().count() > 40 {
        return Err(messages::tr(
            "il nome del server è troppo lungo (max 40)",
            "server name is too long (max 40)",
        ));
    }
    if trimmed
        .chars()
        .any(|character| character.is_control() || "[];=".contains(character))
    {
        return Err(messages::tr(
            "il nome del server contiene caratteri non ammessi ([ ] ; =)",
            "server name contains forbidden characters ([ ] ; =)",
        ));
    }
    Ok(trimmed.to_string())
}

pub fn cfg_dir(root: &Path, entry: &ProfileEntry) -> PathBuf {
    if entry.legacy {
        root.join("cfg")
    } else {
        root.join("profiles").join(&entry.slug).join("cfg")
    }
}

pub fn profile_dir(root: &Path, entry: &ProfileEntry) -> PathBuf {
    if entry.legacy {
        root.to_path_buf()
    } else {
        root.join("profiles").join(&entry.slug)
    }
}

fn unique_slug(root: &Path, index: &ProfileIndex, name: &str) -> String {
    let base = {
        let slug = slugify(name);
        if slug.is_empty() {
            format!("server-{}", index.profiles.len() + 1)
        } else {
            slug
        }
    };
    let mut candidate = base.clone();
    let mut counter = 2;
    while candidate == "default"
        || index
            .profiles
            .iter()
            .any(|entry| entry.id == candidate || entry.slug == candidate)
        || root.join("profiles").join(&candidate).exists()
    {
        candidate = format!("{}-{}", base, counter);
        counter += 1;
    }
    candidate
}

pub fn create(
    root: &Path,
    index: &mut ProfileIndex,
    name: &str,
    from: &ProfileEntry,
) -> Result<ProfileEntry, String> {
    make_profile(root, index, name, &cfg_dir(root, from))
}

/// Importa un server esistente: `source_cfg` è la cartella che contiene
/// `server_cfg.ini` (di solito `<cartella>\cfg`). Se il nome è vuoto si usa il
/// campo `[SERVER] NAME` della configurazione importata.
pub fn import_from(
    root: &Path,
    index: &mut ProfileIndex,
    source_cfg: &Path,
    name: &str,
) -> Result<ProfileEntry, String> {
    if !source_cfg.join("server_cfg.ini").is_file() {
        return Err(messages::tr(
            "nella cartella indicata non c'è server_cfg.ini",
            "server_cfg.ini not found in the selected folder",
        ));
    }
    let display = if name.trim().is_empty() {
        server_name_from_cfg(source_cfg).unwrap_or_else(|| "Server importato".to_string())
    } else {
        name.to_string()
    };
    make_profile(root, index, &display, source_cfg)
}

fn server_name_from_cfg(cfg: &Path) -> Option<String> {
    let text = std::fs::read_to_string(cfg.join("server_cfg.ini")).ok()?;
    let doc = crate::ini::parse(&text);
    doc.sections
        .iter()
        .find(|section| section.name.eq_ignore_ascii_case("SERVER"))
        .and_then(|section| {
            section
                .entries
                .iter()
                .find(|entry| entry.key.eq_ignore_ascii_case("NAME"))
        })
        .map(|entry| entry.value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn make_profile(
    root: &Path,
    index: &mut ProfileIndex,
    name: &str,
    source_cfg: &Path,
) -> Result<ProfileEntry, String> {
    let name = validate_name(name)?;
    let slug = unique_slug(root, index, &name);
    let entry = ProfileEntry {
        id: slug.clone(),
        name,
        slug: slug.clone(),
        legacy: false,
        created: now_iso(),
    };
    let destination = profile_dir(root, &entry);
    std::fs::create_dir_all(destination.join("cfg")).map_err(|error| error.to_string())?;
    for file in ["server_cfg.ini", "entry_list.ini", "welcome.txt"] {
        let from_path = source_cfg.join(file);
        if from_path.is_file() {
            std::fs::copy(&from_path, destination.join("cfg").join(file))
                .map_err(|error| error.to_string())?;
        }
    }
    if let Err(error) = ensure_content_link(root, &entry) {
        let _ = std::fs::remove_dir_all(&destination);
        return Err(error);
    }
    index.profiles.push(entry.clone());
    if let Err(error) = save(root, index) {
        index.profiles.pop();
        let _ = std::fs::remove_dir_all(&destination);
        return Err(error);
    }
    Ok(entry)
}

pub fn duplicate(root: &Path, index: &mut ProfileIndex, id: &str) -> Result<ProfileEntry, String> {
    let source = index
        .profiles
        .iter()
        .find(|entry| entry.id == id)
        .cloned()
        .ok_or_else(|| format!("server {} non trovato", id))?;
    let base_name: String = source.name.chars().take(34).collect();
    create(root, index, &format!("{} copia", base_name.trim()), &source)
}

pub fn rename(index: &mut ProfileIndex, id: &str, name: &str) -> Result<ProfileEntry, String> {
    let name = validate_name(name)?;
    let entry = index
        .profiles
        .iter_mut()
        .find(|entry| entry.id == id)
        .ok_or_else(|| format!("server {} non trovato", id))?;
    entry.name = name;
    Ok(entry.clone())
}

pub fn delete(root: &Path, index: &mut ProfileIndex, id: &str) -> Result<(), String> {
    let entry = index
        .profiles
        .iter()
        .find(|entry| entry.id == id)
        .cloned()
        .ok_or_else(|| format!("server {} non trovato", id))?;
    if entry.legacy {
        return Err(messages::tr(
            "il server predefinito non può essere eliminato",
            "the default server cannot be deleted",
        ));
    }
    if index.profiles.len() <= 1 {
        return Err(messages::tr(
            "deve restare almeno un server",
            "at least one server must remain",
        ));
    }
    let source = profile_dir(root, &entry);
    let backups = root.join("backups").join("profiles");
    std::fs::create_dir_all(&backups).map_err(|error| error.to_string())?;
    let destination = backups.join(format!("{}-{}", entry.slug, stamp()));
    if source.is_dir() {
        std::fs::rename(&source, &destination).map_err(|error| error.to_string())?;
    }
    index.profiles.retain(|item| item.id != id);
    if index.active == id {
        index.active = "default".into();
    }
    Ok(())
}

/// Crea/ripara il junction `content` del profilo verso la cartella condivisa.
pub fn ensure_content_link(root: &Path, entry: &ProfileEntry) -> Result<(), String> {
    if entry.legacy {
        return Ok(());
    }
    let link = profile_dir(root, entry).join("content");
    let target = root.join("content");
    if let Ok(existing) = std::fs::read_link(&link) {
        let resolved = if existing.is_absolute() {
            existing
        } else {
            link.parent()
                .map(|dir| dir.join(&existing))
                .unwrap_or(existing)
        };
        let same = match (resolved.canonicalize(), target.canonicalize()) {
            (Ok(left), Ok(right)) => left == right,
            _ => false,
        };
        if same {
            return Ok(());
        }
        let _ = std::fs::remove_dir(&link);
    } else if link.exists() {
        return Err(messages::tr(
            &format!(
                "il server ha una cartella content propria ({}): spostala o creane uno nuovo",
                link.display()
            ),
            &format!(
                "this server has its own content folder ({}): move it or create a new server",
                link.display()
            ),
        ));
    }

    #[cfg(windows)]
    {
        // mklink /J richiede che la destinazione esista: su un'installazione
        // nuova la creiamo vuota e i contenuti si aggiungono dopo.
        if !target.exists() {
            std::fs::create_dir_all(&target).map_err(|error| error.to_string())?;
        }
        // niente cmd.exe: i percorsi non passano da un interprete di comandi
        junction::create(&target, &link).map_err(|error| {
            messages::tr(
                &format!("impossibile collegare i contenuti: {}", error),
                &format!("could not link the content: {}", error),
            )
        })?;
    }
    #[cfg(not(windows))]
    {
        let _ = &link;
        let _ = &target;
        return Err(messages::tr(
            "i profili con contenuti condivisi richiedono Windows",
            "profiles with shared content require Windows",
        ));
    }

    if link.exists() {
        Ok(())
    } else {
        Err(messages::tr(
            "collegamento dei contenuti non creato",
            "content link not created",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("acprof-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn slugifies_display_names() {
        assert_eq!(slugify("Serata GT3!"), "serata-gt3");
        assert_eq!(slugify("  Drift  notte  "), "drift-notte");
        assert_eq!(slugify("###"), "");
    }

    #[test]
    fn rejects_unsafe_names() {
        assert!(validate_name("").is_err());
        assert!(validate_name("a;b").is_err());
        assert!(validate_name("[x]").is_err());
        assert!(validate_name(&"x".repeat(41)).is_err());
        assert_eq!(validate_name("  Serata GT3  ").unwrap(), "Serata GT3");
    }

    #[test]
    fn load_creates_default_profile() {
        let root = temp_root("load");
        let index = load(&root);
        assert_eq!(index.active, "default");
        assert_eq!(index.profiles.len(), 1);
        assert!(index.profiles[0].legacy);
        assert_eq!(cfg_dir(&root, &index.profiles[0]), root.join("cfg"));
        assert!(index_path(&root).is_file());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn index_roundtrip_keeps_active() {
        let root = temp_root("roundtrip");
        let mut index = load(&root);
        index.active = "default".into();
        save(&root, &index).unwrap();
        let again = load(&root);
        assert_eq!(again.active, "default");
        assert_eq!(again.profiles.len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn creates_duplicates_renames_and_deletes() {
        let root = temp_root("crud");
        std::fs::create_dir_all(root.join("cfg")).unwrap();
        std::fs::write(
            root.join("cfg").join("server_cfg.ini"),
            "[SERVER]\nNAME=Base\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("content").join("cars")).unwrap();
        let mut index = load(&root);
        let base = index.profiles[0].clone();

        let created = create(&root, &mut index, "Serata GT3", &base).unwrap();
        assert_eq!(created.slug, "serata-gt3");
        assert!(created.id != "default");
        assert!(!created.legacy);
        assert!(cfg_dir(&root, &created).join("server_cfg.ini").is_file());
        assert!(root
            .join("profiles")
            .join("serata-gt3")
            .join("content")
            .exists());

        let copy = duplicate(&root, &mut index, &created.id).unwrap();
        assert_eq!(copy.name, "Serata GT3 copia");
        assert_ne!(copy.slug, created.slug);

        let renamed = rename(&mut index, &created.id, "Nuovo nome").unwrap();
        assert_eq!(renamed.name, "Nuovo nome");

        delete(&root, &mut index, &created.id).unwrap();
        assert!(!index.profiles.iter().any(|p| p.id == created.id));
        let moved = std::fs::read_dir(root.join("backups").join("profiles"))
            .unwrap()
            .count();
        assert!(moved >= 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn refuses_to_delete_legacy_or_last_profile() {
        let root = temp_root("nodelete");
        let mut index = load(&root);
        assert!(delete(&root, &mut index, "default").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_duplicate_or_dangerous_names() {
        let root = temp_root("names");
        let mut index = load(&root);
        let base = index.profiles[0].clone();
        assert!(create(&root, &mut index, "a;b", &base).is_err());
        let first = create(&root, &mut index, "Stessa", &base).unwrap();
        let second = create(&root, &mut index, "Stessa", &base).unwrap();
        assert_ne!(first.slug, second.slug);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn repairs_missing_content_link() {
        let root = temp_root("junction");
        std::fs::create_dir_all(root.join("content").join("cars")).unwrap();
        let mut index = load(&root);
        let base = index.profiles[0].clone();
        let entry = create(&root, &mut index, "Link", &base).unwrap();
        let link = profile_dir(&root, &entry).join("content");
        std::fs::remove_dir(&link).unwrap();
        assert!(!link.exists());
        ensure_content_link(&root, &entry).unwrap();
        assert!(link.exists());
        assert!(link.join("cars").is_dir());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn name_default_does_not_collide_with_legacy_id() {
        let root = temp_root("default-name");
        std::fs::create_dir_all(root.join("cfg")).unwrap();
        std::fs::create_dir_all(root.join("content")).unwrap();
        let mut index = load(&root);
        let base = index.profiles[0].clone();
        let entry = create(&root, &mut index, "Default", &base).unwrap();
        assert_ne!(entry.id, "default");
        assert!(!entry.legacy);
        assert_eq!(
            index.profiles.iter().filter(|p| p.id == "default").count(),
            1
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn load_drops_entries_with_invalid_slug() {
        let root = temp_root("corrupt");
        std::fs::create_dir_all(root.join("profiles")).unwrap();
        std::fs::write(
            index_path(&root),
            r#"{"version":1,"active":"x","profiles":[{"id":"x","name":"X","slug":"","legacy":false},{"id":"default","name":"Predefinito","slug":"","legacy":true}]}"#,
        )
        .unwrap();
        let index = load(&root);
        assert!(index
            .profiles
            .iter()
            .all(|p| p.legacy || !p.slug.is_empty()));
        assert_eq!(index.profiles.len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn imports_from_existing_folder() {
        let root = temp_root("import");
        std::fs::create_dir_all(root.join("content")).unwrap();
        let src = temp_root("import-src");
        std::fs::create_dir_all(src.join("cfg")).unwrap();
        std::fs::write(
            src.join("cfg").join("server_cfg.ini"),
            "[SERVER]\nNAME=Serata importata\n",
        )
        .unwrap();
        std::fs::write(src.join("cfg").join("entry_list.ini"), "[CAR_0]\nMODEL=x\n").unwrap();
        let mut index = load(&root);
        let entry = import_from(&root, &mut index, &src.join("cfg"), "").unwrap();
        assert_eq!(entry.name, "Serata importata");
        assert!(cfg_dir(&root, &entry).join("server_cfg.ini").is_file());
        assert!(cfg_dir(&root, &entry).join("entry_list.ini").is_file());
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&src);
    }

    #[test]
    fn duplicate_truncates_long_names() {
        let root = temp_root("longdup");
        std::fs::create_dir_all(root.join("content")).unwrap();
        let mut index = load(&root);
        let base = index.profiles[0].clone();
        let long = "A".repeat(40);
        let entry = create(&root, &mut index, &long, &base).unwrap();
        let copy = duplicate(&root, &mut index, &entry.id).unwrap();
        assert!(copy.name.chars().count() <= 40);
        let _ = std::fs::remove_dir_all(&root);
    }
}

#[cfg(all(test, windows))]
mod link_tests {
    use super::*;

    #[test]
    fn creates_and_reuses_content_junction() {
        let root = std::env::temp_dir().join(format!("acsc-junction-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("content").join("cars")).expect("content");
        std::fs::create_dir_all(root.join("profiles").join("test").join("cfg")).expect("profilo");

        let entry = ProfileEntry {
            id: "test".into(),
            name: "Test".into(),
            slug: "test".into(),
            legacy: false,
            created: String::new(),
        };

        ensure_content_link(&root, &entry).expect("junction creata");
        let link = root.join("profiles").join("test").join("content");
        assert!(link.is_dir());
        assert!(link.join("cars").is_dir());

        // seconda chiamata: la riconosce e non la ricrea
        ensure_content_link(&root, &entry).expect("junction gia presente");

        let _ = std::fs::remove_dir(&link);
        let _ = std::fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod corrupt_index_tests {
    use super::*;

    #[test]
    fn corrupt_index_is_preserved_not_overwritten() {
        let root = std::env::temp_dir().join(format!("acsc-corrupt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("profiles")).expect("profiles");
        // un profilo reale sul disco, che l'indice rotto non deve far sparire
        std::fs::create_dir_all(root.join("profiles").join("serata").join("cfg")).expect("profilo");
        std::fs::write(root.join("profiles").join("index.json"), "{ indice rotto").expect("indice");

        let index = load(&root);
        assert!(index.profiles.iter().any(|entry| entry.legacy));

        // il file rotto e' conservato con un nome di recupero
        let names: Vec<String> = std::fs::read_dir(root.join("profiles"))
            .expect("dir")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect();
        assert!(
            names
                .iter()
                .any(|name| name.starts_with("index.json.corrupt-")),
            "{:?}",
            names
        );
        // la cartella del profilo e' ancora li
        assert!(root.join("profiles").join("serata").join("cfg").is_dir());

        let _ = std::fs::remove_dir_all(&root);
    }
}
