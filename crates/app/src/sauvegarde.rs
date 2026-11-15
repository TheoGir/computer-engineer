//! Les projets enregistrés sur le disque.
//!
//! Un projet est un fichier JSON lisible, `projets/<nom>.json`, dans le dossier de données de
//! l'application. L'écriture passe par un fichier temporaire renommé à la fin : une coupure
//! de courant ne laisse jamais un projet à moitié écrit.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::de::IgnoredAny;
use serde::Deserialize;

use crate::brique::Projet;
use crate::format::Lecture;
use crate::parcours::Parcours;
use crate::reglages::Reglages;

/// Variable d'environnement qui remplace le dossier de données.
pub const VARIABLE_DOSSIER: &str = "CE_DOSSIER";

/// Le dossier où l'application range ses projets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dossier {
    racine: PathBuf,
}

/// Ce qu'on montre d'un projet enregistré dans la liste des projets, sans l'ouvrir en entier.
#[derive(Clone, Debug, PartialEq)]
pub struct FicheProjet {
    /// Nom de fichier, sans extension : ce qui l'identifie dans le dossier.
    pub fichier: String,
    /// Nom du projet.
    pub nom: String,
    /// Clé de la quête jouée dans ce projet, s'il y en a une.
    pub quete: Option<String>,
    /// Nombre d'éléments du plan principal.
    pub elements: usize,
    /// Nombre de briques du projet.
    pub briques: usize,
    /// Dernière modification du fichier.
    pub modifie: Option<SystemTime>,
    /// Vrai si le fichier n'a pas pu être lu.
    pub illisible: bool,
}

/// Le début d'un fichier de projet : ce qu'il faut pour la liste, le reste est sauté.
#[derive(Deserialize)]
struct Entete {
    nom: String,
    #[serde(default)]
    quete: Option<String>,
    #[serde(default)]
    modeles: Vec<IgnoredAny>,
    principal: EnteteDuPlan,
}

#[derive(Deserialize)]
struct EnteteDuPlan {
    #[serde(default)]
    elements: Vec<IgnoredAny>,
}

/// Longueur maximale d'un nom de fichier de projet, en caractères.
const LONGUEUR_NOM: usize = 80;
/// Et en octets : les systèmes de fichiers s'arrêtent à 255 octets par nom, extension et
/// suffixes (`.json.tmp`, numéro de la corbeille) compris.
const OCTETS_NOM: usize = 200;

/// Le début de `texte` qui tient en `caracteres` caractères et en `octets` octets.
fn debut(texte: &str, caracteres: usize, octets: usize) -> &str {
    let fin = texte
        .char_indices()
        .map(|(i, c)| i + c.len_utf8())
        .take(caracteres)
        .take_while(|fin| *fin <= octets)
        .last()
        .unwrap_or(0);
    &texte[..fin]
}

/// Le nom de fichier (sans extension) d'un projet : son nom, sans les caractères qu'un
/// système de fichiers refuse, coupé à 80 caractères (`LONGUEUR_NOM`) et 200 octets.
pub fn nom_de_fichier(nom: &str) -> String {
    let propre: String = nom
        .trim()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '\'' | '(' | ')') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let propre = debut(&propre, LONGUEUR_NOM, OCTETS_NOM);
    let propre = propre.trim().to_owned();
    if propre.is_empty() {
        "projet".to_owned()
    } else if reserve_par_windows(&propre) {
        format!("{propre}_")
    } else {
        propre
    }
}

/// Vrai pour un nom que Windows réserve à un périphérique, quelle que soit l'extension :
/// CON, PRN, AUX, NUL, COM0 à COM9, LPT0 à LPT9, et COM ou LPT suivis de ¹, ² ou ³.
fn reserve_par_windows(nom: &str) -> bool {
    let nom = nom.to_uppercase();
    if matches!(nom.as_str(), "CON" | "PRN" | "AUX" | "NUL") {
        return true;
    }
    let Some(reste) = nom.strip_prefix("COM").or_else(|| nom.strip_prefix("LPT")) else {
        return false;
    };
    let mut chiffre = reste.chars();
    matches!(
        (chiffre.next(), chiffre.next()),
        (Some('0'..='9' | '¹' | '²' | '³'), None)
    )
}

impl Dossier {
    /// Le dossier de données habituel du système : `CE_DOSSIER` s'il est défini, sinon
    /// `%APPDATA%\Computer Engineer` sous Windows, `~/Library/Application Support/Computer
    /// Engineer` sous macOS, `~/.local/share/computer-engineer` ailleurs.
    pub fn par_defaut() -> Option<Dossier> {
        let var = |nom: &str| {
            std::env::var_os(nom)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        };
        let racine = if let Some(force) = var(VARIABLE_DOSSIER) {
            force
        } else if cfg!(windows) {
            var("APPDATA")?.join("Computer Engineer")
        } else if cfg!(target_os = "macos") {
            var("HOME")?
                .join("Library")
                .join("Application Support")
                .join("Computer Engineer")
        } else {
            var("XDG_DATA_HOME")
                .or_else(|| var("HOME").map(|h| h.join(".local").join("share")))?
                .join("computer-engineer")
        };
        Some(Dossier { racine })
    }

    /// Un dossier de données à cet endroit.
    pub fn dans(racine: &Path) -> Dossier {
        Dossier {
            racine: racine.to_owned(),
        }
    }

    /// Où ce dossier se trouve.
    pub fn racine(&self) -> &Path {
        &self.racine
    }

