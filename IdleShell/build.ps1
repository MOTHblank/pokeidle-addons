$ErrorActionPreference = "Stop"

# Remove the obsolete pre-vendor extension layout before preparing the
# canonical IdleShell/vendor/violentmonkey runtime.
$legacyVmDir = Join-Path $PSScriptRoot "violentmonkey"
if (Test-Path $legacyVmDir) {
    Write-Host "Removing legacy Violentmonkey directory: $legacyVmDir"
    Remove-Item $legacyVmDir -Recurse -Force
}

& (Join-Path $PSScriptRoot "prepare-violentmonkey.ps1")
if ($LASTEXITCODE) { throw "Violentmonkey preparation failed" }

dotnet restore "$PSScriptRoot/IdleShell.csproj"
if ($LASTEXITCODE) { throw "restore failed" }

dotnet build "$PSScriptRoot/IdleShell.csproj" -c Release --no-restore
if ($LASTEXITCODE) { throw "build failed" }
