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
        $hasStaticInjector = $manifest.content_scripts -and
            ($manifest.content_scripts | Where-Object {
                $_.js -contains "injected.js" -and
                $_.js -contains "injected-web.js"
            })

        $hasUserScriptsPermission = $manifest.permissions -contains "userScripts"

        return $manifest.manifest_version -eq 3 -and
               ([string]$manifest.version) -eq $version -and
               ([string]$manifest.name) -match "Violentmonkey|extName" -and
               $hasStaticInjector -and
               $hasUserScriptsPermission
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

$manifestPath = Join-Path $vendorRoot "manifest.json"
$installedManifest = Get-Content $manifestPath -Raw | ConvertFrom-Json
if ($installedManifest.manifest_version -ne 3 -or [string]$installedManifest.version -ne $version) {
    throw "Prepared Violentmonkey manifest does not match the pinned MV3 version $version."
}

# WebView2 does not expose Chromium's chrome://extensions "Allow User Scripts"
# toggle. Violentmonkey's MV3 release normally relies on chrome.userScripts for
# dynamic user-code registration, so we also declare VM's own injected bootstrap
# as a regular static content script. VM already contains the full fallback
# injection pipeline (the same injected.js/injected-web.js files used by its
# MV2 build); no script engine is reimplemented here.
$staticInjector = @{
    matches = @("<all_urls>")
    js = @("injected-web.js", "injected.js")
    run_at = "document_start"
    all_frames = $true
}

# PowerShell cannot assign a property that is absent from a PSCustomObject.
# The official MV3 manifest does not currently declare content_scripts, so add
# the property when needed; do the same for permissions for forward compatibility.
if ($null -eq $installedManifest.PSObject.Properties["content_scripts"]) {
    $installedManifest | Add-Member -MemberType NoteProperty -Name "content_scripts" -Value @($staticInjector)
}
else {
    $installedManifest.content_scripts = @($staticInjector)
}

# Keep the official userScripts permission. Violentmonkey's MV3 engine uses
# chrome.userScripts when the host exposes it. The static injected-web.js /
# injected.js content script below remains present for WebView2, so ordinary
# userscripts do not depend on the Chromium "Allow User Scripts" UI.

$manifestJson = $installedManifest | ConvertTo-Json -Depth 30
Set-Content -LiteralPath $manifestPath -Value $manifestJson -Encoding UTF8

$finalManifest = Get-Content $manifestPath -Raw | ConvertFrom-Json
$hasStaticInjector = $finalManifest.content_scripts -and
    ($finalManifest.content_scripts | Where-Object {
        $_.js -contains "injected.js" -and $_.js -contains "injected-web.js"
    })
if (-not $hasStaticInjector) {
    throw "Failed to add Violentmonkey's static injector to the WebView2 manifest."
}
if (-not ($finalManifest.permissions -contains "userScripts")) {
    throw "Prepared Violentmonkey manifest is missing the userScripts permission."
}

Remove-Item $extractRoot -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item $downloadPath -Force -ErrorAction SilentlyContinue

Write-Host "Prepared official Violentmonkey $version MV3 (WebView2 static-injector manifest) at $vendorRoot"
