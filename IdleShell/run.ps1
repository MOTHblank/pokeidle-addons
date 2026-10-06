<#
.SYNOPSIS
  Build and run (or publish) IdleShell.

.EXAMPLES
  .\run.ps1                # build Release and launch
  .\run.ps1 -NoRun         # build only
  .\run.ps1 -Clean         # wipe bin/obj/out first
  .\run.ps1 -Publish       # single-file win-x64 build into .\publish
  .\run.ps1 -DebugBuild    # Debug config
#>
[CmdletBinding()]
param(
    [switch]$Clean,
    [switch]$NoRun,
    [switch]$Publish,
    [switch]$DebugBuild
)

$ErrorActionPreference = "Stop"
$project = Join-Path $PSScriptRoot "IdleShell.csproj"
$config  = if ($DebugBuild) { "Debug" } else { "Release" }
$outDir  = Join-Path $PSScriptRoot "bin\$config\net10.0-windows"
$pubDir  = Join-Path $PSScriptRoot "publish"

# The shell runs the real upstream Violentmonkey extension, not a clone.
# Remove the old pre-vendor layout so a stale unpacked extension can never be
# picked up by a development run. The current layout is IdleShell/vendor/violentmonkey.
$legacyVmDir = Join-Path $PSScriptRoot "violentmonkey"
if (Test-Path $legacyVmDir) {
    Write-Host "Removing legacy Violentmonkey directory: $legacyVmDir"
    Remove-Item $legacyVmDir -Recurse -Force
}
Write-Host "Preparing official Violentmonkey 2.49.0 MV3..."
& (Join-Path $PSScriptRoot "prepare-violentmonkey.ps1")
if ($LASTEXITCODE) { throw "Violentmonkey preparation failed" }

# --- prerequisites -----------------------------------------------------------
if (-not (Get-Command dotnet -ErrorAction SilentlyContinue)) {
    throw ".NET SDK not found. Install the .NET 10 SDK: https://dotnet.microsoft.com/download"
}

$sdkMajor = [int]((dotnet --version).Split('.')[0])
if ($sdkMajor -lt 10) {
    throw "SDK $(dotnet --version) found, but this project targets net10.0-windows (needs SDK 10+)."
}

$wv2Keys = @(
    "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
    "HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
)
if (-not ($wv2Keys | Where-Object { Test-Path $_ })) {
    Write-Warning "WebView2 Runtime not detected. Install: https://developer.microsoft.com/microsoft-edge/webview2/"
}

# --- stop a running instance (it locks the output exe) ------------------------
Get-Process IdleShell -ErrorAction SilentlyContinue | ForEach-Object {
    Write-Host "Stopping running IdleShell (PID $($_.Id))..."
    $_ | Stop-Process -Force
    $_.WaitForExit(5000) | Out-Null
}

# --- clean --------------------------------------------------------------------
if ($Clean) {
    Write-Host "Cleaning..."
    foreach ($d in "bin", "obj", "out", "publish") {
        $p = Join-Path $PSScriptRoot $d
        if (Test-Path $p) { Remove-Item $p -Recurse -Force }
    }
}

# --- build / publish ----------------------------------------------------------
if ($Publish) {
    Write-Host "Publishing ($config, win-x64, single file)..."
    dotnet publish $project -c $config -r win-x64 --self-contained false `
        -p:PublishSingleFile=true -o $pubDir
    if ($LASTEXITCODE) { throw "publish failed" }
    $target = $pubDir
}
else {
    Write-Host "Building ($config)..."
    dotnet build $project -c $config -o $outDir
    if ($LASTEXITCODE) { throw "build failed" }
    $target = $outDir
}

# --- run ----------------------------------------------------------------------
if (-not $NoRun) {
    $exe = Join-Path $target "IdleShell.exe"
    if (-not (Test-Path $exe)) { throw "Build output missing: $exe" }
    Write-Host "Launching..."
    Start-Process -FilePath $exe -WorkingDirectory $target
}