    fn projets(&self) -> PathBuf {
        self.racine.join("projets")
    }

    fn chemin(&self, fichier: &str) -> PathBuf {
        self.projets().join(format!("{fichier}.json"))
    }

    /// Vrai si un projet est enregistré sous ce nom de fichier.
    pub fn existe(&self, fichier: &str) -> bool {
        self.chemin(fichier).is_file()
    }

    /// Les projets enregistrés, par ordre alphabétique.
    pub fn lister(&self) -> Vec<String> {
        let Ok(dossier) = fs::read_dir(self.projets()) else {
            return Vec::new();
        };
        let mut noms: Vec<String> = dossier
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            .filter_map(|p| p.file_stem()?.to_str().map(str::to_owned))
            .collect();
        noms.sort_by_key(|n| n.to_lowercase());
        noms
    }

    /// Enregistre le projet sous son nom. Renvoie le nom de fichier utilisé.
    pub fn enregistrer(&self, projet: &Projet) -> io::Result<String> {
        let fichier = nom_de_fichier(&projet.nom);
        self.enregistrer_sous(projet, &fichier)?;
        Ok(fichier)
    }

    /// Enregistre le projet dans ce fichier-là, quel que soit son nom : un projet rouvert reste
    /// chez lui tant qu'on ne le renomme pas, même si son fichier a un nom que
    /// [`nom_de_fichier`] n'aurait pas donné (copie faite dans l'explorateur de fichiers).
    pub fn enregistrer_sous(&self, projet: &Projet, fichier: &str) -> io::Result<()> {
        fs::create_dir_all(self.projets())?;
        let chemin = self.chemin(fichier);
        let provisoire = chemin.with_extension("json.tmp");
        fs::write(&provisoire, projet.en_json())?;
        fs::rename(&provisoire, &chemin)
    }

    /// Relit un projet enregistré. Un fichier renommé ou copié hors de l'application
    /// (« Additionneur - Copie.json » qui contient le projet « Additionneur ») prend le nom de
    /// son fichier : il s'enregistre chez lui, sans écraser le projet dont il porte le nom.
    pub fn ouvrir(&self, fichier: &str) -> Result<Projet, String> {
        let texte = fs::read_to_string(self.chemin(fichier)).map_err(|e| e.to_string())?;
        let (mut projet, lecture) = Projet::de_json_lu(&texte)?;
        if let Lecture::Migre(n) = lecture {
            self.garder_avant_migration(&format!("{fichier}.json"), n, &texte)
                .map_err(|e| tr!("copie de l'original avant migration impossible : {e}", e = e))?;
        }
        if nom_de_fichier(&projet.nom) != fichier {
            projet.nom = fichier.to_owned();
        }
        Ok(projet)
    }

    /// Le dossier où sont gardés les originaux des fichiers migrés vers un format plus récent.
    pub fn avant_migration(&self) -> PathBuf {
        self.racine.join("avant-migration")
    }

    /// Garde l'original d'un fichier `nom` au format `numero`, avant que sa version migrée ne
    /// le remplace : `avant-migration/<nom sans .json>.format-<numero>.json`. Une copie déjà
    /// gardée n'est pas remplacée (c'est la plus ancienne qui compte).
    fn garder_avant_migration(&self, nom: &str, numero: u32, texte: &str) -> io::Result<()> {
        let dossier = self.avant_migration();
        fs::create_dir_all(&dossier)?;
        let base = nom.strip_suffix(".json").unwrap_or(nom);
        let chemin = dossier.join(format!("{base}.format-{numero}.json"));
        if chemin.exists() {
            return Ok(());
        }
        let provisoire = chemin.with_extension("json.tmp");
        fs::write(&provisoire, texte)?;
        fs::rename(&provisoire, &chemin)
    }

    /// Retire le fichier d'un projet.
    pub fn retirer(&self, fichier: &str) -> io::Result<()> {
        fs::remove_file(self.chemin(fichier))
    }

    /// Enregistre sous son nom un projet qui était enregistré sous le fichier `ancien`, qui
    /// disparaît. Si le nom est déjà pris par un autre projet, `projet.nom` devient un nom libre
    /// proche. Renvoie le nom de fichier utilisé.
    ///
    /// Le fichier est d'abord renommé, puis réécrit : il n'y a jamais deux fichiers pour un
    /// projet, même si l'application est coupée entre les deux (le projet garde alors son
    /// contenu d'avant sous son nouveau nom de fichier). Si l'écriture échoue, il reprend son
    /// ancien nom de fichier.
    pub fn deplacer(&self, projet: &mut Projet, ancien: &str) -> io::Result<String> {
        let mut voulu = nom_de_fichier(&projet.nom);
        if voulu != ancien && self.pris_par_un_autre(&voulu, ancien) {
            projet.nom = self.nom_libre(&projet.nom);
            voulu = nom_de_fichier(&projet.nom);
        }
        let (de, vers) = (self.chemin(ancien), self.chemin(&voulu));
        let deplace = voulu != ancien
            && match fs::rename(&de, &vers) {
                Ok(()) => true,
                // Jamais enregistré sous l'ancien nom : il n'y a rien à déplacer.
                Err(e) if e.kind() == io::ErrorKind::NotFound => false,
                Err(e) => return Err(e),
            };
        self.enregistrer(projet).inspect_err(|_| {
            if deplace {
                let _ = fs::rename(&vers, &de);
            }
        })
    }

