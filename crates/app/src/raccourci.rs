//! Les raccourcis qui lancent l'application : sur le bureau et dans le menu Démarrer de Windows.
//!
//! - Au premier lancement, l'application pose les deux raccourcis, une seule fois : un raccourci
//!   que l'utilisateur a supprimé n'est pas remis dans son dos. Paramètres les pose ou les retire.
//! - À chaque lancement, un raccourci qui mène ailleurs (dossier de l'application déplacé, archive
//!   extraite à un autre endroit) est refait vers l'exécutable lancé. Une mise à jour remplace
//!   l'exécutable sur place : le raccourci reste juste sans rien faire.
//! - Lancée depuis l'archive (l'explorateur de Windows copie alors l'exécutable dans un dossier
//!   temporaire qu'il efface ensuite), l'application ne pose ni ne refait aucun raccourci : il
//!   mènerait à un fichier bientôt effacé. Elle dit d'extraire l'archive d'abord.
//!
//! Rien n'est fait maison : sous Windows, le raccourci est écrit et relu par le shell de Windows
//! lui-même (`IShellLink`, à travers `winsafe`, sans code `unsafe` ici), et les dossiers viennent
//! de `SHGetKnownFolderPath`, qui suit un bureau déplacé (OneDrive) et les noms accentués.
//! Ailleurs qu'à Windows, il n'y a pas de raccourci ([`Raccourcis::du_systeme`] rend `None`).

use std::path::{Path, PathBuf};

/// Le nom du fichier de raccourci, le même aux deux endroits : c'est le nom affiché sous l'icône.
pub const NOM_DU_FICHIER: &str = "Computer Engineer.lnk";

/// La bulle d'aide du raccourci.
pub const DESCRIPTION: &str = "Construire un ordinateur à partir de portes NAND";

/// Un endroit où poser un raccourci.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lieu {
    /// Le bureau de l'utilisateur.
    Bureau,
    /// Le menu Démarrer de l'utilisateur (aussi trouvé par la recherche de Windows).
    MenuDemarrer,
}

impl Lieu {
    /// Les deux lieux, dans l'ordre de l'écran Paramètres.
    pub const TOUS: [Lieu; 2] = [Lieu::Bureau, Lieu::MenuDemarrer];

    /// Le rang du lieu dans [`Lieu::TOUS`].
    pub fn rang(self) -> usize {
        match self {
            Lieu::Bureau => 0,
            Lieu::MenuDemarrer => 1,
        }
    }

    /// Le lieu, écrit dans une phrase : « sur le bureau ».
    pub fn dans_une_phrase(self) -> &'static str {
        match self {
            Lieu::Bureau => "sur le bureau",
            Lieu::MenuDemarrer => "dans le menu Démarrer",
        }
    }

    /// Le lieu d'où l'on retire : « du bureau ».
    pub fn d_ou(self) -> &'static str {
        match self {
            Lieu::Bureau => "du bureau",
            Lieu::MenuDemarrer => "du menu Démarrer",
        }
    }
}

/// Ce que le système sait faire des raccourcis. Sous Windows, le shell ; dans les essais, des
/// fichiers texte.
pub trait Systeme: Send {
    /// Le dossier de ce lieu pour l'utilisateur courant, s'il en a un.
    fn dossier(&self, lieu: Lieu) -> Option<PathBuf>;
    /// Écrit dans `fichier` un raccourci qui lance `cible`.
    fn ecrire(&self, fichier: &Path, cible: &Path) -> Result<(), String>;
    /// Le programme que lance le raccourci `fichier`.
    fn cible(&self, fichier: &Path) -> Result<PathBuf, String>;
}

/// Les raccourcis de l'exécutable en marche.
pub struct Raccourcis {
    systeme: Box<dyn Systeme>,
    /// L'exécutable que les raccourcis lancent.
    exe: PathBuf,
    /// Vrai si l'exécutable tourne depuis un dossier temporaire (archive ouverte sans extraction).
    temporaire: bool,
}

impl std::fmt::Debug for Raccourcis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Raccourcis")
            .field("exe", &self.exe)
            .field("temporaire", &self.temporaire)
            .finish_non_exhaustive()
    }
}

