//! L'application installée par l'installateur Windows (`installateur/computer-engineer.iss`,
//! Inno Setup, pour l'utilisateur seul, sans droits d'administrateur).
//!
//! L'installateur inscrit l'application dans « Applications installées » des Paramètres de
//! Windows, avec son numéro de version. Une mise à jour automatique remplace ensuite
//! l'exécutable sur place sans repasser par l'installateur : au lancement, l'application remet
//! donc elle-même le numéro affiché à jour, si c'est bien elle qui est inscrite à cet endroit.
//! Une application lancée depuis l'archive n'est pas inscrite : rien n'est fait.

use std::path::Path;

/// L'identifiant de l'application pour l'installateur (`AppId` d'Inno Setup). Figé : il relie
/// chaque nouvelle installation, et la désinstallation, à la précédente.
pub const IDENTIFIANT: &str = "{66AE1388-2389-4176-93A1-D83C77A8C7F0}";

/// La clé du registre (sous `HKEY_CURRENT_USER`) où Inno Setup inscrit l'application.
pub fn cle_de_desinstallation() -> String {
    format!(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{IDENTIFIANT}_is1")
}

/// Vrai si `exe` est dans le dossier `inscrit` (la valeur `InstallLocation` de l'inscription),
/// à la casse et à la barre finale près, comme sous Windows.
pub fn installe_ici(exe: &Path, inscrit: &str) -> bool {
    let normal = |t: &str| t.replace('/', "\\").trim_end_matches('\\').to_lowercase();
    let exe = normal(&exe.to_string_lossy());
    let Some((dossier, _)) = exe.rsplit_once('\\') else {
        return false;
    };
    !inscrit.trim().is_empty() && dossier == normal(inscrit)
}

/// Vrai si l'exécutable en marche est celui que l'installateur a inscrit. L'installateur a
/// alors proposé lui-même les raccourcis : l'application ne les repose pas dans le dos de
/// l'utilisateur qui a décoché celui du bureau.
pub fn par_l_installateur() -> bool {
    #[cfg(windows)]
    {
        std::env::current_exe()
            .ok()
            .is_some_and(|exe| registre::inscrite(&exe).unwrap_or(false))
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Remet à jour la version affichée dans les Paramètres de Windows, si l'exécutable en marche
/// est celui que l'installateur a inscrit. Rend vrai si la valeur a été changée. Toute erreur
/// (pas inscrite, registre illisible) laisse les choses en l'état : ce n'est qu'un affichage.
pub fn tenir_la_version_a_jour(version: &str) -> bool {
    #[cfg(windows)]
    {
        std::env::current_exe()
            .ok()
            .is_some_and(|exe| registre::mettre_a_jour(&exe, version).unwrap_or(false))
    }
    #[cfg(not(windows))]
    {
        let _ = version;
        false
    }
}

#[cfg(windows)]
mod registre {
    use std::path::Path;

    use winsafe::{self as w, co};

    fn ouvrir(droits: co::KEY) -> w::SysResult<w::guard::RegCloseKeyGuard> {
        w::HKEY::CURRENT_USER.RegOpenKeyEx(
            Some(&super::cle_de_desinstallation()),
            co::REG_OPTION::default(),
            droits,
        )
    }

    fn ici(cle: &w::HKEY, exe: &Path) -> w::SysResult<bool> {
        Ok(match cle.RegQueryValueEx(Some("InstallLocation"))? {
            w::RegistryValue::Sz(inscrit) => super::installe_ici(exe, &inscrit),
            _ => false,
        })
    }

    pub fn inscrite(exe: &Path) -> w::SysResult<bool> {
        let cle = ouvrir(co::KEY::QUERY_VALUE)?;
        ici(&cle, exe)
    }

    pub fn mettre_a_jour(exe: &Path, version: &str) -> w::SysResult<bool> {
        let cle = ouvrir(co::KEY::QUERY_VALUE | co::KEY::SET_VALUE)?;
        if !ici(&cle, exe)? {
            return Ok(false);
        }
        if let Ok(w::RegistryValue::Sz(affichee)) = cle.RegQueryValueEx(Some("DisplayVersion")) {
            if affichee == version {
                return Ok(false);
            }
        }
        cle.RegSetValueEx(
            Some("DisplayVersion"),
            w::RegistryValue::Sz(version.to_owned()),
        )?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l_identifiant_est_celui_du_script_de_l_installateur() {
        let script = include_str!("../../../installateur/computer-engineer.iss");
        let attendu = format!("AppId={{{IDENTIFIANT}");
        assert!(
            script.contains(&attendu),
            "le script doit déclarer {attendu} (l'accolade ouvrante est doublée dans Inno Setup)"
        );
    }

    #[test]
    fn l_exe_est_reconnu_dans_son_dossier_d_installation() {
        let exe = Path::new(
            r"C:\Users\Zoé\AppData\Local\Programs\Computer Engineer\computer-engineer.exe",
        );
        assert!(installe_ici(
            exe,
            r"C:\Users\Zoé\AppData\Local\Programs\Computer Engineer\"
        ));
        assert!(installe_ici(
            exe,
            r"c:\users\zoé\appdata\local\programs\computer engineer"
        ));
        assert!(!installe_ici(
            exe,
            r"C:\Users\Zoé\Documents\Computer Engineer\"
        ));
        assert!(!installe_ici(exe, ""));
    }
}
