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

## Security model (what the app does and does not do)

- **No telemetry, no cloud.** Nothing leaves the machine unless you send an invite link.
- **Local config.** `settings.json` (next to the executable) holds the invite token,
  the API port, the driver name and the saved invites. `cfg/server_cfg.ini` holds the
  admin password. Both stay out of git.
- **Booking API.** Listens on `0.0.0.0` so friends over a VPN can reach it; every
  request needs the token that is part of the invite link, with a per-IP rate limit.
  It can read the server summary and write into `entry_list.ini` only through the
  booking endpoint, which validates driver, car and skin.
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

## Modello di sicurezza in breve

- Nessuna telemetria e nessun servizio cloud: tutto resta sul PC.
- `settings.json` (token d'invito, porta API, nome pilota) e `cfg/server_cfg.ini`
  (password admin) non vengono versionati.
- L'API delle prenotazioni richiede il token contenuto nel link d'invito, ha un limite
  di richieste per IP e scrive in `entry_list.ini` solo tramite l'endpoint di prenotazione.
- Content Manager è letto in sola lettura: `Values.data` non viene mai scritto.
- Nessun auto-update e nessun codice scaricato dalla rete.

Consigli per chi ospita: tieni l'API dentro la VPN, usa una password admin non banale,
rigenera il token quando un link è circolato troppo e avvia l'app da una cartella
scrivibile solo dal tuo utente.
