//! Mises à jour automatiques, confiées à des bibliothèques éprouvées : `self_update` cherche la
//! dernière version publiée sur GitHub, la télécharge, vérifie sa signature Ed25519 (`zipsign`)
//! avec les clés publiques compilées dans l'application, puis remplace l'exécutable
//! (`self-replace`). Rien n'est remplacé si la signature n'est pas valide.
//!
//! Ce module ne fait que configurer `self_update` et faire tourner ses appels hors de l'image,
//! dans un fil d'exécution à part : l'application ne se bloque jamais et fonctionne sans réseau.
//! Conception complète : `docs/mises-a-jour.md`.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

pub use self_update::VerifyingKey;

/// Nom de l'exécutable dans l'archive publiée (`.exe` ajouté sous Windows).
pub const EXECUTABLE: &str = "computer-engineer";

/// Le dépôt GitHub public d'où viennent les versions, « propriétaire/nom ». Il ne contient que les
/// versions publiées, séparées du dépôt du code.
/// La variable `CE_DEPOT_DES_VERSIONS`, à la compilation, en choisit un autre.
pub const DEPOT_DES_VERSIONS: &str = match option_env!("CE_DEPOT_DES_VERSIONS") {
    Some(depot) => depot,
    None => "TheoGir/computer-engineer-releases",
};

/// Les clés publiques qui signent les versions : deux emplacements, pour pouvoir changer de clé
/// sans rendre muettes les versions déjà installées. Un fichier vide est un emplacement libre.
const FICHIERS_DE_CLES: [&[u8]; 2] = [
    include_bytes!("../cles/verification-1.pub"),
    include_bytes!("../cles/verification-2.pub"),
];

/// Les clés publiques compilées dans l'application.
pub fn cles_officielles() -> Vec<VerifyingKey> {
    FICHIERS_DE_CLES
        .iter()
        .filter_map(|octets| VerifyingKey::try_from(*octets).ok())
        .collect()
}

/// D'où viennent les versions, et comment les vérifier.
#[derive(Clone, Debug)]
pub struct Source {
    /// Propriétaire du dépôt GitHub.
    pub proprietaire: String,
    /// Nom du dépôt.
    pub depot: String,
    /// Adresse de l'API, si ce n'est pas celle de GitHub (bancs d'essai).
    pub api: Option<String>,
    /// Clés publiques acceptées. Vide : aucune mise à jour n'est possible.
    pub cles: Vec<VerifyingKey>,
    /// Version installée.
    pub version: String,
    /// L'exécutable à remplacer ; `None` : celui qui tourne.
    pub cible: Option<PathBuf>,
}

