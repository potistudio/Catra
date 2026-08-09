#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if command -v mise >/dev/null 2>&1; then
  PYTHON_BOOTSTRAP="$(mise exec -- python -c 'import sys; print(sys.executable)')"
else
  PYTHON_BOOTSTRAP="$(command -v python || command -v python3)"
fi

if [ -f ".venv/Scripts/python.exe" ]; then
  PYTHON=".venv/Scripts/python"
elif [ -f ".venv/bin/python" ]; then
  PYTHON=".venv/bin/python"
else
  "$PYTHON_BOOTSTRAP" -m venv .venv
  if [ -f ".venv/Scripts/python.exe" ]; then
    PYTHON=".venv/Scripts/python"
  else
    PYTHON=".venv/bin/python"
  fi
fi

"$PYTHON" -m pip install --upgrade pip
"$PYTHON" -m pip install -e .

echo
echo "Setup complete."
echo "Activate: source .venv/bin/activate  (Windows Git Bash: source .venv/Scripts/activate)"
echo "Test:     $PYTHON -m catra_rekordbox info"
