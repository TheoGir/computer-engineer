; Installateur Windows de Computer Engineer (Inno Setup 6).
;
; Installe l'application pour l'utilisateur seul, sans droits d'administrateur, dans
; %LOCALAPPDATA%\Programs\Computer Engineer (comme les applications courantes qui se mettent à
; jour seules) : les mises à jour automatiques y remplacent l'exécutable sans rien demander.
; Pose les raccourcis du menu Démarrer et du bureau, et inscrit l'application dans
; « Applications installées » des Paramètres de Windows, d'où on la désinstalle. Les projets
; (%APPDATA%\Computer Engineer) ne sont jamais supprimés par la désinstallation.
;
; Construit par la CI à partir de l'exécutable qu'elle a testé :
;   iscc /DVersion=X.Y.Z /DSource=<computer-engineer.exe> /O<dossier> installateur\computer-engineer.iss
;
; AppId est figé : il relie chaque installation et la désinstallation aux précédentes. Il est
; aussi dans crates/app/src/installation.rs (vérifié par un test).

#ifndef Version
  #error "Version manquante : iscc /DVersion=X.Y.Z"
#endif
#ifndef Source
  #define Source "..\target\release\computer-engineer.exe"
#endif

[Setup]
AppId={{66AE1388-2389-4176-93A1-D83C77A8C7F0}
AppName=Computer Engineer
AppVersion={#Version}
AppVerName=Computer Engineer {#Version}
AppPublisher=Computer Engineer
AppPublisherURL=https://github.com/TheoGir/computer-engineer-releases
AppSupportURL=https://github.com/TheoGir/computer-engineer-releases
AppUpdatesURL=https://github.com/TheoGir/computer-engineer-releases/releases/latest
AppComments=Construire un ordinateur complet à partir d'une seule porte logique, la NAND.
VersionInfoVersion={#Version}
VersionInfoDescription=Installateur de Computer Engineer
; Pour l'utilisateur seul : {autopf} est alors %LOCALAPPDATA%\Programs.
PrivilegesRequired=lowest
DefaultDirName={autopf}\Computer Engineer
DisableDirPage=auto
DisableProgramGroupPage=yes
DisableReadyPage=yes
UsePreviousAppDir=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
WizardStyle=modern
SetupIconFile=..\crates\app\icone\computer-engineer.ico
UninstallDisplayIcon={app}\computer-engineer.exe
UninstallDisplayName=Computer Engineer
OutputBaseFilename=computer-engineer-setup-v{#Version}-x86_64
Compression=lzma2/max
SolidCompression=yes
; L'application ouverte est fermée proprement (gestionnaire de redémarrage de Windows) avant de
; remplacer l'exécutable.
CloseApplications=yes
RestartApplications=no
SetupLogging=yes

[Languages]
Name: "fr"; MessagesFile: "compiler:Languages\French.isl"

[Tasks]
Name: "bureau"; Description: "Créer un raccourci sur le bureau"; GroupDescription: "Raccourcis :"

[Files]
Source: "{#Source}"; DestDir: "{app}"; DestName: "computer-engineer.exe"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Computer Engineer"; Filename: "{app}\computer-engineer.exe"; WorkingDir: "{app}"; Comment: "Construire un ordinateur à partir de portes NAND"
Name: "{autodesktop}\Computer Engineer"; Filename: "{app}\computer-engineer.exe"; WorkingDir: "{app}"; Comment: "Construire un ordinateur à partir de portes NAND"; Tasks: bureau

[Run]
Filename: "{app}\computer-engineer.exe"; Description: "Lancer Computer Engineer"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
; Les raccourcis que l'application a pu reposer elle-même (Paramètres), et ce qu'une mise à jour
; automatique laisse à côté de l'exécutable.
Type: files; Name: "{autodesktop}\Computer Engineer.lnk"
Type: files; Name: "{autoprograms}\Computer Engineer.lnk"
Type: files; Name: "{app}\computer-engineer.exe.precedent"
Type: files; Name: "{app}\.computer-engineer.*.__temp__.exe"
Type: dirifempty; Name: "{app}"
