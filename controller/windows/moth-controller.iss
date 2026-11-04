#ifndef AppVersion
  #error AppVersion must be provided by the release workflow
#endif

#ifndef ControllerExe
  #error ControllerExe must be provided by the release workflow
#endif

#ifndef ControllerIcon
  #error ControllerIcon must be provided by the release workflow
#endif

#define MyAppName "Moth Controller"
#define MyAppPublisher "MOTHblank"
#define MyAppURL "https://github.com/MOTHblank/pokeidle-addons"

[Setup]
AppId={{5E1E5540-9723-4D63-9A92-7A9AA6BDA74C}
AppName={#MyAppName}
AppVersion={#AppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}/issues
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={localappdata}\Programs\Moth Controller
DefaultGroupName=Moth Controller
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
UsedUserAreasWarning=no
ArchitecturesAllowed=x64compatible
OutputBaseFilename=Moth-Controller-Setup-v{#AppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
SetupIconFile={#ControllerIcon}
UninstallDisplayIcon={app}\Moth Controller.exe
CloseApplications=yes
RestartApplications=no
SetupLogging=yes

[Files]
Source: "{#ControllerExe}"; DestDir: "{app}"; DestName: "Moth Controller.exe"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Moth Controller"; Filename: "{app}\Moth Controller.exe"; WorkingDir: "{app}"

[Run]
Filename: "{app}\Moth Controller.exe"; Description: "Launch Moth Controller"; Flags: nowait postinstall skipifsilent
Filename: "{app}\Moth Controller.exe"; Flags: nowait; Check: IsMothUpdate

[Code]
function IsMothUpdate(): Boolean;
begin
  Result := CompareText(ExpandConstant('{param:MOTHUPDATE|0}'), '1') = 0;
end;
