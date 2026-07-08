#!/usr/bin/env pwsh
[CmdletBinding()]
param(
    [switch]$SkipQemu,

    [ValidateRange(1, 300)]
    [int]$QemuTimeoutSeconds = 15
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $PSCommandPath
$KernelDir = (Resolve-Path (Join-Path $ScriptDir "..")).Path
$DistDir = Join-Path $KernelDir "dist"
$TargetTriple = "x86_64-unknown-uefi"
$TargetArtifact = Join-Path $KernelDir "target\$TargetTriple\release\asolaria-os.efi"
$DistArtifact = Join-Path $DistDir "asolaria-os-x86_64.efi"
$QemuFatRoot = Join-Path $DistDir "qemu-fat-root"
$script:Failures = 0

function Test-Tool {
    param([Parameter(Mandatory = $true)][string]$Name)
    return $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

function Assert-NotReparsePoint {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Label
    )
    if (Test-Path -LiteralPath $Path) {
        $item = Get-Item -LiteralPath $Path -Force
        if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            Exit-HonestFail "$Label is a reparse point, refusing local artifact write: $Path"
        }
    }
}

function Exit-HonestFail {
    param([Parameter(Mandatory = $true)][string]$Message)
    Write-Host "[honest_fail] $Message"
    exit 2
}

function Add-Failure {
    param(
        [Parameter(Mandatory = $true)][string]$Label,
        [Parameter(Mandatory = $true)][int]$ExitCode
    )
    Write-Host "[fail] $Label exit=$ExitCode"
    $script:Failures += 1
}

function Invoke-Step {
    param(
        [Parameter(Mandatory = $true)][string]$Label,
        [Parameter(Mandatory = $true)][string]$FilePath,
        [string[]]$Arguments = @()
    )

    Write-Host ""
    Write-Host "[run] $Label"
    & $FilePath @Arguments
    $exitCode = if ($null -eq $LASTEXITCODE) { 0 } else { $LASTEXITCODE }
    if ($exitCode -eq 0) {
        Write-Host "[ok] $Label"
        return $true
    }

    Add-Failure -Label $Label -ExitCode $exitCode
    return $false
}

function Copy-DistArtifact {
    if (-not (Test-Path -LiteralPath $TargetArtifact -PathType Leaf)) {
        Add-Failure -Label "artifact missing: $TargetArtifact" -ExitCode 1
        return $false
    }

    try {
        New-Item -ItemType Directory -Force -Path $DistDir | Out-Null
        Assert-NotReparsePoint -Path $DistDir -Label "dist dir"
        Copy-Item -LiteralPath $TargetArtifact -Destination $DistArtifact -Force
        return $true
    } catch {
        Add-Failure -Label "copy artifact to $DistArtifact" -ExitCode 1
        Write-Host "[detail] $($_.Exception.Message)"
        return $false
    }
}

function Write-ArtifactHash {
    param([Parameter(Mandatory = $true)][string]$Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Add-Failure -Label "artifact missing: $Path" -ExitCode 1
        return
    }

    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
    Write-Host "[artifact] $Path"
    Write-Host "[sha256] $hash"
    Write-Host "[sha16]  $($hash.Substring(0, 16))"
}

function Get-OvmfCodePath {
    if ($env:OVMF_CODE) {
        if (Test-Path -LiteralPath $env:OVMF_CODE -PathType Leaf) {
            return (Resolve-Path -LiteralPath $env:OVMF_CODE).Path
        }

        Write-Host "[honest_skip] QEMU smoke skipped: OVMF_CODE is set but does not point to a file: $env:OVMF_CODE"
        return $null
    }

    $candidates = @(
        "C:\Program Files\qemu\share\edk2-x86_64-code.fd",
        "C:\Program Files\qemu\share\OVMF_CODE.fd",
        "C:\Program Files\qemu\edk2-x86_64-code.fd",
        "C:\Program Files\qemu\OVMF_CODE.fd",
        "/usr/share/OVMF/OVMF_CODE.fd",
        "/usr/share/OVMF/OVMF_CODE_4M.fd",
        "/usr/share/ovmf/OVMF.fd",
        "/usr/share/qemu/OVMF.fd",
        "/usr/share/edk2/x64/OVMF_CODE.fd",
        "/usr/share/edk2/ovmf/OVMF_CODE.fd"
    )

    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate -PathType Leaf) {
            return (Resolve-Path -LiteralPath $candidate).Path
        }
    }

    Write-Host "[honest_skip] QEMU smoke skipped: qemu-system-x86_64 is installed, but OVMF_CODE was not found"
    return $null
}

function ConvertTo-ArgumentLine {
    param([Parameter(Mandatory = $true)][string[]]$Arguments)
    return (($Arguments | ForEach-Object {
        if ($_ -match '[\s"]') {
            '"' + ($_ -replace '"', '\"') + '"'
        } else {
            $_
        }
    }) -join " ")
}