impl Source {
    /// La source officielle : le dépôt des versions, les clés compilées, l'exécutable courant.
    pub fn officielle() -> Source {
        let (proprietaire, depot) = DEPOT_DES_VERSIONS
            .split_once('/')
            .unwrap_or(("TheoGir", "computer-engineer-releases"));
        Source {
            proprietaire: proprietaire.to_owned(),
            depot: depot.to_owned(),
            api: None,
            cles: cles_officielles(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            cible: None,
        }
    }

    fn configurer(
        &self,
        delai: Duration,
        version: Option<&str>,
    ) -> Result<self_update::backends::github::Update, String> {
        // Sans clé, `self_update` ne vérifierait rien : on refuse plutôt.
        if self.cles.is_empty() {
            return Err(tr!(
                "les mises à jour automatiques ne sont pas encore configurées dans cette version"
            )
            .to_owned());
        }
        let mut b = self_update::backends::github::Update::configure();
        b.repo_owner(&self.proprietaire)
            .repo_name(&self.depot)
            .bin_name(EXECUTABLE)
            .current_version(&self.version)
            .update_strategy(self_update::update::UpdateStrategy::Latest)
            .verifying_keys(self.cles.clone())
            // Sans limite, une connexion coupée sans prévenir bloquerait la mise à jour jusqu'à
            // la fermeture de l'application.
            .timeout(delai)
            .show_output(false)
            .show_download_progress(false)
            .no_confirm(true);
        if let Some(version) = version {
            // Exactement cette version, et seulement l'archive qui porte son numéro : la signature
            // couvre le nom de l'archive, donc une vieille archive signée ne peut pas être
            // resservie sous l'étiquette d'une version plus récente.
            let attendue = nom_d_archive(version);
            b.release_tag(format!("v{version}"))
                .asset_matcher(move |archives| {
                    archives.iter().find(|a| a.name() == attendue).cloned()
                });
        }
        if let Some(api) = &self.api {
            b.api_base_url(api);
        }
        if let Some(cible) = &self.cible {
            b.bin_install_path(cible);
        }
        b.build().map_err(|e| e.to_string())
    }

    /// La plus récente des versions publiées plus récentes que celle installée (par numéro de
    /// version, pas par date de publication).
    fn plus_recente(&self) -> Result<Option<self_update::Release>, String> {
        let maj = self.configurer(DELAI_RECHERCHE, None)?;
        let versions = maj.get_newer_releases().map_err(|e| e.to_string())?;
        let mut meilleure: Option<self_update::Release> = None;
        for r in versions.into_vec() {
            let plus_grande = match &meilleure {
                None => true,
                Some(m) => {
                    self_update::version::bump_is_greater(m.version(), r.version()).unwrap_or(false)
                }
            };
            if plus_grande {
                meilleure = Some(r);
            }
        }
        Ok(meilleure)
    }

    /// Cherche une version plus récente que celle installée.
    pub fn rechercher(&self) -> Result<Option<Nouveaute>, String> {
        Ok(self.plus_recente()?.map(|r| Nouveaute {
            version: r.version().to_owned(),
            notes: r.body().unwrap_or_default().to_owned(),
        }))
    }

    /// Télécharge la dernière version, vérifie sa signature et remplace l'exécutable. Renvoie
    /// la version installée, ou `None` si l'on avait déjà la dernière. En cas d'échec, le message
    /// dit dans quel état est l'exécutable installé.
    pub fn installer(&self) -> Result<Option<String>, String> {
        let pas_modifiee = |e: String| tr!("{e}. Ta version actuelle n'a pas été modifiée", e = e);
        let Some(r) = self.plus_recente().map_err(pas_modifiee)? else {
            return Ok(None);
        };
        let version = r.version().to_owned();
        let maj = self
            .configurer(DELAI_INSTALLATION, Some(&version))
            .map_err(pas_modifiee)?;
        let exe = match &self.cible {
            Some(cible) => cible.clone(),
            // Sans `canonicalize` : le chemin reste celui que montre l'Explorateur (pas de « \\?\ »)
            // dans les messages.
            None => std::env::current_exe()
                .map_err(|e| pas_modifiee(tr!("exécutable introuvable ({e})", e = e)))?,
        };
        avec_secours(&exe, || {
            match maj.update_extended().map_err(|e| e.to_string())? {
                self_update::update::ReleaseStatus::Updated(r) if r.version() == version => {
                    Ok(Some(version))
                }
                self_update::update::ReleaseStatus::Updated(r) => Err(tr!(
                    "la version {version2} est arrivée au lieu de la {version}",
                    version = version,
                    version2 = r.version()
                )),
                _ => Ok(None),
            }
        })
    }
}

/// Limite d'une recherche (liste des versions).
const DELAI_RECHERCHE: Duration = Duration::from_secs(30);
/// Limite de chaque requête d'une installation, téléchargement compris (même lent).
const DELAI_INSTALLATION: Duration = Duration::from_secs(15 * 60);

/// Le nom de l'archive publiée pour cette version et la cible de compilation, celui que produit
/// `outils/empaqueter.sh`.
pub fn nom_d_archive(version: &str) -> String {
    format!("{EXECUTABLE}-v{version}-{}.zip", self_update::get_target())
}

/// La copie de secours de l'exécutable, à côté de lui, le temps de la mise à jour.
pub fn copie_de_secours(exe: &Path) -> PathBuf {
    let mut nom = exe.file_name().unwrap_or_default().to_os_string();
    nom.push(".precedent");
    exe.with_file_name(nom)
}

/// Fait `installer` en gardant une copie de l'exécutable, et la remet en place si l'installation
/// échoue après l'avoir retiré. Sous Windows, `self-replace` déplace d'abord l'exécutable en
/// marche, puis met le nouveau à sa place : si cette dernière étape échoue (antivirus qui tient le
/// fichier, disque plein), il ne revient pas en arrière et le dossier resterait sans exécutable.
pub fn avec_secours<T>(
    exe: &Path,
    installer: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let copie = copie_de_secours(exe);
    if let Err(e) = std::fs::copy(exe, &copie) {
        let dossier = exe.parent().unwrap_or(exe);
        return Err(tr!("impossible d'écrire dans le dossier « {display} » ({e}) : place l'application dans un dossier à toi, par exemple Documents. Ta version actuelle n'a pas été modifiée", e = e, display = dossier.display()));
    }
    let resultat = installer();
    let Err(e) = resultat else {
        let _ = std::fs::remove_file(&copie);
        return resultat;
    };
    if exe.exists() {
        let _ = std::fs::remove_file(&copie);
        return Err(tr!("{e}. Ta version actuelle n'a pas été modifiée", e = e));
    }
    // L'exécutable a été retiré : on remet la copie en la renommant (même dossier, donc sans
    // place disque, même si le disque est plein), sinon en la recopiant, en insistant un peu si
    // un antivirus tient encore le dossier. Les restes de self-replace partent d'abord : ils
    // occupent de la place.
    nettoyer_les_restes(exe);
    for essai in 0..6u32 {
        if std::fs::rename(&copie, exe).is_ok() {
            return Err(tr!("{e}. Ta version actuelle a été remise en place", e = e));
        }
        if std::fs::copy(&copie, exe).is_ok() {
            let _ = std::fs::remove_file(&copie);
            return Err(tr!("{e}. Ta version actuelle a été remise en place", e = e));
        }
        std::thread::sleep(Duration::from_millis(100 << essai));
    }
    Err(tr!("{e}. Ta version actuelle est gardée dans « {display} » : renomme ce fichier en « {v} » pour retrouver l'application", e = e, display = copie.display(), v = exe.file_name().unwrap_or_default().to_string_lossy()))
}

/// Retire la copie de secours qu'une mise à jour interrompue (application fermée pendant le
/// téléchargement) aurait laissée à côté de l'exécutable en marche : il existe, elle ne sert plus.
pub fn retirer_une_vieille_copie_de_secours() {
    if let Ok(exe) = std::env::current_exe() {
        let copie = copie_de_secours(&exe);
        if exe.exists() && copie.exists() {
            let _ = std::fs::remove_file(copie);
        }
    }
}

/// Retire les copies temporaires que `self-replace` laisse à côté de l'exécutable quand il
/// échoue en cours de route (`.computer-engineer.<hasard>.__temp__.exe`).
fn nettoyer_les_restes(exe: &Path) {
    let Some(dossier) = exe.parent() else {
        return;
    };
    let Some(nom) = exe.file_stem().and_then(|n| n.to_str()) else {
        return;
    };
    let debut = format!(".{nom}.");
    let Ok(entrees) = std::fs::read_dir(dossier) else {
        return;
    };
    for f in entrees.flatten() {
        let n = f.file_name().to_string_lossy().into_owned();
        if n.starts_with(&debut) && n.contains(".__temp__") {
            let _ = std::fs::remove_file(f.path());
        }
    }
}

/// Une version plus récente, publiée.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Nouveaute {
    /// Son numéro, sans le « v ».
    pub version: String,
    /// Ses nouveautés, telles qu'écrites sur la page de la version.
    pub notes: String,
}

