#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

if [ -f "$ROOT/.venv/Scripts/python.exe" ]; then
  PYTHON="$ROOT/.venv/Scripts/python.exe"
elif [ -f "$ROOT/.venv/bin/python" ]; then
  PYTHON="$ROOT/.venv/bin/python"
else
  echo "Rekordbox Python env not found. Run: pnpm setup:rekordbox" >&2
  exit 1
fi

exec "$PYTHON" -m catra_rekordbox "$@"
