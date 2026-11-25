#!/usr/bin/env bash
set -euo pipefail

BIN="./target/debug/clipboard-app"
BUNDLE_ID="dev.sourcec0de.clipboard"

cargo build -p clipboard-app

if [[ "$(uname -s)" == "Darwin" ]]; then
  IDENTITY=$(security find-identity -v -p codesigning 2>/dev/null | awk -F'"' '/Apple Development|Developer ID Application/{print $2; exit}')
  if [[ -n "${IDENTITY:-}" ]]; then
    codesign --force --sign "$IDENTITY" --identifier "$BUNDLE_ID" "$BIN"
  fi
fi

pkill -f clipboard-app || true
sleep 0.2
nohup "$BIN" >/tmp/clipboard.log 2>&1 </dev/null &
sleep 0.5
pgrep -fl clipboard-app