    /// Vrai si le fichier `voulu` existe et appartient à un autre projet que `ancien`.
    ///
    /// Sous Windows et macOS, « Brouillon.json » et « brouillon.json » sont le même fichier :
    /// quand les deux noms ne diffèrent que par la casse, `voulu` n'est un autre projet que si
    /// le dossier liste bien un fichier écrit exactement ainsi.
    fn pris_par_un_autre(&self, voulu: &str, ancien: &str) -> bool {
        self.existe(voulu)
            && (voulu.to_lowercase() != ancien.to_lowercase()
                || self.lister().iter().any(|n| n == voulu))
    }

    /// Le projet ouvert la dernière fois, s'il existe encore.
    pub fn dernier(&self) -> Option<String> {
        self.dernier_note().filter(|nom| self.existe(nom))
    }

    /// Le nom noté dans `dernier.txt`, que le projet existe encore ou non.
    fn dernier_note(&self) -> Option<String> {
        let nom = fs::read_to_string(self.racine.join("dernier.txt")).ok()?;
        Some(nom.trim().to_owned())
    }

    /// Retient le projet ouvert, pour le rouvrir au prochain lancement.
    pub fn noter_dernier(&self, fichier: &str) -> io::Result<()> {
        fs::create_dir_all(&self.racine)?;
        fs::write(self.racine.join("dernier.txt"), fichier)
    }

    /// Les projets enregistrés avec ce qu'il faut pour les présenter, le plus récent d'abord.
    pub fn fiches(&self) -> Vec<FicheProjet> {
        let mut fiches: Vec<FicheProjet> = self
            .lister()
            .into_iter()
            .map(|fichier| {
                let chemin = self.chemin(&fichier);
                let modifie = fs::metadata(&chemin).and_then(|m| m.modified()).ok();
                let entete = fs::read_to_string(&chemin)
                    .ok()
                    .and_then(|t| serde_json::from_str::<Entete>(&t).ok());
                match entete {
                    Some(e) => FicheProjet {
                        // Le nom sous lequel il s'ouvrira (voir `ouvrir`).
                        nom: if nom_de_fichier(&e.nom) == fichier {
                            e.nom
                        } else {
                            fichier.clone()
                        },
                        quete: e.quete,
                        elements: e.principal.elements.len(),
                        briques: e.modeles.len(),
                        modifie,
                        illisible: false,
                        fichier,
                    },
                    None => FicheProjet {
                        nom: fichier.clone(),
                        quete: None,
                        elements: 0,
                        briques: 0,
                        modifie,
                        illisible: true,
                        fichier,
                    },
                }
            })
            .collect();
        fiches.sort_by(|a, b| {
            b.modifie
                .cmp(&a.modifie)
                .then_with(|| a.nom.to_lowercase().cmp(&b.nom.to_lowercase()))
        });
        fiches
    }

    /// Renomme un projet enregistré : son nom dans le fichier et le fichier lui-même. Si le
    /// nom est déjà pris par un autre projet, un nom libre proche est choisi. Renvoie le nouveau
    /// nom de fichier.
    pub fn renommer(&self, fichier: &str, nom: &str) -> Result<String, String> {
        let nom = nom.trim();
        if nom.is_empty() {
            return Err(tr!("un projet doit avoir un nom").to_owned());
        }
        let mut projet = self.ouvrir(fichier)?;
        if projet.nom == nom {
            // Le champ de saisie validé sans rien changer : le fichier ne bouge pas.
            return Ok(fichier.to_owned());
        }
        projet.nom = nom.to_owned();
        let nouveau = self
            .deplacer(&mut projet, fichier)
            .map_err(|e| e.to_string())?;
        if self.dernier_note().as_deref() == Some(fichier) {
            // C'était le dernier projet ouvert : il le reste sous son nouveau nom.
            let _ = self.noter_dernier(&nouveau);
        }
        Ok(nouveau)
    }

    /// Enregistre une copie d'un projet sous un nom libre (« Nom (copie) »). Renvoie le nom de
    /// fichier de la copie.
    pub fn dupliquer(&self, fichier: &str) -> Result<String, String> {
        let mut projet = self.ouvrir(fichier)?;
        projet.nom = self.nom_libre(&format!("{} (copie)", projet.nom));
        self.enregistrer(&projet).map_err(|e| e.to_string())
    }

    fn corbeille(&self) -> PathBuf {
        self.racine.join("corbeille")
    }

    /// Met un projet à la corbeille, un dossier à côté des projets : il disparaît de la liste,
    /// mais [`Dossier::restaurer`] le ramène. Un projet jeté plus tôt sous le même nom reste dans
    /// la corbeille : celui-ci y prend un nom libre (« Nom 2 »…). Renvoie son nom dans la
    /// corbeille.
    pub fn jeter(&self, fichier: &str) -> io::Result<String> {
        let corbeille = self.corbeille();
        fs::create_dir_all(&corbeille)?;
        let place = std::iter::once(fichier.to_owned())
            .chain((2..).map(|k| format!("{fichier} {k}")))
            .find(|n| !corbeille.join(format!("{n}.json")).exists())
            .expect("il reste toujours une place dans la corbeille");
        fs::rename(
            self.chemin(fichier),
            corbeille.join(format!("{place}.json")),
        )?;
        Ok(place)
    }