function Invoke-QemuSmoke {
    if ($SkipQemu) {
        Write-Host "[skip] QEMU smoke skipped by -SkipQemu"
        return
    }

    $qemu = Get-Command "qemu-system-x86_64" -ErrorAction SilentlyContinue
    if ($null -eq $qemu) {
        Write-Host "[honest_skip] QEMU smoke skipped: qemu-system-x86_64 not found"
        return
    }

    $ovmfCode = Get-OvmfCodePath
    if ($null -eq $ovmfCode) {
        return
    }

    if (-not (Test-Path -LiteralPath $DistArtifact -PathType Leaf)) {
        Write-Host "[honest_skip] QEMU smoke skipped: artifact missing at $DistArtifact"
        return
    }

    Assert-NotReparsePoint -Path $DistDir -Label "dist dir"
    Assert-NotReparsePoint -Path $QemuFatRoot -Label "qemu fat root"
    $bootDir = Join-Path $QemuFatRoot "EFI\BOOT"
    New-Item -ItemType Directory -Force -Path $bootDir | Out-Null
    Assert-NotReparsePoint -Path $QemuFatRoot -Label "qemu fat root"
    Copy-Item -LiteralPath $DistArtifact -Destination (Join-Path $bootDir "BOOTX64.EFI") -Force

    $qemuArgs = @(
        "-machine", "q35",
        "-m", "512",
        "-display", "none",
        "-serial", "none",
        "-monitor", "none",
        "-no-reboot",
        "-drive", "if=pflash,format=raw,readonly=on,file=$ovmfCode",
        # OVMF may touch the boot volume during startup. This is a temporary
        # local FAT directory under kernel/dist, never an ESP or USB device.
        "-drive", "format=raw,file=fat:rw:$QemuFatRoot"
    )

    Write-Host ""
    Write-Host "[run] qemu-system-x86_64 smoke timeout=${QemuTimeoutSeconds}s"
    $startArgs = @{
        FilePath = $qemu.Source
        ArgumentList = (ConvertTo-ArgumentLine -Arguments $qemuArgs)
        PassThru = $true
    }

    $isWindowsHost = [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
        [System.Runtime.InteropServices.OSPlatform]::Windows
    )
    if ($isWindowsHost) {
        $startArgs["WindowStyle"] = "Hidden"
    }

    $proc = Start-Process @startArgs
    try {
        if (-not $proc.WaitForExit($QemuTimeoutSeconds * 1000)) {
            Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
            Write-Host "[ok] QEMU liveness smoke stayed up until timeout; this is not a boot-banner or metal proof"
            return
        }

        if ($proc.ExitCode -eq 0) {
            Write-Host "[ok] QEMU liveness smoke exited cleanly; this is not a boot-banner or metal proof"
        } else {
            Add-Failure -Label "QEMU smoke" -ExitCode $proc.ExitCode
        }
    } finally {
        if (-not $proc.HasExited) {
            Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
        }
    }
}

Write-Host "[suite] Asolaria kernel local build/unit harness (PowerShell)"
Write-Host "[safety] local-only build artifacts; no USB/ESP/BCD writes; no diskpart, bcdedit, mountvol, Format-Volume, mkfs, or dd calls"
Write-Host "[root] $KernelDir"

if (-not (Test-Tool "cargo")) {
    Exit-HonestFail "cargo not found; install rustup/cargo. The kernel workspace selects Rust 1.81 from rust-toolchain.toml."
}
if (-not (Test-Tool "rustc")) {
    Exit-HonestFail "rustc not found; install Rust 1.81 through rustup."
}
if (-not (Test-Tool "rustfmt")) {
    Exit-HonestFail "rustfmt not found; install the Rust 1.81 rustfmt component."
}

Push-Location $KernelDir
try {
    $rustcVersion = (& rustc --version 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) {
        Exit-HonestFail "rustc --version failed"
    }
    Write-Host "[tool] $rustcVersion"
    if ($rustcVersion -notmatch '^rustc 1\.81\.') {
        Exit-HonestFail "kernel rust-toolchain.toml must resolve to Rust 1.81; observed '$rustcVersion'"
    }

    $cargoVersion = (& cargo --version 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) {
        Exit-HonestFail "cargo --version failed"
    }
    Write-Host "[tool] $cargoVersion"

    if (Test-Tool "rustup") {
        $installedTargets = & rustup target list --toolchain 1.81 --installed 2>$null
        if (($LASTEXITCODE -eq 0) -and ($installedTargets -notcontains $TargetTriple)) {
            Exit-HonestFail "rustup target '$TargetTriple' is not installed for toolchain 1.81. Run: rustup target add $TargetTriple --toolchain 1.81"
        }
    }

    [void](Invoke-Step -Label "cargo fmt --all --check" -FilePath "cargo" -Arguments @("fmt", "--all", "--check"))
    [void](Invoke-Step -Label "cargo check -p asolaria-kernel-core --all-targets" -FilePath "cargo" -Arguments @("check", "-p", "asolaria-kernel-core", "--all-targets"))
    [void](Invoke-Step -Label "cargo test -p asolaria-kernel-core --all-targets -- --test-threads=1" -FilePath "cargo" -Arguments @("test", "-p", "asolaria-kernel-core", "--all-targets", "--", "--test-threads=1"))
    $buildOk = Invoke-Step -Label "cargo build --release --target $TargetTriple --bin asolaria-os" -FilePath "cargo" -Arguments @("build", "--release", "--target", $TargetTriple, "--bin", "asolaria-os")

    if ($buildOk) {
        $copyOk = Copy-DistArtifact
        Write-ArtifactHash -Path $TargetArtifact
        if ($copyOk) {
            Write-ArtifactHash -Path $DistArtifact
        } else {
            Write-Host "[honest_skip] dist artifact hash skipped because artifact copy failed"
        }
        Invoke-QemuSmoke
    } else {
        Write-Host "[honest_skip] artifact hash skipped: UEFI build failed, avoiding stale artifact claim"
        Write-Host "[honest_skip] QEMU smoke skipped: UEFI build failed"
    }
} finally {
    Pop-Location
}

if ($script:Failures -eq 0) {
    Write-Host ""
    Write-Host "[done] local build/unit suite passed; skipped or passing QEMU smoke is not a system-test proof"
    exit 0
}

Write-Host ""
Write-Host "[done] suite failed failures=$script:Failures"
exit 3
