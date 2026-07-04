#!/usr/bin/env sh
# Mint THIS node's local secret key + seat identity. Everything stays on YOUR machine.
#
# The key is an HMAC secret for recall / Hilbra (key-off-the-wire): it is NEVER transmitted,
# NEVER printed to a server, and NEVER committed to git (see .gitignore). Level-0 recall is
# public + provably PII-free; everything above level 0 requires this key, which only you hold.
# Idempotent: existing files are kept, never overwritten.
set -eu
D="${HOME}/.asolaria"
mkdir -p "$D"
chmod 700 "$D" 2>/dev/null || true

if [ -f "$D/recall.key" ]; then
  echo "recall key already present: $D/recall.key (kept)"
else
  [ -r /dev/urandom ] || { echo "ERROR: /dev/urandom unavailable — cannot mint a key safely" >&2; exit 1; }
  head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n' > "$D/recall.key"   # 32 random bytes -> 64 hex
  chmod 600 "$D/recall.key" 2>/dev/null || true
  echo "minted local secret key -> $D/recall.key  (KEEP PRIVATE — never on the wire, never on GitHub)"
fi

if [ ! -f "$D/seat.pid" ]; then
  PID=$(head -c 8 /dev/urandom | od -An -tx1 | tr -d ' \n')
  printf '%s' "$PID" > "$D/seat.pid"
  printf 'ASOLARIA-NODE-%s' "$(printf '%s' "$PID" | cut -c1-6)" > "$D/seat.name"
  echo "minted local seat  -> $(cat "$D/seat.name")  pid $PID"
else
  echo "seat already present: $(cat "$D/seat.name" 2>/dev/null || echo '?') (kept)"
fi
