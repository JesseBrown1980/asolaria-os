# Mint THIS node's local secret key + seat identity. Everything stays on YOUR machine.
#
# The key is an HMAC secret for recall / Hilbra (key-off-the-wire): NEVER transmitted, NEVER
# committed to git. Level-0 recall is public + provably PII-free; everything above needs this key,
# which only you hold. Idempotent: existing files are kept, never overwritten.
$ErrorActionPreference = 'Stop'
$D = Join-Path $env:USERPROFILE '.asolaria'
New-Item -ItemType Directory -Force -Path $D | Out-Null

if (Test-Path "$D\recall.key") {
  Write-Host "recall key already present: $D\recall.key (kept)"
} else {
  $b = New-Object byte[] 32
  [System.Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($b)
  (($b | ForEach-Object { $_.ToString('x2') }) -join '') | Set-Content -NoNewline -Path "$D\recall.key"
  Write-Host "minted local secret key -> $D\recall.key  (KEEP PRIVATE — never on the wire, never on GitHub)"
}

if (-not (Test-Path "$D\seat.pid")) {
  $p = New-Object byte[] 8
  [System.Security.Cryptography.RandomNumberGenerator]::Create().GetBytes($p)
  $pid16 = ($p | ForEach-Object { $_.ToString('x2') }) -join ''
  $pid16 | Set-Content -NoNewline -Path "$D\seat.pid"
  "ASOLARIA-NODE-$($pid16.Substring(0,6))" | Set-Content -NoNewline -Path "$D\seat.name"
  Write-Host "minted local seat  -> ASOLARIA-NODE-$($pid16.Substring(0,6))  pid $pid16"
} else {
  Write-Host "seat already present (kept)"
}
