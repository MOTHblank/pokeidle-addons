$ErrorActionPreference = "Stop"

dotnet restore "$PSScriptRoot/IdleShell.csproj"
if ($LASTEXITCODE) { throw "restore failed" }

dotnet build "$PSScriptRoot/IdleShell.csproj" -c Release --no-restore
if ($LASTEXITCODE) { throw "build failed" }
