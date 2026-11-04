# Contributing

Thanks for wanting to help. Keep changes small and focused: one problem per pull
request.

## Before you start

- Bug or feature idea? Open an issue first, so we agree on the approach.
- Security problems: follow [SECURITY.md](SECURITY.md), **never** a public issue.

## Development setup

Requirements: Node.js 20+, stable Rust, Visual Studio Build Tools (C++), Windows 10/11.

```powershell
cd app
npm install

# run the app in dev mode
npm run tauri dev
```

## Build and checks

```powershell
# frontend type check
cd app
npx tsc --noEmit

# Rust tests
cd app\src-tauri
cargo test --lib

# portable build -> dist\ACServerControl.exe + dist\ACLauncher.exe
cd app
powershell -NoProfile -ExecutionPolicy Bypass -File .\build.ps1
```

`build.ps1` refuses to overwrite running executables: close the app first.

## What a good change looks like

- Follows the existing structure and naming (`app/src/pages`, `app/src/i18n/namespaces`,
  `app/src-tauri/src/*.rs`).
- **Every user-visible string goes through i18n**: add the key to the namespace file
  with both `it` and `en`; a missing translation must not compile.
- Backend error messages use `messages::tr("italiano", "english")` so both languages
  stay next to each other.
- Does not touch Assetto Corsa data: `.ini` keys and values, car/track/skin names and
  the server log stay as they are.
- Never writes to Content Manager files (`Values.data` is read-only by design).
- No new dependency unless it is really needed.

## Pull request checklist

- [ ] `npx tsc --noEmit` passes
- [ ] `cargo test --lib` passes
- [ ] Checked in both Italian and English
- [ ] No personal data (paths, IPs, tokens, driver names) in code, tests or screenshots
- [ ] Screenshots, if UI changed: use demo data (`GT3 Night` / `Player`)

## License

By contributing you agree that your work is released under the [MIT license](LICENSE).

---

# Contribuire (italiano)

Grazie. Una modifica per pull request, piccola e mirata.

- Prima di iniziare, apri una issue per concordare l'approccio.
- Problemi di sicurezza: segui [SECURITY.md](SECURITY.md), mai una issue pubblica.

```powershell
cd app; npm install; npm run tauri dev     # sviluppo
cd app; npx tsc --noEmit                   # controllo tipi
cd app\src-tauri; cargo test --lib         # test Rust
cd app; .\build.ps1                        # build portable
```

Regole pratiche: ogni testo visibile passa da i18n (chiave in `it` **e** `en`), i
messaggi del backend usano `messages::tr(...)`, non si toccano i dati di Assetto Corsa,
non si scrive nei file di Content Manager, niente dati personali in codice, test o
screenshot.
