#!/usr/bin/env bash
# Build native Linux packages. Run on the target architecture (do not attempt
# macOS-to-Linux cross compilation: WebKitGTK and bundle tooling are native).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if ! command -v pkg-config >/dev/null || ! pkg-config --exists webkit2gtk-4.1 2>/dev/null; then
  cat >&2 <<'EOF'
Missing Linux desktop dependencies. On Ubuntu/Debian install:
  sudo apt install build-essential pkg-config libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
EOF
  exit 1
fi

bun install --cwd ui
bun run build

echo "Packages are in src-tauri/target/release/bundle/{deb,appimage}/"
