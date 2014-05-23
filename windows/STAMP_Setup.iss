; Script generated by the Inno Setup Script Wizard.
; SEE THE DOCUMENTATION FOR DETAILS ON CREATING INNO SETUP SCRIPT FILES!

[Setup]
; NOTE: The value of AppId uniquely identifies this application.
; Do not use the same AppId value in installers for other applications.
; (To generate a new GUID, click Tools | Generate GUID inside the IDE.)
AppId={{058AA4B2-FB5A-4D89-9134-D6E06AB9E894}
AppName=STAMP
AppVerName=STAMP v2.0.3
AppPublisher=Donovan Parks and Robert Beiko
AppPublisherURL=http://kiwi.cs.dal.ca/Software/STAMP
AppSupportURL=http://kiwi.cs.dal.ca/Software/STAMP
AppUpdatesURL=http://kiwi.cs.dal.ca/Software/STAMP
DefaultDirName={pf}\STAMP
DefaultGroupName=STAMP
AllowNoIcons=yes
LicenseFile=..\dist\LICENSE.txt
InfoBeforeFile=..\dist\readme.txt
OutputDir=.\install
OutputBaseFilename=STAMP_2_0_3
SetupIconFile=..\dist\icons\programIcon.ico
Compression=lzma
SolidCompression=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\dist\STAMP.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist\STAMP.exe.log"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist\LICENSE.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist\python27.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist\STAMP_Users_Guide.pdf"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist\w9xpopen.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\dist\examples\*"; DestDir: "{app}\examples"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\dist\icons\*"; DestDir: "{app}\icons"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\dist\library\*"; DestDir: "{app}\library"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\dist\mpl-data\*"; DestDir: "{app}\mpl-data"; Flags: ignoreversion recursesubdirs createallsubdirs
; NOTE: Don't use "Flags: ignoreversion" on any shared system files

[Icons]
Name: "{group}\STAMP"; Filename: "{app}\STAMP.exe"
Name: "{group}\{cm:ProgramOnTheWeb,STAMP}"; Filename: "http://kiwi.cs.dal.ca/Software/STAMP"
Name: "{group}\{cm:UninstallProgram,STAMP}"; Filename: "{uninstallexe}"
Name: "{commondesktop}\STAMP"; Filename: "{app}\STAMP.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\STAMP.exe"; Description: "{cm:LaunchProgram,STAMP}"; Flags: nowait postinstall skipifsilent

