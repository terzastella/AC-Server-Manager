# Changelog

Le modifiche importanti del progetto. Formato basato su
[Keep a Changelog](https://keepachangelog.com/it/1.1.0/), versioni
[SemVer](https://semver.org/lang/it/).

## [1.0.11] - 2026-11-09

### Sicurezza

- **Caratteri di controllo nei file INI**: `ini::apply` ora restituisce `Result` e rifiuta
  CR/LF (e ogni carattere di controllo) in nomi di sezione, chiavi e valori, invece di
  scriverli nel file. Vale per `server_cfg.ini`, `entry_list.ini`, `welcome.txt`,
  `race.ini` e `assists.ini`: un valore con un a capo non può più iniettare righe o
  chiavi nella sezione successiva. Tutti i chiamanti mostrano un messaggio bilingue.
- **Contenimento dei preset unificato**: `canonical_inside` + `ConfineError` sono usati
  sia dal caricamento dei preset sia dall'import da Content Manager, con errori distinti
  per "non trovato" e "fuori dalla cartella consentita".
- **Link e junction nei preset importati**: `copy_tree` rifiuta collegamenti simbolici e
  junction, quindi un preset non può trascinare dentro `cfg/` file che stanno fuori.
  Aggiunto un test che crea una junction reale su Windows e verifica il rifiuto.

### Manutenzione

- **Supply chain**: `npm audit` e `cargo audit` eseguiti anche nel workflow di release.
- **Action GitHub pinnate al commit SHA** (con il tag in commento), non più solo `@v4`/`@v2`.

### Test

- 7 test nuovi (contenimento preset, junction, iniezione di righe negli INI): il totale
  passa a 88 attivi + 4 ignorati.

## [1.0.10] - 2026-11-09

### Corretto

- **Caricamento dei preset**: `load_preset` costruiva il percorso della cartella con
  il nome ricevuto dall'interfaccia senza controllarlo, quindi `..`, separatori o
  percorsi assoluti potevano puntare fuori da `presets/`. Ora il percorso viene
  canonizzato e deve restare dentro `presets/`, con lo stesso criterio già usato in
  salvataggio. Nessun cambiamento per i preset esistenti.

## [1.0.9] - 2026-11-09

### Modificato

- **L'API delle prenotazioni ascolta solo su questo PC per impostazione predefinita**:
  il bind predefinito passa da `0.0.0.0` a `127.0.0.1`. Per far entrare gli amici si
  sceglie l'indirizzo della VPN dalla pagina Invito (serve riavviare l'app). Le
  impostazioni già salvate restano com'erano.
- **Token solo nell'header**: `Authorization: Bearer`. Rimosso il fallback sul
  parametro `token` della query e sul campo `token` del corpo JSON, che finiva nei log
  dei proxy e nella cronologia. I launcher 1.0.2 e successivi usano già l'header; i
  launcher 1.0.0–1.0.1 non sono più compatibili.

### Corretto

- **Slot senza limite**: con `MAX_CLIENTS` mancante o non valido il tetto veniva
  ignorato e le prenotazioni potevano aggiungere slot all'infinito. Ora la richiesta
  fallisce con un messaggio esplicito.

## [1.0.8] - 2026-11-08

### Corretto

- **Pagina Rete in inglese**: le etichette delle quattro statistiche (IP selezionato,
  porta gioco, porta HTTP, adattatori) erano stringhe scritte a mano in italiano e
  restavano tali anche con l'interfaccia in inglese. Ora passano dalle traduzioni
  `network.stat*`, che esistevano già.

### Manutenzione

- `app/package-lock.json` allineato alla versione del progetto (era rimasto a `0.1.0`).
- `settings.json` aggiunto a `.gitignore`: il pannello lo scrive accanto all'eseguibile
  e contiene token d'invito e IP.
- Documentato in `CONTRIBUTING.md` come attivare l'hook locale `pre-push`
  (`git config core.hooksPath .githooks`).

## [1.0.7] - 2026-11-06

### Corretto

- **Versione della lista slot legata al draft**: il frontend riceve l'impronta
  della lista che ha davanti e la rimanda al salvataggio. Se nel frattempo è
  arrivata una prenotazione il salvataggio viene rifiutato, anche se nel
  frattempo l'interfaccia ha riletto gli slot in background.
- **Salvataggio bloccato con prenotazioni in attesa**: mentre il draft è
  modificato e c'è un aggiornamento dal server, il pulsante Salva è disattivato
  fino a "Scarta e ricarica".
- **Backup**: nome della cartella con i millisecondi e suffisso progressivo in
  caso di collisione, quindi due backup ravvicinati non si sovrascrivono.
- **`welcome.txt`**: anche il suo salvataggio usa il lock condiviso, così il
  preset cattura sempre i tre file coerenti.
- **Processi esterni**: verifica dell'eseguibile e terminazione avvengono sullo
  stesso handle (`OpenProcess` + `QueryFullProcessImageNameW` +
  `TerminateProcess`), eliminando la finestra di riuso del PID.

## [1.0.6] - 2026-11-06

### Corretto

- **Lettura degli slot**: contenuto e impronta della lista vengono letti sotto
  lo stesso lock, quindi una prenotazione non può infilarsi tra le due e far
  sembrare aggiornata una lista vecchia.
- **Prenotazioni**: la configurazione (`CARS`, `MAX_CLIENTS`, pista) viene letta
  e validata sotto lo stesso lock della scrittura: una modifica appena salvata
  non può essere aggirata.
- **Pagina Slot**: se arriva una prenotazione mentre il draft è modificato, il
  draft non viene più sostituito. Compare un avviso con "Scarta e ricarica".
- **Preset**: anche il salvataggio di un preset usa il lock, così cattura i tre
  file nello stesso istante.
- **Lock delle impostazioni**: su Windows è un mutex nominato, con esclusione
  reale e rilascio automatico se il processo termina di colpo (niente più
  scadenza dopo 10 secondi).

## [1.0.5] - 2026-11-06

### Corretto

- **File di configurazione condivisi**: prenotazioni HTTP, salvataggio manuale
  di slot e configurazione, caricamento dei preset usano lo stesso lock, quindi
  non si sovrascrivono a vicenda.
- **Lista slot obsoleta**: se il file è cambiato dopo l'ultima lettura
  dell'interfaccia (per esempio per una prenotazione arrivata nel frattempo),
  il salvataggio viene rifiutato con un messaggio che chiede di ricaricare,
  invece di cancellare la prenotazione.
- **File temporanei**: nome univoco per ogni singola scrittura, non solo per
  processo.
- **`settings.json`**: se il file esiste ma non è leggibile o non è valido, il
  salvataggio si interrompe e il contenuto resta intatto (prima si ripartiva
  dai valori predefiniti).
- **Lock tra processi**: il file di lock contiene un token del proprietario,
  così un processo non cancella il lock di un altro subentrato dopo un timeout.
- **API**: il tetto di 32 connessioni viene applicato prima di creare il
  thread, non dopo.
- **Profili**: se l'indice è corrotto o assente, le cartelle dei profili ancora
  presenti vengono reimportate automaticamente.
- **Release CI**: il workflow dei tag esegue anche i18n, TypeScript, `cargo fmt`,
  Clippy e i test prima di compilare gli eseguibili.

## [1.0.4] - 2026-11-06

### Corretto

- **Manager e Launcher sullo stesso `settings.json`**: prima l'ultimo che
  salvava sovrascriveva i campi dell'altro (il nome pilota poteva sparire).
  Ora ogni processo rilegge il file e aggiorna solo i campi di sua competenza,
  sotto un lock tra processi (`settings.lock`, considerato scaduto dopo 10 s).

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
  del contenuto prima della sostituzione; le scritture sono serializzate.
- Processi esterni: il percorso deve essere assoluto e il nome del file
  esattamente `acServer.exe` (prima bastava il suffisso). Se il percorso non
  è leggibile la terminazione viene rifiutata.
- Porta API: se la porta scelta è occupata si usa la successiva e il link
  d'invito punta a quella davvero aperta, non più a quella configurata.
- Indice dei profili corrotto: non viene più sovrascritto, viene conservato
  come `index.json.corrupt-<data>`.
- `settings.json` non valido: conservato come `settings.json.invalid-<data>`
  invece di essere sostituito in silenzio con i valori predefiniti.
- Contenuti: identificativi con separatori, `..` o unità di disco vengono
  rifiutati prima di costruire i percorsi di copia.
- Junction dei profili creata con l'API di Windows, senza `cmd.exe`.
- API: header malformati o duplicati, `Content-Length` non numerico e
  `Transfer-Encoding` vengono rifiutati.
- Prenotazioni: codici HTTP corretti (400 dati non validi, 409 nessuno slot,
  500 errore interno) al posto di 200 con `ok: false`.
- Rate limiter: gli IP inattivi vengono dimenticati e la mappa ha un tetto.
- README: la descrizione degli slot ora parla di SteamID64 e non più di
  blocco per solo nome.

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

[1.0.11]: https://github.com/terzastella/AC-Server-Manager/releases/tag/v1.0.11
[1.0.10]: https://github.com/terzastella/AC-Server-Manager/releases/tag/v1.0.10
[1.0.9]: https://github.com/terzastella/AC-Server-Manager/releases/tag/v1.0.9
[1.0.8]: https://github.com/terzastella/AC-Server-Manager/releases/tag/v1.0.8
[1.0.7]: https://github.com/terzastella/AC-Server-Manager/releases/tag/v1.0.7
[1.0.6]: https://github.com/terzastella/AC-Server-Manager/releases/tag/v1.0.6
[1.0.5]: https://github.com/terzastella/AC-Server-Manager/releases/tag/v1.0.5
[1.0.4]: https://github.com/terzastella/AC-Server-Manager/releases/tag/v1.0.4
[1.0.3]: https://github.com/terzastella/AC-Server-Manager/releases/tag/v1.0.3
[1.0.2]: https://github.com/terzastella/AC-Server-Manager/releases/tag/v1.0.2
[1.0.1]: https://github.com/terzastella/AC-Server-Manager/releases/tag/v1.0.1
[1.0.0]: https://github.com/terzastella/AC-Server-Manager/releases/tag/v1.0.0
