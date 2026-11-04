<p align="center">
  <img src="docs/brand/manager.png" width="92" alt="AC Server Control">
  &nbsp;&nbsp;&nbsp;
  <img src="docs/brand/launcher.png" width="92" alt="AC Launcher">
</p>

<h1 align="center">AC Server Control</h1>

<p align="center">
  <a href="README.md">← Index</a> · <a href="README-IT.md">Italiano</a> · <b>English</b>
</p>

<p align="center">
  Portable manager for a Windows-hosted Assetto Corsa dedicated server,<br>
  with a launcher that drops your friends straight into the race.<br>
  <sub>Tauri 2 · React + TypeScript · Rust · IT/EN interface</sub>
</p>

---

## What it is

Two executables, no installation:

| Executable | What it does |
|---|---|
| **ACServerControl.exe** | Runs the server: start/stop, `server_cfg.ini`, slots and skins, content, ports, invites, multiple server profiles, presets and backups, live logs |
| **ACLauncher.exe** | Sent to friends: they paste the invite link, pick car and skin, book their slot and **go straight into the game** (writes `race.ini` and starts `acs.exe`, no Content Manager needed) |

Everything lives next to the server itself: configuration, content and logs stay in
plain files (`cfg/*.ini`, `content/`, `logs/`) — no database, no external service.

**Main features**

- Start, stop and restart the server with live logs and real-time status
- `server_cfg.ini` and `entry_list.ini` editor that preserves order and comments, with an explanation for every field
- Slots with car, skin and driver; soft booking by driver name
- Content browser (cars, tracks, weather) with previews and copy from the game to the server
- **Read-only** access to Content Manager settings, import of server presets
- Network: detected IPs, `acmanager://` links, QR code, port checks, firewall rules
- Branded invite page, token and HTTP booking API
- Multiple server profiles sharing the same content (one `content/` folder)
- Presets, automatic backups, logs of previous sessions
- **Italian / English** interface, switchable on the fly

## Screenshots

| Content | Slots & cars |
|---|---|
| ![Content](docs/img/manager-content.png) | ![Slots](docs/img/manager-slots.png) |

| Configuration | Launcher |
|---|---|
| ![Configuration](docs/img/manager-config.png) | ![Launcher](docs/img/launcher-home.png) |

## How it works

```text
+- HOST PC ------------------------------------------------------+
|                                                                |
|  +---------------------+              +---------------------+  |
|  | ACServerControl.exe |              | acServer.exe        |  |
|  | manager + API       | <------->    | UDP/TCP 9600        |  |
|  | HTTP : 8082         |              | HTTP   8081         |  |
|  +---------------------+              +---------------------+  |
|             |                                    ^             |
|             v content/ . cfg/ . logs/            |             |
|                                                  |             |
|                                                  |             |
+--------------------------------------------------|-------------+
+- FRIENDS' PC --------------------------------------------------+
|                                                  |             |
|  +---------------------+   invite link + token   | race        |
|  | ACLauncher.exe      |   GET  /api/server      |             |
|  | opens the link      |   POST /api/reserve     |             |
|  | books the slot      |                         |             |
|  +---------------------+                         |             |
|             v                                    |             |
|     +-------------+                              |             |
|     | acs.exe     +------------------------------+             |
|     +-------------+                                            |
|                                                                |
+----------------------------------------------------------------+
```

Step by step:

1. The host opens **ACServerControl.exe** and starts `acServer.exe` (`UDP/TCP 9600`, `HTTP 8081`).
2. The manager builds a link: `acservercontrol://join?ip=…&httpPort=8081&apiPort=8082&token=…`.
3. A friend opens it: **ACLauncher.exe** calls `GET /api/server` and gets name, track, cars and skins.
4. They pick car, skin and name → `POST /api/reserve` → the slot is written into `entry_list.ini`.
5. The launcher writes `race.ini` (the `[REMOTE]` section) and starts `acs.exe`.
6. Assetto Corsa joins the server, in the booked slot.

The manager talks to the server on `HTTP_PORT` (default 8081) and exposes a small
API for the launcher on `API_PORT` (default 8082), protected by a token with
per-IP rate limiting.

## Requirements

- Windows 10/11 x64 with **WebView2 Runtime** (bundled with Windows 11)
- **Assetto Corsa** installed from Steam, with the dedicated server files
  (`acServer.exe` and `content/`)
- *Optional*: Content Manager, read-only, to import settings and presets
- To play over the internet: Hamachi, Radmin VPN, ZeroTier, Tailscale or port forwarding

## Build from source

```powershell
# dependencies
cd app
npm install

# portable build (produces dist\ACServerControl.exe and dist\ACLauncher.exe)
powershell -NoProfile -ExecutionPolicy Bypass -File .\build.ps1
```

You need Node.js 20+, a stable Rust toolchain and Visual Studio Build Tools (C++).

Brand assets are regenerated with `python docs\brand\generate-assets.py` (icons)
and `python docs\brand\cutout.py` (background-free mark for the interface).

## Layout

```
app/                     Tauri: React frontend (src/) and Rust backend (src-tauri/src/)
  src/i18n/              IT/EN dictionaries, one file per feature area
  src/pages/             Dashboard, Configuration, Slots, Content, CM, Network, Invite...
  src-tauri/src/         commands, HTTP API, ini, profiles, slots, content, messages
docs/brand/              logos (PNG/ICO) and the scripts that generate them
docs/img/                screenshots used by the READMEs
```

## Internationalisation

The interface is available in **Italian** and **English**: switch it in the manager's
Settings or with the `IT / EN` selector in the launcher. The choice applies to both
(and is stored in `settings.json`).

Translations live in `app/src/i18n/namespaces/*.ts`: every file exposes `it` and `en`
with the same keys, so a missing translation is a TypeScript compile error. Messages
produced by the Rust backend are bilingual in `app/src-tauri/src/messages.rs`.

Assetto Corsa data (car, track and skin names), `.ini` keys and the server log
content are intentionally not translated.

## What is not in the repository

The repo contains sources, documentation and images. It does not contain game
binaries, content or personal configuration.

| Excluded | Why |
|---|---|
| `acServer.exe`, `acServerManager.exe` | Assetto Corsa binaries, not redistributable |
| `content/`, `system/` | cars, tracks and server DLLs |
| `cfg/`, `profiles/`, `presets/`, `setups/`, `backups/`, `logs/` | local data (admin password, driver names) |
| `dist/`, `node_modules/`, `target/` | builds and dependencies |

After cloning you still need `acServer.exe` and the content from your own Assetto
Corsa installation; everything else is created by the app. Prebuilt executables are
attached to the **v1.0.0** release.

## Privacy

- No telemetry, no cloud service: everything runs locally.
- The launcher API listens on the network to accept bookings, is token-protected and
  only accepts requests from the expected addresses.
- Admin credentials and invite tokens live in `settings.json` and
  `cfg/server_cfg.ini`, which are not versioned.

---

<p align="center">
  <sub>Personal project, not affiliated with Kunos Simulazioni or Assetto Corsa.</sub>
</p>
