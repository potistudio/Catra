# AGENTS.md

## Development

SHOULD format and commit when you make any changes.

### Run

```bash
pnpm tauri dev
```

### Structure

- `apps/desktop` — Tauri + SvelteKit desktop app
- `apps/chrome` — SoundCloud Chrome extension (load unpacked from this directory)

Rekordbox integration is a Rust reimplementation of pyrekordbox (`apps/desktop/src-tauri/src/rekordbox/`). Catra library storage is unchanged (`library.db`).

Build requires OpenSSL for SQLCipher on Windows:

```powershell
winget install ShiningLight.OpenSSL.Light
```

Shining Light installs import libraries under `lib\VC\x64\`, not `lib\` directly. Set:

```powershell
$env:OPENSSL_DIR = "C:\Program Files\OpenSSL-Win64"
$env:OPENSSL_INCLUDE_DIR = "C:\Program Files\OpenSSL-Win64\include"
# pnpm dev (debug):
$env:OPENSSL_LIB_DIR = "C:\Program Files\OpenSSL-Win64\lib\VC\x64\MDd"
# pnpm build (release):
# $env:OPENSSL_LIB_DIR = "C:\Program Files\OpenSSL-Win64\lib\VC\x64\MD"
```

`mise.toml`, `.cargo/config.toml`, and `apps/desktop/.cargo/config.toml` set these for local dev (MDd).

After changing OpenSSL settings, run `cargo clean` in `apps/desktop/src-tauri` once.
