# Security Policy

## Supported versions

Only the latest release is supported with security fixes.

| Version | Supported |
| --- | --- |
| 1.x (latest) | ✅ |
| older builds | ❌ |

## Reporting a vulnerability

Please **do not open a public issue** for security problems.

1. Open a private advisory: https://github.com/terzastella/AC-Server-Manager/security/advisories/new
2. Include: what you found, how to reproduce it, the version/commit, and the impact you expect.
3. You will get an answer as soon as possible; a fix and a release follow if the report is valid.

## Scope

In scope:

- The manager (`ACServerControl.exe`) and the launcher (`ACLauncher.exe`)
- The local HTTP booking API (`API_PORT`, default 8082)
- The `acservercontrol://` protocol handler registration
- The build scripts and the release artifacts in this repository

Out of scope:

- `acServer.exe`, Assetto Corsa, Content Manager and any third-party content
- Issues that require physical access to an already-compromised machine
- Denial of service against your own server from your own machine

## Dependency advisories

`app/src-tauri/.cargo/audit.toml` tells `cargo audit` to skip two advisories. Both belong to the same
chain — `glib` and `glib-macros`, pulled in by `gtk` through `tao`/`tauri` — which is compiled on Linux
only, never on Windows:

- `RUSTSEC-2024-0429` (`glib`, unsound `VariantStrIter` iterators): fixed upstream in `glib` 0.20, which
  `tao` 0.37.2 (latest) does not use yet, because `gtk 0.18` requires `glib ^0.18`.
- `RUSTSEC-2024-0370` (`proc-macro-error`, unmaintained): a proc-macro of `glib-macros`, same chain.

Verify with:

```powershell
cd app/src-tauri
cargo tree -i glib --target x86_64-pc-windows-msvc             # nothing to print
cargo tree -i proc-macro-error --target x86_64-pc-windows-msvc # nothing to print
```

Removing the ignore list makes `cargo audit` report exactly these two entries and nothing else.

## Security model (what the app does and does not do)

- **No telemetry, no cloud.** Nothing leaves the machine unless you send an invite link.
- **Local config.** `settings.json` (next to the executable) holds the invite token,
  the API port, the driver name and the saved invites. `cfg/server_cfg.ini` holds the
  admin password. Both stay out of git.
- **Booking API.** Listens on `127.0.0.1` by default: you pick the VPN adapter in the
  Invite page when friends must reach it. Every request needs the invite token, sent
  **only** as an `Authorization: Bearer` header (never in the URL or the body), with a
  per-IP rate limit. It can read the server summary and write into `entry_list.ini`
  only through the booking endpoint, which validates driver, car and skin.
- **Content Manager.** Settings are read only; `Values.data` is never written.
- **Protocol handler.** `acservercontrol://join?...` is registered per user in HKCU
  and only forwards the arguments to the launcher window.
- **No auto-update, no remote code.** The app never downloads or executes code from
  the network.

Hardening tips for hosts:

- Keep the booking API inside your VPN instead of exposing it on the internet.
- Use a non-trivial admin password and rotate the invite token (Invite → Rigenera token)
  when a link has been shared too widely.
- Run the app from a folder writable only by your user account.

---

# Politica di sicurezza (italiano)

## Versioni supportate

Solo l'ultima release riceve correzioni di sicurezza.

## Segnalare una vulnerabilità

**Non aprire una issue pubblica.** Usa l'advisory privato:
https://github.com/terzastella/AC-Server-Manager/security/advisories/new

Indica cosa hai trovato, come riprodurlo, la versione/commit e l'impatto atteso.

## Advisory sulle dipendenze

`app/src-tauri/.cargo/audit.toml` dice a `cargo audit` di saltare due advisory. Appartengono alla stessa
catena — `glib` e `glib-macros`, tirati dentro da `gtk` tramite `tao`/`tauri` — che viene compilata solo
su Linux, mai su Windows:

- `RUSTSEC-2024-0429` (`glib`, iteratori `VariantStrIter` unsound): corretto a monte in `glib` 0.20, che
  `tao` 0.37.2 (l'ultima) non usa ancora, perché `gtk 0.18` richiede `glib ^0.18`.
- `RUSTSEC-2024-0370` (`proc-macro-error`, non mantenuto): proc-macro di `glib-macros`, stessa catena.

Verifica con:

```powershell
cd app/src-tauri
cargo tree -i glib --target x86_64-pc-windows-msvc             # nothing to print
cargo tree -i proc-macro-error --target x86_64-pc-windows-msvc # nothing to print
```

Togliendo la lista di ignore, `cargo audit` segnala esattamente queste due voci e nient'altro.

## Modello di sicurezza in breve

- Nessuna telemetria e nessun servizio cloud: tutto resta sul PC.
- `settings.json` (token d'invito, porta API, nome pilota) e `cfg/server_cfg.ini`
  (password admin) non vengono versionati.
- L'API delle prenotazioni ascolta su `127.0.0.1` per impostazione predefinita:
  l'indirizzo della VPN si sceglie dalla pagina Invito. Richiede il token d'invito
  inviato **solo** nell'header `Authorization: Bearer` (mai nell'URL o nel corpo),
  ha un limite di richieste per IP e scrive in `entry_list.ini` solo tramite
  l'endpoint di prenotazione.
- Content Manager è letto in sola lettura: `Values.data` non viene mai scritto.
- Nessun auto-update e nessun codice scaricato dalla rete.

Consigli per chi ospita: tieni l'API dentro la VPN, usa una password admin non banale,
rigenera il token quando un link è circolato troppo e avvia l'app da una cartella
scrivibile solo dal tuo utente.
