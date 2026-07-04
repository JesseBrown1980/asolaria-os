#!/usr/bin/env sh
# Asolaria ASI OS — Linux launcher for the BARE Asolaria-on-metal OS (Windows not required).
# Starts the pure-Rust front-end if it isn't already live on :4600, then opens it full-screen.
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$DIR/asolaria-asi-os-linux"
[ -x "$BIN" ] || BIN="$DIR/target-linux/release/asolaria-asi-os"

# start server if nothing is listening on :4600
if ! (exec 3<>/dev/tcp/127.0.0.1/4600) 2>/dev/null; then
  nohup "$BIN" >/tmp/asolaria-asi-os.log 2>&1 &
fi

# wait for it to answer
i=0; while [ "$i" -lt 20 ]; do
  curl -sf http://127.0.0.1:4600/health >/dev/null 2>&1 && break
  i=$((i+1)); sleep 0.25
done

# open full-screen: chromium/chrome kiosk, else firefox, else xdg-open
for b in chromium-browser chromium google-chrome google-chrome-stable; do
  if command -v "$b" >/dev/null 2>&1; then exec "$b" --app=http://127.0.0.1:4600 --start-fullscreen; fi
done
if command -v firefox >/dev/null 2>&1; then exec firefox --kiosk http://127.0.0.1:4600; fi
xdg-open http://127.0.0.1:4600 2>/dev/null || echo "Asolaria ASI OS live at http://127.0.0.1:4600"