/// Où en est la mise à jour.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Etat {
    /// Rien n'a été demandé.
    Repos,
    /// Recherche en cours.
    Recherche {
        /// Demandée par l'utilisateur : on lui dira le résultat, même « rien de neuf ».
        demandee: bool,
    },
    /// Une version plus récente existe.
    Disponible(Nouveaute),
    /// Téléchargement, vérification et remplacement en cours.
    Installation,
    /// La nouvelle version est en place : elle tourne au prochain lancement.
    Installee(String),
}

/// Ce que rapporte le fil d'exécution de la mise à jour.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Nouvelle {
    /// Résultat d'une recherche.
    Recherche {
        /// Demandée par l'utilisateur.
        demandee: bool,
        /// La version trouvée, ou l'erreur.
        resultat: Result<Option<Nouveaute>, String>,
    },
    /// Résultat d'une installation.
    Installation(Result<Option<String>, String>),
}

/// La mise à jour vue de l'application : lance les recherches et installations dans un fil
/// d'exécution à part, et relève leurs résultats à chaque image.
pub struct MiseAJour {
    /// D'où viennent les versions.
    pub source: Source,
    /// Où l'on en est.
    pub etat: Etat,
    canal: Option<mpsc::Receiver<Nouvelle>>,
}

impl MiseAJour {
    /// Une mise à jour au repos, depuis cette source.
    pub fn nouvelle(source: Source) -> Self {
        MiseAJour {
            source,
            etat: Etat::Repos,
            canal: None,
        }
    }

