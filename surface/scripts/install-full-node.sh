#!/usr/bin/env sh
# Full Asolaria fabric NODE = the OS surface + the daemon engines (recall + the 8-byte host),
# running together under YOUR OWN local key. This clones + builds the PUBLIC engines from
# asolaria-federation-1024; your recall corpus starts EMPTY and is NEVER published.
# E = 0: nothing fires but what you type or click.
set -eu
ROOT="$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
FED="${ASOLARIA_FED_DIR:-$(dirname "$ROOT")/asolaria-federation-1024}"
command -v cargo >/dev/null 2>&1 || { echo "Rust needed — https://rustup.rs" >&2; exit 1; }
command -v git   >/dev/null 2>&1 || { echo "git needed" >&2; exit 1; }

echo "== 1/5  mint your local key + seat (kept if already present) =="
sh "$ROOT/scripts/keygen.sh"
SEAT="$(cat "$HOME/.asolaria/seat.name")"
PIDV="$(cat "$HOME/.asolaria/seat.pid")"

echo "== 2/5  fetch + build the daemon engines (recall + 8-byte host) =="
[ -d "$FED/.git" ] || git clone --depth 1 https://github.com/JesseBrown1980/asolaria-federation-1024 "$FED"
( cd "$FED" && cargo build --release -p recall-serve -p asolaria-host8-serve )

echo "== 3/5  build the OS front-end =="
( cd "$ROOT" && cargo build --release )

echo "== 4/5  start the daemons — recall :4796 (YOUR empty corpus) · 8-byte host :5088 =="
mkdir -p "$HOME/.asolaria/recall"          # your corpus dir — starts empty, never published
PORT=4796 ASOLARIA_RECALL_BIND=127.0.0.1 \
  ASOLARIA_RECALL_COLONY="$SEAT" ASOLARIA_RECALL_OWNER_PID="$PIDV" \
  ASOLARIA_RECALL_DIR="$HOME/.asolaria/recall" ASOLARIA_RECALL_BASENAME="MY-RECALL" \
  nohup "$FED/target/release/recall-serve" > "$HOME/.asolaria/recall.log" 2>&1 &
nohup "$FED/target/release/host8-serve" --bind 127.0.0.1:5088 > "$HOME/.asolaria/host8.log" 2>&1 &
sleep 2

echo "== 5/5  launch the OS -> http://127.0.0.1:4600 (recall + host8 tiles light up) =="
ASOLARIA_SEAT="$SEAT" ASOLARIA_PID="$PIDV" exec "$ROOT/target/release/asolaria-asi-os"
