# Full Asolaria fabric NODE = the OS surface + the daemon engines (recall + the 8-byte host),
# running together under YOUR OWN local key. Clones + builds the PUBLIC engines; your recall
# corpus starts EMPTY and is NEVER published. E = 0: nothing fires but what you type or click.
$ErrorActionPreference = 'Stop'
$ROOT = Split-Path $PSScriptRoot -Parent
$FED  = if ($env:ASOLARIA_FED_DIR) { $env:ASOLARIA_FED_DIR } else { Join-Path (Split-Path $ROOT -Parent) 'asolaria-federation-1024' }
foreach ($t in 'cargo','git') { if (-not (Get-Command $t -ErrorAction SilentlyContinue)) { Write-Host "$t needed"; exit 1 } }

Write-Host "== 1/5  mint your local key + seat (kept if already present) =="
& (Join-Path $PSScriptRoot 'keygen.ps1')
$SEAT = Get-Content "$env:USERPROFILE\.asolaria\seat.name"
$PIDV = Get-Content "$env:USERPROFILE\.asolaria\seat.pid"

Write-Host "== 2/5  fetch + build the daemon engines (recall + 8-byte host) =="
if (-not (Test-Path "$FED\.git")) { git clone --depth 1 https://github.com/JesseBrown1980/asolaria-federation-1024 $FED }
Push-Location $FED; cargo build --release -p recall-serve -p asolaria-host8-serve; Pop-Location

Write-Host "== 3/5  build the OS front-end =="
Push-Location $ROOT; cargo build --release; Pop-Location

Write-Host "== 4/5  start the daemons — recall :4796 (YOUR empty corpus) · 8-byte host :5088 =="
$rec = Join-Path $env:USERPROFILE '.asolaria\recall'; New-Item -ItemType Directory -Force -Path $rec | Out-Null
$env:PORT = '4796'; $env:ASOLARIA_RECALL_BIND = '127.0.0.1'
$env:ASOLARIA_RECALL_COLONY = $SEAT; $env:ASOLARIA_RECALL_OWNER_PID = $PIDV
$env:ASOLARIA_RECALL_DIR = $rec; $env:ASOLARIA_RECALL_BASENAME = 'MY-RECALL'
Start-Process -WindowStyle Hidden "$FED\target\release\recall-serve.exe"
Start-Process -WindowStyle Hidden "$FED\target\release\host8-serve.exe" -ArgumentList '--bind','127.0.0.1:5088'
Start-Sleep -Seconds 2

Write-Host "== 5/5  launch the OS -> http://127.0.0.1:4600 (recall + host8 tiles light up) =="
$env:ASOLARIA_SEAT = $SEAT; $env:ASOLARIA_PID = $PIDV
& "$ROOT\target\release\asolaria-asi-os.exe"