    /// Vrai si un fil d'exécution travaille encore.
    pub fn occupee(&self) -> bool {
        self.canal.is_some()
    }

    fn lancer(&mut self, travail: impl FnOnce(&Source) -> Nouvelle + Send + 'static) {
        let (envoi, reception) = mpsc::channel();
        let source = self.source.clone();
        std::thread::spawn(move || {
            let _ = envoi.send(travail(&source));
        });
        self.canal = Some(reception);
    }

    /// Cherche une version plus récente, sans bloquer. `demandee` : l'utilisateur l'a demandé ;
    /// si une recherche automatique tourne déjà, on lui dira son résultat.
    pub fn rechercher(&mut self, demandee: bool) {
        if let Etat::Recherche { demandee: d } = &mut self.etat {
            *d |= demandee;
            return;
        }
        if self.occupee() || matches!(self.etat, Etat::Installee(_)) {
            return;
        }
        self.etat = Etat::Recherche { demandee };
        self.lancer(move |s| Nouvelle::Recherche {
            demandee,
            resultat: s.rechercher(),
        });
    }

    /// Installe la dernière version, sans bloquer. Ne fait rien si une recherche ou une
    /// installation tourne déjà, ou si une version est déjà installée.
    pub fn installer(&mut self) {
        if self.occupee() || matches!(self.etat, Etat::Installee(_)) {
            return;
        }
        self.etat = Etat::Installation;
        self.lancer(|s| Nouvelle::Installation(s.installer()));
    }

    /// Le résultat du fil d'exécution, s'il vient d'arriver. L'état est mis à jour.
    pub fn relever(&mut self) -> Option<Nouvelle> {
        let nouvelle = match self.canal.as_ref()?.try_recv() {
            Ok(n) => n,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => self.interrompue(),
        };
        Some(self.recevoir(nouvelle))
    }

    /// Attend le résultat du fil d'exécution (pour les essais et la ligne de commande).
    pub fn attendre(&mut self) -> Option<Nouvelle> {
        let nouvelle = match self.canal.as_ref()?.recv() {
            Ok(n) => n,
            Err(mpsc::RecvError) => self.interrompue(),
        };
        Some(self.recevoir(nouvelle))
    }

    /// Le fil d'exécution s'est arrêté sans rien dire (il a paniqué).
    fn interrompue(&self) -> Nouvelle {
        let e = tr!("la mise à jour s'est arrêtée en cours de route").to_owned();
        match self.etat {
            Etat::Recherche { demandee } => Nouvelle::Recherche {
                demandee,
                resultat: Err(e),
            },
            _ => Nouvelle::Installation(Err(e)),
        }
    }

    fn recevoir(&mut self, mut nouvelle: Nouvelle) -> Nouvelle {
        self.canal = None;
        // Demandée pendant qu'elle tournait déjà : l'utilisateur attend la réponse.
        if let (Etat::Recherche { demandee: true }, Nouvelle::Recherche { demandee, .. }) =
            (&self.etat, &mut nouvelle)
        {
            *demandee = true;
        }
        self.etat = match &nouvelle {
            Nouvelle::Recherche {
                resultat: Ok(Some(n)),
                ..
            } => Etat::Disponible(n.clone()),
            Nouvelle::Installation(Ok(Some(v))) => Etat::Installee(v.clone()),
            _ => Etat::Repos,
        };
        nouvelle
    }
}

/// Les réglages de l'utilisateur, dont la recherche automatique des mises à jour.
pub use crate::reglages::Reglages;

/// Redémarre sur la version installée. Ne revient qu'en cas d'échec.
pub fn redemarrer() -> String {
    match self_update::restart::restart_with(std::iter::empty::<&str>()) {
        Ok(jamais) => match jamais {},
        Err(e) => e.to_string(),
    }
}
