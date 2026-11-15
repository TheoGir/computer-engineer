//! Quand quelque chose tourne mal : l'application ne s'arrête jamais en silence.
//!
//! - Un plantage (panique) écrit un rapport lisible dans `rapports/` du dossier de données :
//!   version, système, message, endroit du code et pile d'appels. L'exécutable publié s'arrête
//!   aussitôt après (`panic = "abort"`), mais le projet ouvert a été enregistré au plus une
//!   seconde avant (enregistrement automatique), et le lancement suivant le dit et propose
//!   d'ouvrir le rapport.
//! - Un démarrage impossible (carte graphique introuvable, pilote trop ancien) écrit aussi un
//!   rapport et, sous Windows où il n'y a pas de console, l'annonce dans une boîte de message.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Sous-dossier des rapports, dans le dossier de données.
pub const DOSSIER_RAPPORTS: &str = "rapports";
/// Nombre de rapports gardés : les plus anciens s'effacent.
const RAPPORTS_GARDES: usize = 20;
/// Marque d'un rapport déjà signalé à l'utilisateur.
const LU: &str = ".lu";

/// Le dossier des rapports sous cette racine de données.
pub fn dossier_des_rapports(racine: &Path) -> PathBuf {
    racine.join(DOSSIER_RAPPORTS)
}

/// Date et heure UTC lisibles, `AAAA-MM-JJ HH:MM:SS`, sans bibliothèque de dates.
pub fn date_utc(instant: SystemTime) -> String {
    let s = instant
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (jours, reste) = (s / 86_400, s % 86_400);
    let (a, m, j) = civil(jours as i64);
    format!(
        "{a:04}-{m:02}-{j:02} {:02}:{:02}:{:02}",
        reste / 3600,
        reste % 3600 / 60,
        reste % 60
    )
}

/// Jour civil (année, mois, jour) du jour `z` compté depuis le 1er janvier 1970 (algorithme
/// de H. Hinnant, calendrier grégorien proleptique).
fn civil(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let ere = z.div_euclid(146_097);
    let jde = z.rem_euclid(146_097); // jour de l'ère, 0 à 146 096
    let ade = (jde - jde / 1460 + jde / 36_524 - jde / 146_096) / 365; // année de l'ère
    let jda = jde - (365 * ade + ade / 4 - ade / 100); // jour de l'année, depuis le 1er mars
    let mp = (5 * jda + 2) / 153;
    let j = (jda - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let a = ade + ere * 400 + i64::from(m <= 2);
    (a, m, j)
}

/// Le texte d'un rapport : en-tête commun puis le détail.
pub fn texte_du_rapport(titre: &str, detail: &str) -> String {
    tr!("Computer Engineer {version} — {titre}\nDate (UTC) : {date}\nSystème : {os} {arch}\n\n{detail}\n\nCe rapport ne contient aucun de tes projets. Pour aider à corriger le problème, tu peux l'envoyer tel quel à qui s'occupe de l'application.\n", version = env!("CARGO_PKG_VERSION"), titre = titre, date = date_utc(SystemTime::now()), os = std::env::consts::OS, arch = std::env::consts::ARCH, detail = detail)
}

/// Écrit un rapport `<genre>-<horodatage>.txt` dans `dossier`, efface les plus anciens au-delà
/// de `RAPPORTS_GARDES`, et renvoie son chemin.
pub fn ecrire_un_rapport(dossier: &Path, genre: &str, texte: &str) -> io::Result<PathBuf> {
    fs::create_dir_all(dossier)?;
    let instant = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    let chemin = (0..)
        .map(|k| {
            let suffixe = if k == 0 {
                String::new()
            } else {
                format!("-{k}")
            };
            dossier.join(format!("{genre}-{instant}{suffixe}.txt"))
        })
        .find(|c| !c.exists())
        .expect("une suite infinie trouve toujours un nom libre");
    fs::write(&chemin, texte)?;
    elaguer(dossier);
    Ok(chemin)
}

/// Les rapports de ce dossier, du plus ancien au plus récent.
fn rapports(dossier: &Path) -> Vec<PathBuf> {
    let Ok(lecture) = fs::read_dir(dossier) else {
        return Vec::new();
    };
    let mut tous: Vec<(SystemTime, PathBuf)> = lecture
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "txt"))
        .map(|p| {
            let date = fs::metadata(&p)
                .and_then(|m| m.modified())
                .unwrap_or(UNIX_EPOCH);
            (date, p)
        })
        .collect();
    tous.sort();
    tous.into_iter().map(|(_, p)| p).collect()
}

