#!/usr/bin/env bash
# Build a standalone `headroom` proxy binary with PyInstaller and drop it where
# Tauri expects a sidecar: src-tauri/binaries/headroom-proxy-<target-triple>.
#
# This makes the desktop app self-contained — no system `headroom` install
# required at runtime. Run this before `pnpm tauri build`.
set -euo pipefail

HEADROOM_SPEC="${HEADROOM_SPEC:-headroom-ai}"   # PyPI package (override for a local checkout)
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TRIPLE="$(rustc -vV | awk '/^host:/ {print $2}')"
OUT_DIR="$ROOT/src-tauri/binaries"
BUILD_DIR="$(mktemp -d)"
trap 'rm -rf "$BUILD_DIR"' EXIT

echo "==> target triple: $TRIPLE"
echo "==> building in: $BUILD_DIR"

python3 -m venv "$BUILD_DIR/venv"
# shellcheck disable=SC1091
source "$BUILD_DIR/venv/bin/activate"
pip install --quiet --upgrade pip
pip install --quiet "$HEADROOM_SPEC" pyinstaller

# Entry shim: headroom's console_script main.
cat > "$BUILD_DIR/entry.py" <<'PY'
from headroom.cli import main

if __name__ == "__main__":
    main()
PY

pyinstaller --onefile --name headroom-proxy \
  --distpath "$BUILD_DIR/dist" --workpath "$BUILD_DIR/work" \
  --specpath "$BUILD_DIR" \
  --collect-all headroom \
  "$BUILD_DIR/entry.py"

mkdir -p "$OUT_DIR"
cp "$BUILD_DIR/dist/headroom-proxy" "$OUT_DIR/headroom-proxy-$TRIPLE"
chmod +x "$OUT_DIR/headroom-proxy-$TRIPLE"
echo "==> wrote $OUT_DIR/headroom-proxy-$TRIPLE"
