#!/usr/bin/env sh
# One-shot install: build the OS (pure std, no network), mint your local key + seat, and run it.
set -eu
cd "$(CDPATH= cd "$(dirname "$0")/.." && pwd)"
command -v cargo >/dev/null 2>&1 || { echo "Rust toolchain not found — install it: https://rustup.rs" >&2; exit 1; }

echo "== building asolaria-asi-os (pure Rust std, 0 deps — no network needed) =="
cargo build --release

echo "== minting your local secret key + seat (if absent) =="
sh scripts/keygen.sh

echo "== launching Asolaria ASI OS -> http://127.0.0.1:4600 =="
ASOLARIA_SEAT="$(cat "$HOME/.asolaria/seat.name")" \
ASOLARIA_PID="$(cat "$HOME/.asolaria/seat.pid")" \
exec ./target/release/asolaria-asi-os
