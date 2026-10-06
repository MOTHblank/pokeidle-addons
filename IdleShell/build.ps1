$ErrorActionPreference = "Stop"

& (Join-Path $PSScriptRoot "prepare-violentmonkey.ps1")
if ($LASTEXITCODE) { throw "Violentmonkey preparation failed" }

dotnet restore "$PSScriptRoot/IdleShell.csproj"
if ($LASTEXITCODE) { throw "restore failed" }

dotnet build "$PSScriptRoot/IdleShell.csproj" -c Release --no-restore
if ($LASTEXITCODE) { throw "build failed" }
