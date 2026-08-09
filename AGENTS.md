# AGENTS.md

## Development

MUST Commit when you make any changes.

### Run

```bash
mise install
pnpm install
pnpm dev
```

### Structure

- `apps/desktop` — Tauri + SvelteKit desktop app
- `apps/chrome` — SoundCloud Chrome extension (load unpacked from this directory)

Rekordbox integration is a Rust reimplementation of pyrekordbox (`apps/desktop/src-tauri/src/rekordbox/`). Catra library storage is unchanged (`library.db`).

Build requires OpenSSL for SQLCipher on Windows:

```powershell
winget install ShiningLight.OpenSSL.Light
$env:OPENSSL_DIR = "C:\Program Files\OpenSSL-Win64"
```
