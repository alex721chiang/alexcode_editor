; AlexCode Windows 安裝程式（Inno Setup）。版本與部署資料夾由 CI 以 /D 傳入。
#ifndef MyAppVersion
  #define MyAppVersion "4.5.0"
#endif
#ifndef DeployDir
  #define DeployDir "..\build\src\Release"
#endif
#ifndef OutDir
  #define OutDir "..\dist"
#endif

#define MyAppName "AlexCode"
#define MyAppExeName "AlexCode.exe"
#define MyAppPublisher "Alex Chiang"
#define MyAppURL "https://github.com/alex721chiang/alexcode_editor"

[Setup]
AppId={{8B2A1C5E-4D3F-4A6B-9E12-AAAACODE0001}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
OutputDir={#OutDir}
OutputBaseFilename=AlexCode-Setup-{#MyAppVersion}
Compression=lzma2
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
WizardStyle=modern
UninstallDisplayIcon={app}\{#MyAppExeName}

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#DeployDir}\*"; DestDir: "{app}"; Flags: recursesubdirs createallsubdirs ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#MyAppName}}"; Flags: nowait postinstall skipifsilent