/// Ce qu'a fait [`Raccourcis::au_lancement`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bilan {
    /// Les raccourcis posés pour la première fois.
    pub poses: Vec<Lieu>,
    /// Les raccourcis qui menaient ailleurs et ont été refaits.
    pub refaits: Vec<Lieu>,
    /// Les échecs, en une phrase chacun.
    pub erreurs: Vec<String>,
    /// Vrai si les raccourcis ont été proposés pendant ce lancement : à retenir dans les réglages.
    pub proposes: bool,
    /// Vrai si l'application tourne depuis l'archive : rien n'a été fait.
    pub depuis_l_archive: bool,
}

impl Bilan {
    /// Le message à montrer, s'il y a quelque chose à dire.
    pub fn message(&self) -> Option<String> {
        if self.depuis_l_archive {
            return Some(MESSAGE_ARCHIVE.to_owned());
        }
        if let Some(e) = self.erreurs.first() {
            return Some(format!(
                "Le raccourci n'a pas pu être posé : {e}. Tu peux réessayer dans Paramètres."
            ));
        }
        match self.poses.as_slice() {
            [] => None,
            [lieu] => Some(format!(
                "Raccourci ajouté {}. Tu peux le retirer dans Paramètres.",
                lieu.dans_une_phrase()
            )),
            _ => Some(
                "Raccourcis ajoutés sur le bureau et dans le menu Démarrer. Tu peux les retirer dans Paramètres."
                    .to_owned(),
            ),
        }
    }
}

/// Ce que l'application dit quand elle tourne depuis l'archive.
pub const MESSAGE_ARCHIVE: &str = "L'application tourne depuis l'archive : pour avoir un raccourci sur le bureau, extrais d'abord l'archive (clic droit, « Extraire tout »), puis lance l'application depuis le dossier obtenu.";

impl Raccourcis {
    /// Les raccourcis de l'exécutable en marche, avec le shell de Windows. `None` ailleurs qu'à
    /// Windows, ou si l'exécutable est introuvable.
    pub fn du_systeme() -> Option<Raccourcis> {
        #[cfg(windows)]
        {
            let exe = std::env::current_exe().ok()?;
            let temporaire = dans_un_dossier_temporaire(&exe, &std::env::temp_dir());
            Some(Raccourcis::avec(
                Box::new(coquille::Coquille),
                exe,
                temporaire,
            ))
        }
        #[cfg(not(windows))]
        {
            None
        }
    }

    /// Les raccourcis de `exe`, avec ce système (essais).
    pub fn avec(systeme: Box<dyn Systeme>, exe: PathBuf, temporaire: bool) -> Raccourcis {
        Raccourcis {
            systeme,
            exe,
            temporaire,
        }
    }

    /// Vrai si l'application tourne depuis un dossier temporaire.
    pub fn depuis_l_archive(&self) -> bool {
        self.temporaire
    }

    /// Le fichier du raccourci de ce lieu.
    pub fn fichier(&self, lieu: Lieu) -> Option<PathBuf> {
        self.systeme.dossier(lieu).map(|d| d.join(NOM_DU_FICHIER))
    }

    /// Pour chaque lieu de [`Lieu::TOUS`], vrai si son raccourci existe.
    pub fn presents(&self) -> [bool; 2] {
        Lieu::TOUS.map(|l| self.fichier(l).is_some_and(|f| f.is_file()))
    }

    /// Pose (ou refait) le raccourci de ce lieu vers l'exécutable en marche.
    pub fn poser(&self, lieu: Lieu) -> Result<(), String> {
        if self.temporaire {
            return Err(MESSAGE_ARCHIVE.to_owned());
        }
        let fichier = self
            .fichier(lieu)
            .ok_or_else(|| format!("Windows ne donne pas de dossier {}", lieu.dans_une_phrase()))?;
        if let Some(dossier) = fichier.parent() {
            std::fs::create_dir_all(dossier)
                .map_err(|e| format!("{} n'a pas pu être créé ({e})", dossier.display()))?;
        }
        self.systeme.ecrire(&fichier, &self.exe)
    }

