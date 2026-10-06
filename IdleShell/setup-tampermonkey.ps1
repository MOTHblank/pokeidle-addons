#requires -Version 5.1
<#
.SYNOPSIS
    Installs the official Tampermonkey package for PokéIdle Idle Shell.

.DESCRIPTION
    Downloads the official stable Chrome/Edge MV3 Tampermonkey package,
    extracts the CRX payload, and installs it into the persistent Idle Shell
    extension directory.

    The package is not committed to this repository because current
    Tampermonkey releases are distributed under a proprietary license.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$DownloadUrl = 'https://www.tampermonkey.net/crx/tampermonkey_stable.crx'
$ExtensionRoot = Join-Path $env:LOCALAPPDATA 'Moth\IdleShell\Extensions'
$Destination = Join-Path $ExtensionRoot 'Tampermonkey'
$TempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ('IdleShell-Tampermonkey-' + [Guid]::NewGuid().ToString('N'))
$CrxPath = Join-Path $TempRoot 'tampermonkey.crx'
$ZipPath = Join-Path $TempRoot 'tampermonkey.zip'

function Read-UInt32LittleEndian {
    param(
        [byte[]] $Bytes,
        [int] $Offset
    )

    [BitConverter]::ToUInt32($Bytes, $Offset)
}

try {
    New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null
    New-Item -ItemType Directory -Force -Path $ExtensionRoot | Out-Null

    Write-Host 'Downloading official stable Tampermonkey package...'
    $client = New-Object System.Net.WebClient
    try {
        $client.DownloadFile($DownloadUrl, $CrxPath)
    }
    finally {
        $client.Dispose()
    }

    $bytes = [System.IO.File]::ReadAllBytes($CrxPath)

    if ($bytes.Length -lt 12) {
        throw 'The downloaded file is too small to be a valid CRX package.'
    }

    $magic = [System.Text.Encoding]::ASCII.GetString($bytes, 0, 4)
    if ($magic -ne 'Cr24') {
        throw 'The downloaded file is not a CRX package.'
    }

    $crxVersion = Read-UInt32LittleEndian -Bytes $bytes -Offset 4

    switch ($crxVersion) {
        2 {
            if ($bytes.Length -lt 16) {
                throw 'The CRX2 header is incomplete.'
            }

            $publicKeyLength = Read-UInt32LittleEndian -Bytes $bytes -Offset 8
            $signatureLength = Read-UInt32LittleEndian -Bytes $bytes -Offset 12
            $zipOffset = 16 + $publicKeyLength + $signatureLength
        }

        3 {
            $headerLength = Read-UInt32LittleEndian -Bytes $bytes -Offset 8
            $zipOffset = 12 + $headerLength
        }

        default {
            throw "Unsupported CRX version: $crxVersion"
        }
    }

    if ($zipOffset -ge $bytes.Length) {
        throw 'The CRX payload offset is outside the downloaded file.'
    }

    $zipBytes = New-Object byte[] ($bytes.Length - $zipOffset)
    [Array]::Copy($bytes, $zipOffset, $zipBytes, 0, $zipBytes.Length)
    [System.IO.File]::WriteAllBytes($ZipPath, $zipBytes)

    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $Destination
    New-Item -ItemType Directory -Force -Path $Destination | Out-Null

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [System.IO.Compression.ZipFile]::ExtractToDirectory(
        $ZipPath,
        $Destination)

    $manifestPath = Join-Path $Destination 'manifest.json'
    if (-not (Test-Path $manifestPath -PathType Leaf)) {
        throw 'The extracted package does not contain manifest.json.'
    }

    $manifest = Get-Content -Raw -Path $manifestPath | ConvertFrom-Json
    Write-Host "Installed $($manifest.name) $($manifest.version)."
    Write-Host "Path: $Destination"
}
finally {
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $TempRoot
}