fn elaguer(dossier: &Path) {
    let tous = rapports(dossier);
    let trop = tous.len().saturating_sub(RAPPORTS_GARDES);
    for vieux in &tous[..trop] {
        let _ = fs::remove_file(vieux);
    }
}

/// Vrai si ce rapport a déjà été signalé.
fn est_lu(chemin: &Path) -> bool {
    chemin
        .file_stem()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.ends_with(LU))
}

/// Le plus récent rapport pas encore signalé, s'il y en a un.
pub fn rapport_non_lu(dossier: &Path) -> Option<PathBuf> {
    rapports(dossier).into_iter().rev().find(|p| !est_lu(p))
}

/// Marque tous les rapports comme signalés (`plantage-1.txt` devient `plantage-1.lu.txt`).
pub fn marquer_lus(dossier: &Path) {
    for p in rapports(dossier) {
        if est_lu(&p) {
            continue;
        }
        if let Some(nom) = p.file_stem().and_then(|s| s.to_str()) {
            let _ = fs::rename(&p, p.with_file_name(format!("{nom}{LU}.txt")));
        }
    }
}

/// Le détail d'une panique : message, endroit, fil d'exécution, pile d'appels.
pub fn detail_de_panique(info: &std::panic::PanicHookInfo<'_>) -> String {
    let message = info
        .payload()
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| info.payload().downcast_ref::<String>().cloned())
        .unwrap_or_else(|| tr!("(message illisible)").to_owned());
    let endroit = info.location().map_or_else(
        || "inconnu".to_owned(),
        |l| format!("{}:{}:{}", l.file(), l.line(), l.column()),
    );
    let fil = std::thread::current()
        .name()
        .unwrap_or(tr!("sans nom"))
        .to_owned();
    let pile = std::backtrace::Backtrace::force_capture();
    tr!("Message : {message}\nEndroit : {endroit}\nFil d'exécution : {fil}\n\nPile d'appels :\n{pile}", message = message, endroit = endroit, fil = fil, pile = pile)
}

/// Installe le rapport de plantage : toute panique écrit un rapport dans `dossier`, puis le
/// comportement habituel suit (message sur la sortie d'erreur, arrêt).
pub fn installer_le_rapport_de_plantage(dossier: PathBuf) {
    let precedent = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let texte = texte_du_rapport("plantage", &detail_de_panique(info));
        let _ = ecrire_un_rapport(&dossier, "plantage", &texte);
        precedent(info);
    }));
}

/// Nom du fichier verrou, dans le dossier de données.
pub const VERROU: &str = "application-ouverte.verrou";

/// Ce que donne la prise du verrou du dossier de données.
#[derive(Debug)]
pub enum Verrou {
    /// Le verrou est pris : le garder ouvert tant que l'application tourne. Le système le rend
    /// tout seul à l'arrêt du programme, même brutal.
    Pris(fs::File),
    /// Une autre fenêtre de l'application tient ce dossier.
    Occupe,
    /// Le verrou n'a pas pu être posé (droits, disque) : on continue sans.
    Impossible(io::Error),
}

