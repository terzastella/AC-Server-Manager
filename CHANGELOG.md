# Changelog

Le modifiche importanti del progetto. Formato basato su
[Keep a Changelog](https://keepachangelog.com/it/1.1.0/), versioni
[SemVer](https://semver.org/lang/it/).

## [1.0.3] - 2026-11-06

### Sicurezza

- Confronto del token a tempo costante: il tempo di risposta non dipende da
  quanti caratteri combaciano (niente più `==` sulle stringhe).
- API su tutte le interfacce: la pagina Invito mostra un avviso esplicito
  quando `api_bind` è `0.0.0.0`, così la scelta è consapevole.

### Corretto

- `state.persist()` restituisce `Result`: se `settings.json` non si scrive,
  il comando fallisce e l'interfaccia lo mostra, invece di dare per salvato
  ciò che è rimasto solo in memoria.
- Impostazioni scritte in modo atomico con file temporaneo univoco e verifica
  del contenuto prima della sostituzione.
- Processi esterni: il percorso deve essere assoluto e il nome del file
  esattamente `acServer.exe` (prima bastava il suffisso).

## [1.0.2] - 2026-11-06

Correzioni emerse da una seconda revisione esterna.

### Corretto

- `entry_list.ini`: un errore di lettura non viene più scambiato per "file
  vuoto" (solo un file assente vale come vuoto), quindi non si sovrascrive
  più la lista slot quando il file è bloccato o illeggibile.
- Slot: il GUID identifica il giocatore. Due omonimi con SteamID diversi non
  finiscono più sullo stesso slot; uno slot prenotato solo per nome viene
  adottato da chi arriva con quel nome.
- Processi esterni: se ne termina uno solo, quello scelto, dopo aver
  verificato che il PID sia ancora `acServer.exe`.
- Token d'invito: se il CSPRNG fallisce l'errore arriva in interfaccia invece
  di salvare un invito con token vuoto.
- IPv6: gli indirizzi sono racchiusi tra parentesi quadre negli URL dell'API.

### Sicurezza

- Il launcher invia il token nell'header `Authorization: Bearer` e non più
  nella query string o nel corpo JSON. Il server accetta ancora i vecchi
  metodi per i launcher 1.0.0.
- Un `api_bind` non valido non ricade più su `0.0.0.0`: l'API non parte e
  l'errore finisce nel log.

### Tecnico

- File temporaneo di scrittura univoco per processo (`<file>.<pid>.tmp`).

## [1.0.1] - 2026-11-05

Release di sicurezza e integrità: nessuna funzione nuova.

### Sicurezza

- Token d'invito da CSPRNG (32 byte, base64url) al posto di tempo e PID.
- API: token nell'header `Authorization: Bearer`, scadenza opzionale, interfaccia
  di ascolto selezionabile, timeout di I/O, limiti su header e connessioni.
- CSP di Tauri attiva (`default-src 'self'`, nessuna origine esterna).
- Validazione della livrea rispetto all'auto e dello SteamID64.

### Corretto

- `entry_list.ini`: il salvataggio preserva commenti, ordine e chiavi sconosciute.
- Backup obbligatorio prima di ogni salvataggio e scritture atomiche.
- Lo stesso SteamID riusa il proprio slot invece di crearne un altro.
- I processi esterni vengono mostrati con PID e percorso prima di terminarli.

### CI

- `cargo clippy --all-targets --all-features -- -D warnings`.

## [1.0.0] - 2026-11-04

Prima release pubblica.

### Aggiunto

- Manager portable (`ACServerControl.exe`): avvio, stop e riavvio del server,
  log live, stato e porte in tempo reale.
- Editor di `server_cfg.ini` ed `entry_list.ini` che preserva ordine e commenti,
  con spiegazione per ogni campo.
- Slot con auto, livrea e pilota; prenotazione per nome guidatore.
- Browser contenuti con anteprime e copia dal gioco al server.
- Lettura in sola lettura delle impostazioni di Content Manager e import dei preset.
- Rete: IP rilevati, link `acmanager://`, QR code, controllo porte, regole firewall.
- Invito con grafica personalizzata, token e API HTTP di prenotazione.
- Profili multipli con contenuti condivisi.
- Preset, backup automatici e log delle sessioni precedenti.
- Launcher (`ACLauncher.exe`): link d'invito, scelta auto/livrea, prenotazione slot
  e ingresso diretto in partita tramite `race.ini` (`[REMOTE]`).
- Interfaccia italiana e inglese, commutabile a caldo.

### Build

- I binari non contengono più i percorsi assoluti della macchina di compilazione:
  `build.ps1` riscrive il profilo utente in `/build` (`--remap-path-prefix`).

[1.0.3]: https://github.com/terzastella/AC-Server-Manager/releases/tag/v1.0.3
[1.0.2]: https://github.com/terzastella/AC-Server-Manager/releases/tag/v1.0.2
[1.0.1]: https://github.com/terzastella/AC-Server-Manager/releases/tag/v1.0.1
[1.0.0]: https://github.com/terzastella/AC-Server-Manager/releases/tag/v1.0.0
