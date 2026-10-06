$ErrorActionPreference = "Stop"

dotnet restore "$PSScriptRoot/IdleShell.csproj"
dotnet build "$PSScriptRoot/IdleShell.csproj" -c Release --no-restore