/// Prend le verrou du dossier de données, en réessayant pendant `patience` : après une mise à
/// jour, la nouvelle version démarre pendant que l'ancienne finit de s'arrêter.
pub fn verrouiller(racine: &Path, patience: std::time::Duration) -> Verrou {
    if let Err(e) = fs::create_dir_all(racine) {
        return Verrou::Impossible(e);
    }
    let fichier = match fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(racine.join(VERROU))
    {
        Ok(f) => f,
        Err(e) => return Verrou::Impossible(e),
    };
    let debut = std::time::Instant::now();
    loop {
        match fichier.try_lock() {
            Ok(()) => return Verrou::Pris(fichier),
            Err(fs::TryLockError::WouldBlock) => {
                if debut.elapsed() >= patience {
                    return Verrou::Occupe;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(fs::TryLockError::Error(e)) => return Verrou::Impossible(e),
        }
    }
}

/// Ouvre ce dossier dans l'explorateur de fichiers du système.
pub fn ouvrir_dans_l_explorateur(chemin: &Path) -> io::Result<()> {
    let programme = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(programme)
        .arg(chemin)
        .spawn()
        .map(|_| ())
}

/// Dit à l'utilisateur, par une boîte de message du système, que l'application n'a pas pu
/// démarrer. Sous Windows seulement : ailleurs, la sortie d'erreur suffit.
pub fn annoncer_un_echec(texte: &str) {
    #[cfg(windows)]
    {
        use winsafe::co::MB;
        use winsafe::prelude::*;
        let _ =
            winsafe::HWND::NULL.MessageBox(texte, tr!("Computer Engineer"), MB::ICONERROR | MB::OK);
    }
    #[cfg(not(windows))]
    let _ = texte;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dossier_vide(nom: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ce-incident-{nom}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn une_seule_fenetre_par_dossier() {
        let d = dossier_vide("verrou");
        let court = std::time::Duration::from_millis(250);
        let Verrou::Pris(premier) = verrouiller(&d, court) else {
            panic!("le premier verrou doit se prendre");
        };
        assert!(matches!(verrouiller(&d, court), Verrou::Occupe));
        // Rendu (fin du programme, ou fermeture du fichier), il se reprend : y compris pendant
        // l'attente, comme au redémarrage après une mise à jour.
        let attente = std::thread::spawn({
            let d = d.clone();
            move || {
                matches!(
                    verrouiller(&d, std::time::Duration::from_secs(5)),
                    Verrou::Pris(_)
                )
            }
        });
        std::thread::sleep(std::time::Duration::from_millis(300));
        drop(premier);
        assert!(attente.join().unwrap());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn dates_connues() {
        let a = |s: u64| date_utc(UNIX_EPOCH + std::time::Duration::from_secs(s));
        assert_eq!(a(0), "1970-01-01 00:00:00");
        assert_eq!(a(951_782_400), "2000-02-29 00:00:00");
        assert_eq!(a(1_794_709_801), "2026-11-15 02:30:01");
        assert_eq!(a(4_107_542_399), "2100-02-28 23:59:59");
        assert_eq!(a(4_107_542_400), "2100-03-01 00:00:00");
    }

    #[test]
    fn civil_suit_les_jours_un_par_un() {
        // Chaque jour de 1970 à 2200 suit le précédent : même mois et jour + 1, ou premier du
        // mois suivant, ou 1er janvier de l'année suivante ; février a 29 jours exactement
        // les années bissextiles grégoriennes.
        let bissextile = |a: i64| (a % 4 == 0 && a % 100 != 0) || a % 400 == 0;
        let longueur = |a: i64, m: u32| match m {
            2 if bissextile(a) => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        let mut avant = civil(0);
        assert_eq!(avant, (1970, 1, 1));
        for z in 1..84_000 {
            let (a, m, j) = civil(z);
            let (pa, pm, pj) = avant;
            let attendu = if pj < longueur(pa, pm) {
                (pa, pm, pj + 1)
            } else if pm < 12 {
                (pa, pm + 1, 1)
            } else {
                (pa + 1, 1, 1)
            };
            assert_eq!((a, m, j), attendu, "jour {z}");
            avant = (a, m, j);
        }
    }

    #[test]
    fn rapports_ecrits_signales_puis_marques_et_elagues() {
        let d = dossier_vide("rapports");
        assert_eq!(rapport_non_lu(&d), None);
        let texte = texte_du_rapport("plantage", "Message : essai");
        assert!(texte.contains(env!("CARGO_PKG_VERSION")));
        let p = ecrire_un_rapport(&d, "plantage", &texte).unwrap();
        assert_eq!(fs::read_to_string(&p).unwrap(), texte);
        assert_eq!(rapport_non_lu(&d), Some(p.clone()));
        marquer_lus(&d);
        assert_eq!(rapport_non_lu(&d), None);
        assert!(!p.exists());
        for _ in 0..RAPPORTS_GARDES + 5 {
            ecrire_un_rapport(&d, "plantage", "x").unwrap();
        }
        assert_eq!(rapports(&d).len(), RAPPORTS_GARDES);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn une_panique_ecrit_son_rapport() {
        let d = dossier_vide("panique");
        let ecrit = std::thread::spawn({
            let d = d.clone();
            move || {
                let precedent = std::panic::take_hook();
                std::panic::set_hook(Box::new(move |info| {
                    let t = texte_du_rapport("plantage", &detail_de_panique(info));
                    let _ = ecrire_un_rapport(&d, "plantage", &t);
                }));
                let r = std::panic::catch_unwind(|| panic!("bascule introuvable {}", 42));
                std::panic::set_hook(precedent);
                r.is_err()
            }
        })
        .join()
        .unwrap();
        assert!(ecrit);
        // Une autre épreuve qui panique en même temps peut avoir écrit ici aussi : on cherche
        // le rapport de celle-ci parmi tous.
        let t = rapports(&d)
            .iter()
            .filter_map(|p| fs::read_to_string(p).ok())
            .find(|t| t.contains("bascule introuvable 42"))
            .expect("le rapport de la panique");
        assert!(t.contains("incident.rs"), "{t}");
        let _ = fs::remove_dir_all(&d);
    }
}
