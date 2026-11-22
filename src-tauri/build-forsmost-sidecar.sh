#!/usr/bin/env bash
# Fetch or build the `forsmost` CLI sidecar and place it where Tauri's
# `externalBin` expects it: src-tauri/bin/forsmost-$TARGET_TRIPLE.
#
# Tauri v2 bundles this binary with the app, so Office carving (docx, pptx,
# xlsx) works with no manual install. Run this before `tauri build` or let
# CI (.github/workflows/release.yml) do it.
set -euo pipefail

cd "$(dirname "$0")"

TRIPLE="${1:-$(rustc -vV | awk '/^host:/ {print $2}')}"
OUT="bin/forsmost-${TRIPLE}"

if [ -x "$OUT" ]; then
  echo "sidecar already present: $OUT"
  exit 0
fi

mkdir -p bin
echo "building forsmost sidecar for ${TRIPLE}..."
cargo install --locked --root "$(pwd)/.sidecar" forsmost
cp ".sidecar/bin/forsmost" "$OUT"
chmod +x "$OUT"
echo "sidecar ready: $OUT"