    /// Retire le raccourci de ce lieu. Rien à faire s'il n'existe pas.
    pub fn retirer(&self, lieu: Lieu) -> Result<(), String> {
        let Some(fichier) = self.fichier(lieu) else {
            return Ok(());
        };
        match std::fs::remove_file(&fichier) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!(
                "{} n'a pas pu être supprimé ({e})",
                fichier.display()
            )),
        }
    }

    /// Au lancement : pose les raccourcis s'ils n'ont encore jamais été proposés (`proposes` est
    /// le réglage retenu), refait ceux qui mènent à un autre exécutable.
    pub fn au_lancement(&self, proposes: bool) -> Bilan {
        let mut bilan = Bilan::default();
        if self.temporaire {
            // Un message seulement tant que les raccourcis n'ont jamais été posés : celui qui les
            // a déjà n'a pas besoin qu'on lui explique comment les avoir.
            bilan.depuis_l_archive = !proposes;
            return bilan;
        }
        for lieu in Lieu::TOUS {
            let Some(fichier) = self.fichier(lieu) else {
                continue;
            };
            if !proposes {
                match self.poser(lieu) {
                    Ok(()) => bilan.poses.push(lieu),
                    Err(e) => bilan.erreurs.push(e),
                }
            } else if fichier.is_file() {
                let juste = self
                    .systeme
                    .cible(&fichier)
                    .is_ok_and(|c| meme_fichier(&c, &self.exe));
                if !juste {
                    match self.poser(lieu) {
                        Ok(()) => bilan.refaits.push(lieu),
                        Err(e) => bilan.erreurs.push(e),
                    }
                }
            }
        }
        bilan.proposes = !proposes;
        bilan
    }
}

/// Vrai si deux chemins désignent le même fichier, à la casse près comme sous Windows.
fn meme_fichier(a: &Path, b: &Path) -> bool {
    if let (Ok(a), Ok(b)) = (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        return a == b;
    }
    a.as_os_str().to_string_lossy().to_lowercase() == b.as_os_str().to_string_lossy().to_lowercase()
}

/// Vrai si l'exécutable tourne depuis le dossier temporaire : c'est là que l'explorateur de
/// Windows copie un programme lancé depuis une archive qu'on n'a pas extraite (`Temp1_…zip`).
/// Le dossier temporaire peut être écrit en nom court (`ZO~1` pour `Zoé`) : les deux chemins
/// sont donc comparés une fois résolus, puis par leur texte.
pub fn dans_un_dossier_temporaire(exe: &Path, temporaire: &Path) -> bool {
    if let (Ok(e), Ok(t)) = (
        std::fs::canonicalize(exe),
        std::fs::canonicalize(temporaire),
    ) {
        if e.starts_with(&t) {
            return true;
        }
    }
    if exe.starts_with(temporaire) {
        return true;
    }
    let texte = exe.to_string_lossy().to_lowercase().replace('/', "\\");
    texte.contains("\\appdata\\local\\temp\\")
}

/// Le shell de Windows, à travers `winsafe`.
#[cfg(windows)]
mod coquille {
    use std::path::{Path, PathBuf};

    use winsafe::{self as w, co, prelude::*};

    use super::{Lieu, Systeme, DESCRIPTION};

    /// Le shell de Windows.
    pub struct Coquille;

    /// Fait `travail` dans un fil à lui, avec COM ouvert en appartement simple : le fil de la
    /// fenêtre a déjà son propre réglage de COM (glisser-déposer), auquel on ne touche pas.
    fn dans_un_fil<T: Send>(travail: impl FnOnce() -> w::HrResult<T> + Send) -> Result<T, String> {
        std::thread::scope(|s| {
            s.spawn(|| {
                let _com =
                    w::CoInitializeEx(co::COINIT::APARTMENTTHREADED | co::COINIT::DISABLE_OLE1DDE)?;
                travail()
            })
            .join()
        })
        .map_err(|_| "erreur interne du shell".to_owned())?
        .map_err(|e| e.to_string())
    }

    fn texte(chemin: &Path) -> Result<&str, String> {
        chemin
            .to_str()
            .ok_or_else(|| format!("chemin illisible : {}", chemin.display()))
    }

    fn lien() -> w::HrResult<w::IShellLink> {
        w::CoCreateInstance::<w::IShellLink>(
            &co::CLSID::ShellLink,
            None::<&w::IUnknown>,
            co::CLSCTX::INPROC_SERVER,
        )
    }

    impl Systeme for Coquille {
        fn dossier(&self, lieu: Lieu) -> Option<PathBuf> {
            let id = match lieu {
                Lieu::Bureau => &co::KNOWNFOLDERID::Desktop,
                Lieu::MenuDemarrer => &co::KNOWNFOLDERID::Programs,
            };
            w::SHGetKnownFolderPath(id, co::KF::DEFAULT, None)
                .ok()
                .filter(|d| !d.is_empty())
                .map(PathBuf::from)
        }

