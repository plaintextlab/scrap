; Windows installer for Scrap (Inno Setup 6).
;
; Built by CI next to the portable exe, after `cargo build --release`:
;   iscc /DVersion=0.1.25 packaging\windows\scrap.iss   -> dist\scrap-v0.1.25-windows-x64-setup.exe
;
; Installs for the current user by default (no admin prompt), into
; %LOCALAPPDATA%\Programs\Scrap; "Install for all users" puts it in Program Files.
; The in-app updater recognises an installed copy by the uninstaller next to scrap.exe
; and updates it by running the newer setup with /SILENT /RELAUNCH=1.

#ifndef Version
  #define Version "0.0.0"
#endif
#ifndef Source
  #define Source "..\..\target\release\scrap.exe"
#endif
#ifndef Output
  #define Output "..\..\dist"
#endif

[Setup]
; Never change AppId: it's how Windows matches upgrades and the uninstaller.
AppId={{4A2BB295-1971-4EC6-953C-C0EEC23CD936}
AppName=Scrap
AppVersion={#Version}
AppVerName=Scrap {#Version}
AppPublisher=plaintextlab
AppPublisherURL=https://github.com/plaintextlab/scrap
AppSupportURL=https://github.com/plaintextlab/scrap/issues
AppUpdatesURL=https://github.com/plaintextlab/scrap/releases
VersionInfoVersion={#Version}
VersionInfoProductName=Scrap
DefaultDirName={autopf}\Scrap
DefaultGroupName=Scrap
DisableProgramGroupPage=yes
DisableDirPage=auto
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog commandline
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
WizardStyle=modern
SetupIconFile=..\icons\scrap.ico
UninstallDisplayIcon={app}\scrap.exe
UninstallDisplayName=Scrap
CloseApplications=yes
RestartApplications=no
Compression=lzma2/max
SolidCompression=yes
OutputDir={#Output}
OutputBaseFilename=scrap-v{#Version}-windows-x64-setup

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#Source}"; DestDir: "{app}"; DestName: "scrap.exe"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Scrap"; Filename: "{app}\scrap.exe"; AppUserModelID: "dev.plaintextlab.Scrap"
Name: "{autodesktop}\Scrap"; Filename: "{app}\scrap.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\scrap.exe"; Description: "{cm:LaunchProgram,Scrap}"; Flags: nowait postinstall skipifsilent runasoriginaluser
; After an in-app update (silent), start Scrap again.
Filename: "{app}\scrap.exe"; Flags: nowait runasoriginaluser; Check: Relaunch

[UninstallDelete]
; Left behind by an update that replaced the exe while it was running.
Type: files; Name: "{app}\scrap.exe.old"
Type: files; Name: "{app}\scrap.exe.new"

[Code]
function Relaunch: Boolean;
begin
  Result := ExpandConstant('{param:RELAUNCH|0}') = '1';
end;
