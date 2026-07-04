# One-shot install: build the OS (pure std, no network), mint your local key + seat, and run it.
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
  Write-Host "Rust toolchain not found — install it: https://rustup.rs"; exit 1
}
Write-Host "== building asolaria-asi-os (pure Rust std, 0 deps) =="
cargo build --release
Write-Host "== minting your local secret key + seat (if absent) =="
& (Join-Path $PSScriptRoot 'keygen.ps1')
$env:ASOLARIA_SEAT = Get-Content "$env:USERPROFILE\.asolaria\seat.name"
$env:ASOLARIA_PID  = Get-Content "$env:USERPROFILE\.asolaria\seat.pid"
Write-Host "== launching Asolaria ASI OS -> http://127.0.0.1:4600 =="
& ".\target\release\asolaria-asi-os.exe"
