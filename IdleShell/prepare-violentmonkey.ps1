<#
.SYNOPSIS
  Downloads the official Violentmonkey MV3 release used by IdleShell.

  This does not clone, rebuild, or reimplement Violentmonkey. It places the
  upstream released unpacked extension under IdleShell/vendor/violentmonkey so
  WebView2 can install that exact extension.
#>

$ErrorActionPreference = "Stop"

$version = "2.49.0"
$assetUrl = "https://github.com/violentmonkey/violentmonkey/releases/download/v$version/Violentmonkey-mv3-v$version.zip"
$expectedSha256 = "3fa676c803a9698453ed23dfe496d4bc29a4bec52f6a95e55c43c2955921f8ee"

$vendorRoot = Join-Path $PSScriptRoot "vendor\violentmonkey"
$downloadPath = Join-Path $env:TEMP "Violentmonkey-mv3-v$version.zip"
$extractRoot = Join-Path $env:TEMP "IdleShell-Violentmonkey-$version-$([Guid]::NewGuid().ToString('N'))"

function Test-ViolentmonkeyFolder {
    if (-not (Test-Path (Join-Path $vendorRoot "manifest.json"))) {
        return $false
    }

    try {
        $manifest = Get-Content (Join-Path $vendorRoot "manifest.json") -Raw | ConvertFrom-Json
        return $manifest.manifest_version -eq 3 -and
               ([string]$manifest.version) -eq $version -and
               ([string]$manifest.name) -match "Violentmonkey|extName"
    }
    catch {
        return $false
    }
}

if (Test-ViolentmonkeyFolder) {
    Write-Host "Violentmonkey $version MV3 is already prepared."
    exit 0
}

Write-Host "Downloading official Violentmonkey $version MV3..."
Invoke-WebRequest -Uri $assetUrl -OutFile $downloadPath

$actualSha256 = (Get-FileHash -Algorithm SHA256 $downloadPath).Hash.ToLowerInvariant()
if ($actualSha256 -ne $expectedSha256) {
    Remove-Item $downloadPath -Force -ErrorAction SilentlyContinue
    throw "Violentmonkey archive SHA256 mismatch. Expected $expectedSha256 but received $actualSha256."
}

New-Item -ItemType Directory -Path $extractRoot -Force | Out-Null
Expand-Archive -LiteralPath $downloadPath -DestinationPath $extractRoot -Force

$manifest = Get-ChildItem -Path $extractRoot -Filter "manifest.json" -File -Recurse |
    Select-Object -First 1

if ($null -eq $manifest) {
    Remove-Item $extractRoot -Recurse -Force -ErrorAction SilentlyContinue
    throw "The official Violentmonkey archive did not contain manifest.json."
}

$sourceRoot = $manifest.Directory.FullName

if (Test-Path $vendorRoot) {
    Remove-Item $vendorRoot -Recurse -Force
}
New-Item -ItemType Directory -Path $vendorRoot -Force | Out-Null

Get-ChildItem -LiteralPath $sourceRoot -Force | ForEach-Object {
    Copy-Item -LiteralPath $_.FullName -Destination $vendorRoot -Recurse -Force
}

$installedManifest = Get-Content (Join-Path $vendorRoot "manifest.json") -Raw | ConvertFrom-Json
if ($installedManifest.manifest_version -ne 3 -or [string]$installedManifest.version -ne $version) {
    throw "Prepared Violentmonkey manifest does not match the pinned MV3 version $version."
}

Remove-Item $extractRoot -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item $downloadPath -Force -ErrorAction SilentlyContinue

Write-Host "Prepared official Violentmonkey $version MV3 at $vendorRoot"
