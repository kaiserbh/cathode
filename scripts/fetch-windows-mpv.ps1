#!/usr/bin/env pwsh
# Fetches the vendored Windows libmpv + ANGLE runtime into
# src-tauri/vendor/mpv/windows-x64/, which src-tauri/build.rs links and ships.
#
# These binaries used to be committed through Git LFS. LFS bills bandwidth per
# download, so every Windows CI checkout cost ~125 MB; they now live as a GitHub
# Release asset, whose storage and bandwidth are free and unmetered.
#
# The pin lives in scripts/mpv-windows.lock -- bump it there, not here.

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$vendor   = Join-Path $repoRoot 'src-tauri/vendor/mpv/windows-x64'
$lockPath = Join-Path $PSScriptRoot 'mpv-windows.lock'

# mpv.lib satisfies the linker; the three DLLs are copied next to the built
# binaries by build.rs so they resolve at runtime.
$expected = @('libmpv-2.dll', 'libEGL.dll', 'libGLESv2.dll', 'mpv.lib')

if (-not (Test-Path $lockPath)) { throw "Missing lock file: $lockPath" }

$lock = @{}
foreach ($line in Get-Content $lockPath) {
    if ($line -match '^\s*([A-Z0-9_]+)\s*=\s*(.+?)\s*$') { $lock[$Matches[1]] = $Matches[2] }
}
foreach ($key in @('URL', 'SHA256')) {
    if (-not $lock.ContainsKey($key)) { throw "$lockPath is missing $key" }
}

# Already vendored (a CI cache hit, or a repeat local build): do nothing. This is
# what makes the cached CI step free rather than a redundant 47 MB download.
$missing = @($expected | Where-Object { -not (Test-Path (Join-Path $vendor $_)) })
if ($missing.Count -eq 0) {
    Write-Host "Windows libmpv already vendored in $vendor - nothing to fetch."
    exit 0
}

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) "mpv-windows-x64-$PID.zip"
try {
    Write-Host "Fetching $($lock.URL)"
    # The progress bar slows large downloads to a crawl on Windows PowerShell 5.1.
    $prevProgress = $ProgressPreference
    $ProgressPreference = 'SilentlyContinue'
    try { Invoke-WebRequest -Uri $lock.URL -OutFile $tmp -UseBasicParsing }
    finally { $ProgressPreference = $prevProgress }

    # Authenticate before unpacking -- never extract an unverified archive.
    $actual = (Get-FileHash -Path $tmp -Algorithm SHA256).Hash.ToLowerInvariant()
    $want   = $lock.SHA256.ToLowerInvariant()
    if ($actual -ne $want) {
        throw "SHA-256 mismatch for $($lock.URL)`n  expected $want`n  actual   $actual"
    }

    New-Item -ItemType Directory -Force -Path $vendor | Out-Null
    Expand-Archive -Path $tmp -DestinationPath $vendor -Force
}
finally {
    Remove-Item $tmp -Force -ErrorAction SilentlyContinue
}

$stillMissing = @($expected | Where-Object { -not (Test-Path (Join-Path $vendor $_)) })
if ($stillMissing.Count -gt 0) {
    throw "Archive did not contain: $($stillMissing -join ', ')"
}

Write-Host "Vendored Windows libmpv into $vendor"