        fn ecrire(&self, fichier: &Path, cible: &Path) -> Result<(), String> {
            let fichier = texte(fichier)?;
            let exe = texte(cible)?;
            let dossier = cible.parent().map(texte).transpose()?.unwrap_or("");
            dans_un_fil(|| {
                let l = lien()?;
                l.SetPath(exe)?;
                l.SetWorkingDirectory(dossier)?;
                l.SetIconLocation(exe, 0)?;
                l.SetDescription(DESCRIPTION)?;
                l.QueryInterface::<w::IPersistFile>()?
                    .Save(Some(fichier), true)
            })
        }

        fn cible(&self, fichier: &Path) -> Result<PathBuf, String> {
            let fichier = texte(fichier)?;
            dans_un_fil(|| {
                let l = lien()?;
                l.QueryInterface::<w::IPersistFile>()?
                    .Load(fichier, co::STGM::READ)?;
                l.GetPath(None, co::SLGP::RAWPATH)
            })
            .map(PathBuf::from)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un faux système : un dossier par lieu, et un raccourci est un fichier texte qui contient
    /// le chemin de sa cible.
    struct Faux {
        racine: PathBuf,
        sans_menu: bool,
    }

    impl Systeme for Faux {
        fn dossier(&self, lieu: Lieu) -> Option<PathBuf> {
            match lieu {
                Lieu::Bureau => Some(self.racine.join("Bureau")),
                Lieu::MenuDemarrer if self.sans_menu => None,
                Lieu::MenuDemarrer => Some(self.racine.join("Menu").join("Programmes")),
            }
        }
        fn ecrire(&self, fichier: &Path, cible: &Path) -> Result<(), String> {
            std::fs::write(fichier, cible.to_string_lossy().as_bytes()).map_err(|e| e.to_string())
        }
        fn cible(&self, fichier: &Path) -> Result<PathBuf, String> {
            std::fs::read_to_string(fichier)
                .map(PathBuf::from)
                .map_err(|e| e.to_string())
        }
    }

    fn essai(nom: &str) -> PathBuf {
        let r = std::env::temp_dir().join(format!("ce-raccourci-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&r);
        std::fs::create_dir_all(r.join("Bureau")).unwrap();
        std::fs::create_dir_all(r.join("Zoé app")).unwrap();
        std::fs::write(r.join("Zoé app").join("computer-engineer.exe"), b"exe").unwrap();
        r
    }

    fn raccourcis(r: &Path, exe: &str) -> Raccourcis {
        Raccourcis::avec(
            Box::new(Faux {
                racine: r.to_owned(),
                sans_menu: false,
            }),
            r.join(exe),
            false,
        )
    }

    #[test]
    fn au_premier_lancement_les_deux_raccourcis_sont_poses_une_seule_fois() {
        let r = essai("premier");
        let rc = raccourcis(&r, "Zoé app/computer-engineer.exe");
        assert_eq!(rc.presents(), [false, false]);
        let bilan = rc.au_lancement(false);
        assert_eq!(bilan.poses, Lieu::TOUS.to_vec());
        assert!(bilan.proposes);
        assert!(bilan
            .message()
            .unwrap()
            .contains("bureau et dans le menu Démarrer"));
        assert_eq!(
            rc.presents(),
            [true, true],
            "le dossier du menu est créé s'il manque"
        );

        // L'utilisateur supprime celui du bureau : il n'est pas remis au lancement suivant.
        std::fs::remove_file(rc.fichier(Lieu::Bureau).unwrap()).unwrap();
        let bilan = rc.au_lancement(true);
        assert_eq!(bilan, Bilan::default());
        assert_eq!(bilan.message(), None);
        assert_eq!(rc.presents(), [false, true]);
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn un_raccourci_qui_mene_ailleurs_est_refait_vers_l_executable_lance() {
        let r = essai("ailleurs");
        let ancien = raccourcis(&r, "Zoé app/computer-engineer.exe");
        ancien.au_lancement(false);
        // L'application a été déplacée : le raccourci du bureau mène à l'ancien endroit.
        std::fs::create_dir_all(r.join("Nouveau")).unwrap();
        std::fs::write(r.join("Nouveau/computer-engineer.exe"), b"exe").unwrap();
        std::fs::remove_dir_all(r.join("Zoé app")).unwrap();
        std::fs::remove_file(ancien.fichier(Lieu::MenuDemarrer).unwrap()).unwrap();
        let nouveau = raccourcis(&r, "Nouveau/computer-engineer.exe");
        let bilan = nouveau.au_lancement(true);
        assert_eq!(
            bilan.refaits,
            vec![Lieu::Bureau],
            "le raccourci retiré ne revient pas"
        );
        assert_eq!(bilan.message(), None, "rien à dire : il marche de nouveau");
        let cible = Faux {
            racine: r.clone(),
            sans_menu: false,
        }
        .cible(&nouveau.fichier(Lieu::Bureau).unwrap())
        .unwrap();
        assert!(meme_fichier(
            &cible,
            &r.join("Nouveau/computer-engineer.exe")
        ));
        // Juste : rien n'est refait.
        assert_eq!(nouveau.au_lancement(true), Bilan::default());
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn depuis_l_archive_rien_n_est_pose_et_l_application_le_dit() {
        let r = essai("archive");
        let rc = Raccourcis::avec(
            Box::new(Faux {
                racine: r.clone(),
                sans_menu: false,
            }),
            r.join("Zoé app/computer-engineer.exe"),
            true,
        );
        let bilan = rc.au_lancement(false);
        assert!(bilan.depuis_l_archive && !bilan.proposes);
        assert!(bilan.message().unwrap().contains("Extraire tout"));
        assert_eq!(rc.presents(), [false, false]);
        assert!(rc.poser(Lieu::Bureau).is_err());
        // Déjà posés une autre fois : rien à dire, et rien n'est refait vers le dossier temporaire.
        assert_eq!(rc.au_lancement(true), Bilan::default());
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn poser_et_retirer_depuis_les_parametres() {
        let r = essai("parametres");
        let rc = Raccourcis::avec(
            Box::new(Faux {
                racine: r.clone(),
                sans_menu: true,
            }),
            r.join("Zoé app/computer-engineer.exe"),
            false,
        );
        rc.poser(Lieu::Bureau).unwrap();
        assert_eq!(rc.presents(), [true, false]);
        rc.retirer(Lieu::Bureau).unwrap();
        rc.retirer(Lieu::Bureau).unwrap();
        assert_eq!(rc.presents(), [false, false]);
        assert!(
            rc.poser(Lieu::MenuDemarrer).is_err(),
            "pas de dossier : une erreur dite"
        );
        // Au premier lancement, seul le bureau est posé, et le message le dit.
        let bilan = rc.au_lancement(false);
        assert_eq!(bilan.poses, vec![Lieu::Bureau]);
        assert_eq!(
            bilan.message().unwrap(),
            "Raccourci ajouté sur le bureau. Tu peux le retirer dans Paramètres."
        );
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn le_dossier_temporaire_se_reconnait_meme_en_nom_court() {
        let t = std::env::temp_dir();
        assert!(dans_un_dossier_temporaire(
            &t.join("Temp1_computer-engineer.zip/computer-engineer.exe"),
            &t
        ));
        assert!(dans_un_dossier_temporaire(
            Path::new(r"C:\Users\Zoé\AppData\Local\Temp\Temp1_ce.zip\computer-engineer.exe"),
            Path::new(r"C:\Users\ZO~1\AppData\Local\Temp"),
        ));
        assert!(!dans_un_dossier_temporaire(
            Path::new(r"C:\Users\Zoé\Documents\Computer Engineer\computer-engineer.exe"),
            Path::new(r"C:\Users\ZO~1\AppData\Local\Temp"),
        ));
    }

    /// Le vrai shell de Windows (CI Windows) : un raccourci écrit dans un dossier au nom accentué
    /// se relit avec la bonne cible.
    #[cfg(windows)]
    #[test]
    fn le_shell_de_windows_ecrit_et_relit_un_raccourci() {
        let r = essai("shell");
        let exe = r.join("Zoé app").join("computer-engineer.exe");
        let fichier = r.join("Bureau").join(NOM_DU_FICHIER);
        let shell = coquille::Coquille;
        shell.ecrire(&fichier, &exe).unwrap();
        assert!(fichier.is_file());
        assert!(meme_fichier(&shell.cible(&fichier).unwrap(), &exe));
        assert!(shell.dossier(Lieu::Bureau).is_some());
        assert!(shell.dossier(Lieu::MenuDemarrer).is_some());
        let _ = std::fs::remove_dir_all(&r);
    }
}
