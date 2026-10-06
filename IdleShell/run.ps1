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
$outDir  = Join-Path $PSScriptRoot "out"
$pubDir  = Join-Path $PSScriptRoot "publish"

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

# --- extensions folder (drop unpacked Tampermonkey here) ----------------------
$extDir = Join-Path $target "extensions"
if (-not (Test-Path $extDir)) {
    New-Item -ItemType Directory -Path $extDir | Out-Null
    Write-Host "Created $extDir  (put unpacked extensions in subfolders, e.g. extensions\tampermonkey\manifest.json)"
}

$exe = Join-Path $target "IdleShell.exe"
Write-Host "Built: $exe"

# --- run ----------------------------------------------------------------------
if (-not $NoRun) {
    Write-Host "Launching..."
    Start-Process -FilePath $exe -WorkingDirectory $target
}
