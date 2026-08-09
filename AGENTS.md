# AGENTS.md

## Development

MUST Commit when you make any changes.

### Run

```bash
mise install
pnpm install
pnpm dev
```

### Rekordbox (pyrekordbox)

```bash
pnpm setup:rekordbox          # create venv + install pyrekordbox
pnpm rekordbox info           # version check
pnpm rekordbox check          # Rekordbox install paths
pnpm rekordbox db-status      # master.db track/playlist counts (JSON)
```

Python package lives in `apps/desktop/python/`.

### Structure

- `apps/desktop` — Tauri + SvelteKit desktop app
- `apps/desktop/python` — pyrekordbox wrapper CLI for Rekordbox integration
- `apps/chrome` — SoundCloud Chrome extension (load unpacked from this directory)