    /// Ramène de la corbeille le projet rangé sous `jete` (le nom que [`Dossier::jeter`] a
    /// renvoyé) à sa place d'avant, `fichier`. Si un projet de ce nom est apparu entre-temps,
    /// celui de la corbeille revient sous un nom libre. Renvoie le nom de fichier retrouvé.
    pub fn restaurer(&self, jete: &str, fichier: &str) -> Result<String, String> {
        let source = self.corbeille().join(format!("{jete}.json"));
        if !self.existe(fichier) {
            fs::create_dir_all(self.projets()).map_err(|e| e.to_string())?;
            fs::rename(&source, self.chemin(fichier)).map_err(|e| e.to_string())?;
            return Ok(fichier.to_owned());
        }
        let texte = fs::read_to_string(&source).map_err(|e| e.to_string())?;
        let nouveau = match Projet::de_json(&texte) {
            Ok(mut projet) => {
                projet.nom = self.nom_libre(&projet.nom);
                self.enregistrer(&projet).map_err(|e| e.to_string())?
            }
            // Illisible : il revient tel quel, sous un nom de fichier libre.
            Err(_) => {
                let libre = nom_de_fichier(&self.nom_libre(fichier));
                fs::rename(&source, self.chemin(&libre)).map_err(|e| e.to_string())?;
                return Ok(libre);
            }
        };
        let _ = fs::remove_file(source);
        Ok(nouveau)
    }

