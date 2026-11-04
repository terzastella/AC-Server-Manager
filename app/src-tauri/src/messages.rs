//! Messaggi dell'interfaccia generati dal backend.
//!
//! La lingua è globale (un solo processo, un solo utente) e viene impostata
//! all'avvio da `settings.json` e aggiornata dal comando `set_language`.
//! Ogni messaggio porta con sé la versione italiana e quella inglese, così le
//! due restano affiancate e non possono divergere.

use std::sync::atomic::{AtomicBool, Ordering};

static ENGLISH: AtomicBool = AtomicBool::new(false);

pub fn set(lang: &str) {
    ENGLISH.store(lang.eq_ignore_ascii_case("en"), Ordering::Relaxed);
}

pub fn english() -> bool {
    ENGLISH.load(Ordering::Relaxed)
}

/// Restituisce il messaggio nella lingua attiva.
pub fn tr(it: &str, en: &str) -> String {
    if english() {
        en.to_string()
    } else {
        it.to_string()
    }
}
