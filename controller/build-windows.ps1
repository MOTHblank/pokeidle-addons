# Build Moth Controller without overwriting an executable still in use.
# Run from PowerShell: .\build-windows.ps1
# The currently running controller and accounts are never stopped.

$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $PSScriptRoot

function Test-ExecutableLocked {
    param([string] $Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return $false
    }

    try {
        # A running Windows .exe cannot be replaced. Checking the file
        # itself avoids terminating the player's active game sessions.
        $stream = [System.IO.File]::Open(
            $Path,
            [System.IO.FileMode]::Open,
            [System.IO.FileAccess]::ReadWrite,
            [System.IO.FileShare]::None
        )
        $stream.Dispose()
        return $false
    }
    catch [System.IO.IOException] {
        return $true
    }
    catch [System.UnauthorizedAccessException] {
        return $true
    }
}

$targetDir = Join-Path $PSScriptRoot 'target'
$binary = Join-Path $targetDir 'release/moth-controller.exe'

if (Test-ExecutableLocked -Path $binary) {
    Write-Host 'The existing Moth Controller is running or locked; building separately.'
    $targetDir = Join-Path $PSScriptRoot 'target/build-while-running'
    $binary = Join-Path $targetDir 'release/moth-controller.exe'

    if (Test-ExecutableLocked -Path $binary) {
        # Do not overwrite another build which is also running.
        $suffix = Get-Date -Format 'yyyyMMdd-HHmmss-fff'
        $targetDir = Join-Path $PSScriptRoot "target/build-while-running-$suffix"
        $binary = Join-Path $targetDir 'release/moth-controller.exe'
    }
}

& cargo build --release --locked --target-dir $targetDir
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

Write-Host ''
Write-Host 'Build completed successfully.'
Write-Host "Executable: $binary"
if ($targetDir -ne (Join-Path $PSScriptRoot 'target')) {
    Write-Host 'The old running app is unchanged; launch the newly built executable when ready.'
}
