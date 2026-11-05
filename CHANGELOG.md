# Changelog

Le modifiche importanti del progetto. Formato basato su
[Keep a Changelog](https://keepachangelog.com/it/1.1.0/), versioni
[SemVer](https://semver.org/lang/it/).

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

[1.0.0]: https://github.com/terzastella/AC-Server-Manager/releases/tag/v1.0.0
