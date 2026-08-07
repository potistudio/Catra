#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <chrome-extension-id>" >&2
  exit 1
fi

EXTENSION_ID="$1"
HOST_DIR="$(cd "$(dirname "$0")" && pwd)"
LAUNCHER_PATH="$HOST_DIR/ytdlp-host.sh"
MANIFEST_PATH="$HOST_DIR/com.catra.ytdlp.json"

chmod +x "$HOST_DIR/ytdlp-host.sh"

cat >"$MANIFEST_PATH" <<EOF
{
  "name": "com.catra.ytdlp",
  "description": "Catra yt-dlp native messaging host",
  "path": "$LAUNCHER_PATH",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://$EXTENSION_ID/"
  ]
}
EOF

if [[ "$OSTYPE" == "darwin"* ]]; then
  TARGET_DIR="$HOME/Library/Application Support/Google/Chrome/NativeMessagingHosts"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
  TARGET_DIR="$HOME/.config/google-chrome/NativeMessagingHosts"
else
  echo "Unsupported OS for install.sh. Use install.ps1 on Windows." >&2
  exit 1
fi

mkdir -p "$TARGET_DIR"
cp "$MANIFEST_PATH" "$TARGET_DIR/com.catra.ytdlp.json"

echo "Installed native messaging host to $TARGET_DIR/com.catra.ytdlp.json"