    /// Relit l'avancement dans le parcours des quêtes. `Ok(None)` s'il n'y en a pas encore.
    pub fn parcours(&self) -> Result<Option<Parcours>, String> {
        match fs::read_to_string(self.racine.join("parcours.json")) {
            Ok(texte) => {
                let (parcours, lecture) = Parcours::de_json_lu(&texte)?;
                if let Lecture::Migre(n) = lecture {
                    self.garder_avant_migration("parcours.json", n, &texte)
                        .map_err(|e| {
                            tr!("copie de l'original avant migration impossible : {e}", e = e)
                        })?;
                }
                Ok(Some(parcours))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Met de côté un `parcours.json` qui n'a pas pu être relu (abîmé, écrit par une version
    /// plus récente), pour que l'avancement de la séance ne l'écrase pas. Renvoie le nom du
    /// fichier gardé.
    pub fn garder_le_parcours_illisible(&self) -> io::Result<String> {
        let garde = std::iter::once("parcours-illisible.json".to_owned())
            .chain((2..).map(|k| format!("parcours-illisible {k}.json")))
            .find(|n| !self.racine.join(n).exists())
            .expect("il reste toujours un nom libre");
        fs::rename(self.racine.join("parcours.json"), self.racine.join(&garde))?;
        Ok(garde)
    }

    /// Met de côté l'avancement actuel avant de recommencer le parcours, sous un nom libre
    /// (`parcours-avant-recommencement.json`, puis `… 2.json`…). Renvoie ce nom, ou `None`
    /// s'il n'y avait pas d'avancement enregistré.
    pub fn garder_le_parcours_avant_recommencement(&self) -> io::Result<Option<String>> {
        let actuel = self.racine.join("parcours.json");
        if !actuel.exists() {
            return Ok(None);
        }
        let garde = std::iter::once("parcours-avant-recommencement.json".to_owned())
            .chain((2..).map(|k| format!("parcours-avant-recommencement {k}.json")))
            .find(|n| !self.racine.join(n).exists())
            .expect("un nom libre");
        fs::copy(&actuel, self.racine.join(&garde))?;
        Ok(Some(garde))
    }

    /// Enregistre l'avancement dans le parcours des quêtes.
    pub fn enregistrer_parcours(&self, parcours: &Parcours) -> io::Result<()> {
        fs::create_dir_all(&self.racine)?;
        let chemin = self.racine.join("parcours.json");
        let provisoire = chemin.with_extension("json.tmp");
        fs::write(&provisoire, parcours.en_json())?;
        fs::rename(&provisoire, &chemin)
    }

    /// Les réglages de l'utilisateur, dans `reglages.json`. Voir [`Dossier::reglages_lus`].
    pub fn reglages(&self) -> Reglages {
        self.reglages_lus().0
    }

    /// Les réglages de l'utilisateur, et `false` si une partie du fichier n'a pas été comprise.
    ///
    /// Le fichier est écrit avec les valeurs par défaut s'il n'existe pas encore, pour qu'on le
    /// trouve et le modifie. Un réglage illisible garde sa valeur par défaut sans emporter les
    /// autres ([`Reglages::lire`]) ; un fichier qu'on ne peut pas lire (droits, verrou) n'est
    /// pas écrasé.
    pub fn reglages_lus(&self) -> (Reglages, bool) {
        let chemin = self.racine.join("reglages.json");
        match fs::read(&chemin) {
            Ok(octets) => Reglages::lire(&texte_de(&octets)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let reglages = Reglages::default();
                let _ = fs::create_dir_all(&self.racine)
                    .and_then(|()| fs::write(&chemin, reglages.en_json()));
                (reglages, true)
            }
            Err(_) => (Reglages::default(), false),
        }
    }

    /// Enregistre les réglages de l'utilisateur.
    pub fn enregistrer_reglages(&self, reglages: &Reglages) -> io::Result<()> {
        fs::create_dir_all(&self.racine)?;
        let chemin = self.racine.join("reglages.json");
        let provisoire = chemin.with_extension("json.tmp");
        fs::write(&provisoire, reglages.en_json())?;
        fs::rename(&provisoire, &chemin)
    }

    /// Un nom de projet qui n'est pas encore pris : `base`, sinon `base 2`, `base 3`… Un nom
    /// trop long pour un fichier est raccourci avant le numéro, pour que le numéro compte.
    pub fn nom_libre(&self, base: &str) -> String {
        let base = base.trim();
        if !self.existe(&nom_de_fichier(base)) {
            return base.to_owned();
        }
        (2u64..)
            .map(|k| {
                let numero = format!(" {k}");
                let tete = debut(base, LONGUEUR_NOM - numero.len(), OCTETS_NOM - numero.len());
                format!("{}{numero}", tete.trim_end())
            })
            .find(|n| !self.existe(&nom_de_fichier(n)))
            .expect("il reste toujours un nom libre")
    }
}

/// Le texte d'un fichier retouché à la main : UTF-8, ou UTF-16 avec sa marque d'ordre des
/// octets (ce qu'écrivent le Bloc-notes « Unicode » et PowerShell 5).
fn texte_de(octets: &[u8]) -> String {
    let utf16 = |reste: &[u8], lire: fn([u8; 2]) -> u16| {
        let unites: Vec<u16> = reste.as_chunks::<2>().0.iter().map(|c| lire(*c)).collect();
        String::from_utf16_lossy(&unites)
    };
    match octets {
        [0xFF, 0xFE, reste @ ..] => utf16(reste, u16::from_le_bytes),
        [0xFE, 0xFF, reste @ ..] => utf16(reste, u16::from_be_bytes),
        _ => String::from_utf8_lossy(octets).into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    fn dossier(nom: &str) -> Dossier {
        let racine =
            std::env::temp_dir().join(format!("ce-sauvegarde-{nom}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&racine);
        Dossier::dans(&racine)
    }

    #[test]
    fn un_projet_enregistre_se_relit_a_l_identique() {
        let d = dossier("relire");
        assert!(d.lister().is_empty());
        assert_eq!(d.dernier(), None);
        let mut projet = Projet::nouveau("Mon additionneur", Document::demi_additionneur());
        projet.quete = Some("demi-additionneur".to_owned());
        let fichier = d.enregistrer(&projet).unwrap();
        assert_eq!(fichier, "Mon additionneur");
        assert_eq!(d.lister(), ["Mon additionneur"]);
        let relu = d.ouvrir(&fichier).unwrap();
        assert_eq!(relu.nom, projet.nom);
        assert_eq!(relu.quete, projet.quete);
        assert_eq!(relu.principal, projet.principal);
        d.noter_dernier(&fichier).unwrap();
        assert_eq!(d.dernier().as_deref(), Some("Mon additionneur"));
        assert_eq!(d.nom_libre("Mon additionneur"), "Mon additionneur 2");
        // Enregistrer de nouveau remplace le fichier, sans laisser de fichier provisoire.
        d.enregistrer(&projet).unwrap();
        assert_eq!(fs::read_dir(d.projets()).unwrap().count(), 1);
        d.retirer(&fichier).unwrap();
        assert!(d.lister().is_empty());
        assert_eq!(d.dernier(), None, "le dernier projet n'existe plus");
        let _ = fs::remove_dir_all(d.racine());
    }

    #[test]
    fn l_ordinateur_complet_survit_a_l_enregistrement() {
        use crate::document::Genre;
        use crate::simulation::Session;
        let d = dossier("ordinateur");
        let exemple = &crate::exemples::EXEMPLES[0];
        let projet = crate::exemples::ordinateur(exemple.nom, exemple.programme).unwrap();
        let fichier = d.enregistrer(&projet).unwrap();
        let relu = d.ouvrir(&fichier).unwrap();
        assert_eq!(relu.modeles.len(), projet.modeles.len());
        let trouver = |genre: Genre| {
            relu.principal
                .elements
                .iter()
                .find(|e| e.genre == genre)
                .unwrap()
                .id
        };
        let mut s = Session::nouvelle(&relu.principal, &relu, &[]);
        assert_eq!(s.erreur(), None);
        s.taper(trouver(Genre::Clavier), b'6');
        s.taper(trouver(Genre::Clavier), b'7');
        s.avancer_vite(128 * 400);
        assert_eq!(s.ecran(trouver(Genre::Ecran)), "6*7=42");
        let _ = fs::remove_dir_all(d.racine());
    }

    #[test]
    fn changer_seulement_la_casse_garde_le_projet() {
        // Sous Windows et macOS, « Brouillon.json » et « brouillon.json » sont le même fichier :
        // renommer ne doit ni l'effacer ni inventer « brouillon 2 ».
        let d = dossier("casse");
        let doc = Document::demi_additionneur();
        let a = d
            .enregistrer(&Projet::nouveau("Brouillon", doc.clone()))
            .unwrap();
        d.noter_dernier(&a).unwrap();
        let b = d.renommer(&a, "brouillon").unwrap();
        assert_eq!(b, "brouillon");
        assert_eq!(d.lister(), vec!["brouillon".to_owned()]);
        let relu = d.ouvrir(&b).unwrap();
        assert_eq!(relu.nom, "brouillon");
        assert_eq!(relu.principal.elements.len(), doc.elements.len());
        assert_eq!(d.dernier().as_deref(), Some("brouillon"));
        let _ = fs::remove_dir_all(d.racine());
    }

    #[test]
    fn renommer_dupliquer_jeter_et_restaurer() {
        let d = dossier("gestion");
        let mut projet = Projet::nouveau("Additionneur", Document::demi_additionneur());
        projet.quete = Some("demi-additionneur".to_owned());
        let a = d.enregistrer(&projet).unwrap();
        d.noter_dernier(&a).unwrap();
        d.enregistrer(&Projet::nouveau("Mémoire", Document::nouveau()))
            .unwrap();

        // La liste : le nom, la quête, le nombre d'éléments, sans ouvrir tout le projet.
        let fiches = d.fiches();
        assert_eq!(fiches.len(), 2);
        let f = fiches.iter().find(|f| f.fichier == "Additionneur").unwrap();
        assert_eq!(f.quete.as_deref(), Some("demi-additionneur"));
        assert_eq!(f.elements, projet.principal.elements.len());
        assert!(f.modifie.is_some() && !f.illisible);

        // Renommer : le fichier suit, le nom dans le fichier aussi, le dernier projet aussi.
        let b = d.renommer(&a, "Mon additionneur").unwrap();
        assert_eq!(b, "Mon additionneur");
        assert!(!d.existe(&a));
        assert_eq!(d.ouvrir(&b).unwrap().nom, "Mon additionneur");
        assert_eq!(d.dernier().as_deref(), Some("Mon additionneur"));
        // Vers un nom déjà pris : un nom libre.
        assert_eq!(d.renommer(&b, "Mémoire").unwrap(), "Mémoire 2");
        assert!(d.renommer("Mémoire 2", "  ").is_err());

        // Dupliquer.
        let copie = d.dupliquer("Mémoire").unwrap();
        assert_eq!(copie, "Mémoire (copie)");
        assert_eq!(d.lister().len(), 3);

        // Jeter puis restaurer.
        let jete = d.jeter("Mémoire").unwrap();
        assert!(!d.existe("Mémoire"));
        assert_eq!(d.restaurer(&jete, "Mémoire").unwrap(), "Mémoire");
        assert!(d.existe("Mémoire"));
        // Restaurer quand le nom a été repris entre-temps : un nom libre.
        let jete = d.jeter("Mémoire").unwrap();
        d.enregistrer(&Projet::nouveau("Mémoire", Document::nouveau()))
            .unwrap();
        let revenu = d.restaurer(&jete, "Mémoire").unwrap();
        assert_eq!(revenu, "Mémoire 3");

        // Un fichier abîmé reste listé, marqué illisible.
        fs::write(d.chemin("abîmé"), "{ pas du json").unwrap();
        assert!(d
            .fiches()
            .iter()
            .any(|f| f.fichier == "abîmé" && f.illisible));
        let _ = fs::remove_dir_all(d.racine());
    }

    #[test]
    fn deux_projets_jetes_sous_le_meme_nom_restent_dans_la_corbeille() {
        let d = dossier("corbeille");
        let mut premier = Projet::nouveau("Brouillon", Document::demi_additionneur());
        premier.quete = Some("demi-additionneur".to_owned());
        d.enregistrer(&premier).unwrap();
        let a = d.jeter("Brouillon").unwrap();
        d.enregistrer(&Projet::nouveau("Brouillon", Document::nouveau()))
            .unwrap();
        let b = d.jeter("Brouillon").unwrap();
        assert_ne!(a, b, "le second n'écrase pas le premier");
        assert_eq!(fs::read_dir(d.corbeille()).unwrap().count(), 2);
        // Chacun revient tel qu'il était.
        assert_eq!(d.restaurer(&a, "Brouillon").unwrap(), "Brouillon");
        assert_eq!(d.ouvrir("Brouillon").unwrap().quete, premier.quete);
        let revenu = d.restaurer(&b, "Brouillon").unwrap();
        assert_eq!(revenu, "Brouillon 2");
        assert!(d.ouvrir(&revenu).unwrap().principal.elements.is_empty());
        // Un projet illisible revient lui aussi, tel quel.
        fs::write(d.chemin("abîmé"), "{ pas du json").unwrap();
        let c = d.jeter("abîmé").unwrap();
        fs::write(d.chemin("abîmé"), "{}").unwrap();
        let revenu = d.restaurer(&c, "abîmé").unwrap();
        assert_eq!(
            fs::read_to_string(d.chemin(&revenu)).unwrap(),
            "{ pas du json"
        );
        let _ = fs::remove_dir_all(d.racine());
    }

    #[test]
    fn les_noms_tres_longs_trouvent_un_nom_libre() {
        // Le nom de fichier est coupé à 80 caractères : « nom 2 » doit rester distinct de
        // « nom », sinon la recherche d'un nom libre ne s'arrête jamais.
        let d = dossier("longs");
        let long = "Additionneur ".repeat(10);
        let a = d
            .enregistrer(&Projet::nouveau(&long, Document::demi_additionneur()))
            .unwrap();
        assert_eq!(a.chars().count(), 80);
        let libre = d.nom_libre(&long);
        assert_ne!(nom_de_fichier(&libre), a);
        assert!(libre.ends_with(" 2"));
        let copie = d.dupliquer(&a).unwrap();
        assert_ne!(copie, a);
        assert_eq!(d.lister().len(), 2);
        let encore = d.dupliquer(&a).unwrap();
        assert_eq!(d.lister().len(), 3);
        assert!(encore.ends_with(" 3"), "{encore}");
        // Renommer vers un nom long déjà pris.
        d.enregistrer(&Projet::nouveau("court", Document::nouveau()))
            .unwrap();
        let renomme = d.renommer("court", &long).unwrap();
        assert_eq!(d.lister().len(), 4);
        assert!(renomme.ends_with(" 4"), "{renomme}");
        let _ = fs::remove_dir_all(d.racine());
    }

    #[test]
    fn un_fichier_copie_hors_de_l_application_prend_le_nom_de_son_fichier() {
        let d = dossier("copie");
        let f = d
            .enregistrer(&Projet::nouveau(
                "Additionneur",
                Document::demi_additionneur(),
            ))
            .unwrap();
        fs::copy(d.chemin(&f), d.chemin("Additionneur - Copie")).unwrap();
        let copie = d.ouvrir("Additionneur - Copie").unwrap();
        assert_eq!(copie.nom, "Additionneur - Copie");
        assert_eq!(d.ouvrir(&f).unwrap().nom, "Additionneur");
        let fiches = d.fiches();
        assert!(fiches
            .iter()
            .any(|x| x.fichier == "Additionneur - Copie" && x.nom == "Additionneur - Copie"));
        // Un nom que le nom de fichier abrège ou remplace reste tel quel.
        let long = "Additionneur ".repeat(8);
        let g = d
            .enregistrer(&Projet::nouveau("a/b : c", Document::nouveau()))
            .unwrap();
        assert_eq!(d.ouvrir(&g).unwrap().nom, "a/b : c");
        let h = d
            .enregistrer(&Projet::nouveau(&long, Document::nouveau()))
            .unwrap();
        assert_eq!(d.ouvrir(&h).unwrap().nom, long);
        let _ = fs::remove_dir_all(d.racine());
    }

    #[test]
    fn un_renommage_ne_laisse_jamais_deux_fichiers() {
        let d = dossier("renommage");
        let mut projet = Projet::nouveau("Ancien nom", Document::demi_additionneur());
        let f = d.enregistrer(&projet).unwrap();
        d.enregistrer(&Projet::nouveau("Pris", Document::nouveau()))
            .unwrap();
        // Vers un nom libre : un seul fichier, rien d'autre dans le dossier.
        projet.nom = "Nouveau nom".to_owned();
        assert_eq!(d.deplacer(&mut projet, &f).unwrap(), "Nouveau nom");
        assert_eq!(d.lister(), ["Nouveau nom", "Pris"]);
        assert_eq!(fs::read_dir(d.projets()).unwrap().count(), 2);
        // Vers un nom pris : un nom libre, et l'autre projet reste intact.
        projet.nom = "Pris".to_owned();
        assert_eq!(d.deplacer(&mut projet, "Nouveau nom").unwrap(), "Pris 2");
        assert_eq!(projet.nom, "Pris 2");
        assert!(d.ouvrir("Pris").unwrap().principal.elements.is_empty());
        assert_eq!(d.lister(), ["Pris", "Pris 2"]);
        // Coupure entre le renommage du fichier et sa réécriture : le projet est là, une seule
        // fois, sous le nom de son fichier.
        fs::rename(d.chemin("Pris 2"), d.chemin("Coupé")).unwrap();
        assert_eq!(d.lister(), ["Coupé", "Pris"]);
        let relu = d.ouvrir("Coupé").unwrap();
        assert_eq!(relu.nom, "Coupé");
        assert_eq!(
            relu.principal.elements.len(),
            projet.principal.elements.len()
        );
        // Un projet jamais enregistré s'écrit simplement sous son nom.
        let mut neuf = Projet::nouveau("Neuf", Document::nouveau());
        assert_eq!(d.deplacer(&mut neuf, "Jamais écrit").unwrap(), "Neuf");
        // Renommer sans rien changer ne touche pas au fichier, même si son nom n'est pas celui
        // que l'application aurait donné.
        fs::rename(d.chemin("Neuf"), d.chemin("Neuf, v2.0")).unwrap();
        let nom = d.ouvrir("Neuf, v2.0").unwrap().nom;
        assert_eq!(d.renommer("Neuf, v2.0", &nom).unwrap(), "Neuf, v2.0");
        assert!(d.existe("Neuf, v2.0"));
        let _ = fs::remove_dir_all(d.racine());
    }

    #[test]
    fn un_parcours_illisible_est_garde_de_cote() {
        let d = dossier("parcours-illisible");
        fs::create_dir_all(d.racine()).unwrap();
        for attendu in ["parcours-illisible.json", "parcours-illisible 2.json"] {
            fs::write(d.racine().join("parcours.json"), attendu).unwrap();
            assert!(d.parcours().is_err());
            assert_eq!(d.garder_le_parcours_illisible().unwrap(), attendu);
            assert_eq!(
                fs::read_to_string(d.racine().join(attendu)).unwrap(),
                attendu
            );
            assert_eq!(d.parcours(), Ok(None));
        }
        let _ = fs::remove_dir_all(d.racine());
    }

    #[test]
    fn recommencer_le_parcours_garde_une_copie_sans_rien_ecraser() {
        let d = dossier("recommencer");
        fs::create_dir_all(d.racine()).unwrap();
        assert_eq!(d.garder_le_parcours_avant_recommencement().unwrap(), None);
        for (texte, attendu) in [
            ("premier", "parcours-avant-recommencement.json"),
            ("second", "parcours-avant-recommencement 2.json"),
        ] {
            fs::write(d.racine().join("parcours.json"), texte).unwrap();
            assert_eq!(
                d.garder_le_parcours_avant_recommencement().unwrap(),
                Some(attendu.to_owned())
            );
            assert_eq!(fs::read_to_string(d.racine().join(attendu)).unwrap(), texte);
        }
        assert_eq!(
            fs::read_to_string(d.racine().join("parcours-avant-recommencement.json")).unwrap(),
            "premier"
        );
        let _ = fs::remove_dir_all(d.racine());
    }

    #[test]
    fn un_nom_en_lettres_larges_tient_dans_un_nom_de_fichier() {
        // 80 caractères de 4 octets feraient 320 octets : plus que ce qu'un nom de fichier admet.
        let d = dossier("octets");
        let long = "𝐀".repeat(80);
        let fichier = nom_de_fichier(&long);
        assert!(fichier.len() <= 200 && fichier.chars().all(|c| c == '𝐀'));
        let a = d
            .enregistrer(&Projet::nouveau(&long, Document::nouveau()))
            .unwrap();
        let copie = d.dupliquer(&a).unwrap();
        assert_ne!(copie, a);
        assert!(copie.len() <= 200 && copie.ends_with(" 2"), "{copie}");
        assert_eq!(d.lister().len(), 2);
        let _ = fs::remove_dir_all(d.racine());
    }

    #[test]
    fn des_reglages_illisibles_ne_sont_pas_ecrases() {
        let d = dossier("reglages-illisibles");
        fs::create_dir_all(d.racine()).unwrap();
        let chemin = d.racine().join("reglages.json");
        fs::write(
            &chemin,
            "\u{feff}{ \"mises_a_jour_automatiques\": false, \"theme\": \"clair\", \"taille_texte\": \"énorme\" }",
        )
        .unwrap();
        let r = d.reglages();
        assert!(
            !r.mises_a_jour_automatiques,
            "le choix de l'utilisateur reste"
        );
        assert_eq!(r.theme, Some(crate::apparence::Mode::Clair));
        assert_eq!(r.taille_texte, 1.0);
        assert!(fs::read_to_string(&chemin).unwrap().contains("énorme"));
        // Un fichier dans un autre encodage n'est pas remplacé par les valeurs par défaut.
        let utf16: Vec<u8> = "{ \"theme\": \"clair\" }"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        fs::write(&chemin, &utf16).unwrap();
        assert_eq!(d.reglages(), Reglages::default(), "sans marque : illisible");
        assert_eq!(fs::read(&chemin).unwrap(), utf16);
        // Avec sa marque d'ordre des octets (Bloc-notes « Unicode », PowerShell), il se lit.
        let marque: Vec<u8> = [0xFF, 0xFE].into_iter().chain(utf16).collect();
        fs::write(&chemin, &marque).unwrap();
        let (r, complet) = d.reglages_lus();
        assert_eq!(r.theme, Some(crate::apparence::Mode::Clair));
        assert!(complet);
        // Une faute de frappe (virgule en trop, mot sans guillemets, virgule décimale) ne remet
        // pas tout à zéro, et la recherche des mises à jour reste coupée.
        fs::write(
            &chemin,
            "{\n  \"mises_a_jour_automatiques\": false,\n  \"theme\": clair,\n  \"taille_texte\": 1,3,\n  \"vitesse\": deux,\n}\n",
        )
        .unwrap();
        let (r, complet) = d.reglages_lus();
        assert!(!r.mises_a_jour_automatiques);
        assert_eq!(r.theme, Some(crate::apparence::Mode::Clair));
        assert!((r.taille_texte - 1.3).abs() < 1e-6);
        assert_eq!(r.vitesse, None);
        assert!(!complet, "« deux » n'est pas une vitesse : on le signale");
        fs::write(
            &chemin,
            "{ \"mises_a_jour_automatiques\": false, \"theme\": \"clair\", }",
        )
        .unwrap();
        let (r, _) = d.reglages_lus();
        assert!(!r.mises_a_jour_automatiques);
        assert_eq!(r.theme, Some(crate::apparence::Mode::Clair));
        let _ = fs::remove_dir_all(d.racine());
    }

    #[test]
    fn les_reglages_s_enregistrent() {
        let d = dossier("reglages");
        let mut r = d.reglages();
        assert_eq!(r, Reglages::default());
        r.taille_texte = 1.15;
        r.au_lancement = crate::reglages::Lancement::Reprendre;
        d.enregistrer_reglages(&r).unwrap();
        assert_eq!(d.reglages(), r);
        let _ = fs::remove_dir_all(d.racine());
    }

    #[test]
    fn les_noms_de_fichier_sont_acceptes_partout() {
        assert_eq!(nom_de_fichier("  Mon circuit  "), "Mon circuit");
        assert_eq!(nom_de_fichier("a/b\\c:d*e?"), "a_b_c_d_e_");
        assert_eq!(nom_de_fichier("Mémoire 8 bits (v2)"), "Mémoire 8 bits (v2)");
        assert_eq!(nom_de_fichier(""), "projet");
        assert_eq!(nom_de_fichier("nul"), "nul_");
        for reserve in ["com0", "COM9", "Lpt5", "COM¹", "lpt³", "aux"] {
            assert_eq!(nom_de_fichier(reserve), format!("{reserve}_"));
        }
        for libre in ["COM", "COM10", "LPT 1", "console", "COMa"] {
            assert_eq!(nom_de_fichier(libre), libre);
        }
        assert_eq!(
            nom_de_fichier("..").chars().filter(|c| *c == '.').count(),
            0
        );
    }
}
