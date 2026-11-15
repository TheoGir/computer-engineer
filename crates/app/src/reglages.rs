//! Les réglages de l'utilisateur : ce qu'il choisit dans l'écran Paramètres, enregistré dans
//! `reglages.json` à côté de ses projets.
//!
//! Les réglages s'ajoutent à l'apparence lue dans `apparence.json` sans la remplacer : un réglage
//! absent (`None`) laisse la valeur du fichier.

use serde::{Deserialize, Serialize};

use crate::apparence::{Apparence, Mode};

/// Ce que l'application montre au lancement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Lancement {
    /// L'écran d'accueil, le dernier projet prêt à reprendre d'un clic ou avec Entrée.
    #[default]
    Accueil,
    /// Directement le dernier projet, comme on l'a laissé.
    Reprendre,
}

/// Les tailles de texte proposées, en facteur de l'échelle de l'apparence.
pub const TAILLES_TEXTE: [(f32, &str); 4] = [
    (0.9, "Petite"),
    (1.0, "Normale"),
    (1.15, "Grande"),
    (1.3, "Très grande"),
];

/// Les réglages de l'utilisateur.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Reglages {
    /// Chercher une nouvelle version au lancement.
    pub mises_a_jour_automatiques: bool,
    /// Ce qui s'affiche au lancement.
    pub au_lancement: Lancement,
    /// Thème choisi ; `None` : celui d'`apparence.json`.
    pub theme: Option<Mode>,
    /// Animations réduites (pas de perles ni de pulsations) ; `None` : selon `apparence.json`.
    pub animations_sobres: Option<bool>,
    /// Facteur de la taille du texte (1 = normale).
    pub taille_texte: f32,
    /// Vitesse de simulation au départ, rang dans les vitesses lentes ; `None` : selon
    /// `apparence.json`.
    pub vitesse: Option<usize>,
    /// Vrai une fois les raccourcis du bureau et du menu Démarrer posés au premier lancement :
    /// un raccourci que l'utilisateur a supprimé n'est plus remis (`raccourci.rs`).
    pub raccourcis_proposes: bool,
}

impl Default for Reglages {
    fn default() -> Self {
        Reglages {
            mises_a_jour_automatiques: true,
            au_lancement: Lancement::Accueil,
            theme: None,
            animations_sobres: None,
            taille_texte: 1.0,
            vitesse: None,
            raccourcis_proposes: false,
        }
    }
}

impl Reglages {
    /// L'apparence du fichier, avec les choix de l'utilisateur par-dessus.
    pub fn appliquer(&self, fichier: &Apparence) -> Apparence {
        let mut a = fichier.clone();
        if let Some(mode) = self.theme {
            a.mode = mode;
        }
        if let Some(sobre) = self.animations_sobres {
            a.mouvement.sobre = sobre;
        }
        if self.taille_texte.is_finite() {
            a.formes.echelle_texte =
                (a.formes.echelle_texte * self.taille_texte.clamp(0.75, 1.5)).clamp(0.6, 2.0);
        }
        if let Some(v) = self.vitesse {
            a.mouvement.vitesse_initiale = v.min(a.mouvement.vitesses.len().saturating_sub(1));
        }
        a
    }

    /// Le rang de la taille de texte choisie dans [`TAILLES_TEXTE`] (la plus proche).
    pub fn rang_taille_texte(&self) -> usize {
        TAILLES_TEXTE
            .iter()
            .enumerate()
            .min_by(|(_, (a, _)), (_, (b, _))| {
                (a - self.taille_texte)
                    .abs()
                    .total_cmp(&(b - self.taille_texte).abs())
            })
            .map_or(1, |(i, _)| i)
    }

    /// Les réglages écrits en JSON.
    pub fn en_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("les réglages se sérialisent toujours")
    }

    /// Relit des réglages écrits en JSON. Voir [`Reglages::lire`].
    pub fn de_json(texte: &str) -> Reglages {
        Reglages::lire(texte).0
    }

    /// Relit des réglages écrits en JSON, champ par champ : un réglage illisible (une valeur
    /// d'un autre type, retouchée à la main ou écrite par une autre version) garde sa valeur par
    /// défaut sans emporter les autres. Un fichier que sa retouche a rendu invalide (virgule en
    /// trop, mot sans guillemets, virgule décimale) est relu ligne par ligne. Renvoie aussi
    /// `false` si quelque chose n'a pas été compris.
    pub fn lire(texte: &str) -> (Reglages, bool) {
        use serde_json::{Map, Value};
        let texte = texte.trim_start_matches('\u{feff}');
        let (champs, mut complet) = match serde_json::from_str(texte) {
            Ok(Value::Object(champs)) => (champs, true),
            _ => (champs_ligne_par_ligne(texte), false),
        };
        let mut lisibles = Map::new();
        for (cle, valeur) in champs {
            let mut essai = lisibles.clone();
            essai.insert(cle.clone(), valeur.clone());
            if serde_json::from_value::<Reglages>(Value::Object(essai)).is_ok() {
                lisibles.insert(cle, valeur);
            } else {
                complet = false;
            }
        }
        let reglages = serde_json::from_value(Value::Object(lisibles)).unwrap_or_default();
        (reglages, complet)
    }
}

/// Les champs d'un fichier de réglages qui n'est plus du JSON valide : une ligne ou un
/// morceau « "clé": valeur » à la fois, en acceptant `False`, `clair` sans guillemets et `1,3`.
fn champs_ligne_par_ligne(texte: &str) -> serde_json::Map<String, serde_json::Value> {
    use serde_json::Value;
    let mut champs = serde_json::Map::new();
    let propre = |m: &str| -> String {
        m.trim()
            .trim_matches(|c| matches!(c, '{' | '}' | ','))
            .trim()
            .to_owned()
    };
    // Une clé par ligne, comme l'application l'écrit ; sinon tout sur une ligne, coupé aux
    // virgules.
    let morceaux: Vec<String> = if texte.lines().filter(|l| l.contains(':')).count() > 1 {
        texte.lines().map(propre).collect()
    } else {
        texte.split(',').map(propre).collect()
    };
    for morceau in morceaux {
        let Some((cle, valeur)) = morceau.split_once(':') else {
            continue;
        };
        let cle = cle.trim().trim_matches(|c| matches!(c, '"' | '{' | ' '));
        let valeur = valeur.trim();
        let essais = [
            valeur.to_owned(),
            valeur.to_lowercase(),
            valeur.replace(',', "."),
            format!("\"{}\"", valeur.trim_matches(|c| matches!(c, '"' | '\''))),
        ];
        if let Some(v) = essais
            .iter()
            .find_map(|e| serde_json::from_str::<Value>(e).ok())
        {
            champs.insert(cle.to_owned(), v);
        }
    }
    champs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sans_choix_l_apparence_du_fichier_reste_telle_quelle() {
        let a = Apparence::default();
        assert_eq!(Reglages::default().appliquer(&a), a);
    }

    #[test]
    fn les_choix_passent_par_dessus_le_fichier() {
        let a = Apparence::default();
        let r = Reglages {
            theme: Some(Mode::Clair),
            animations_sobres: Some(true),
            taille_texte: 1.3,
            vitesse: Some(99),
            ..Reglages::default()
        };
        let b = r.appliquer(&a);
        assert_eq!(b.mode, Mode::Clair);
        assert!(b.mouvement.sobre);
        assert!((b.formes.echelle_texte - 1.3).abs() < 1e-6);
        assert_eq!(b.mouvement.vitesse_initiale, a.mouvement.vitesses.len() - 1);
        assert_eq!(r.rang_taille_texte(), 3);
    }

    #[test]
    fn un_ancien_fichier_de_reglages_se_relit() {
        // Le fichier écrit par la version 0.1.0 n'avait que ce champ.
        let r: Reglages = serde_json::from_str(r#"{ "mises_a_jour_automatiques": false }"#)
            .expect("ancien fichier");
        assert!(!r.mises_a_jour_automatiques);
        assert_eq!(r.au_lancement, Lancement::Accueil);
        assert_eq!(r.taille_texte, 1.0);
        let relu: Reglages = serde_json::from_str(&r.en_json()).unwrap();
        assert_eq!(relu, r);
    }

    #[test]
    fn un_reglage_illisible_n_emporte_pas_les_autres() {
        let r = Reglages::de_json(
            r#"{ "theme": "clair", "taille_texte": "grande", "vitesse": 2,
                 "au_lancement": "ailleurs", "inconnu": [1, 2] }"#,
        );
        assert_eq!(r.theme, Some(Mode::Clair));
        assert_eq!(r.vitesse, Some(2));
        assert_eq!(r.taille_texte, 1.0, "valeur d'un autre type : par défaut");
        assert_eq!(
            r.au_lancement,
            Lancement::Accueil,
            "valeur inconnue : par défaut"
        );
        assert_eq!(Reglages::de_json("pas du json"), Reglages::default());
        assert_eq!(Reglages::de_json("[1]"), Reglages::default());
        // Tous les réglages font l'aller-retour.
        let tout = Reglages {
            mises_a_jour_automatiques: false,
            au_lancement: Lancement::Reprendre,
            theme: Some(Mode::Clair),
            animations_sobres: Some(true),
            taille_texte: 1.3,
            vitesse: Some(1),
            raccourcis_proposes: true,
        };
        assert_eq!(Reglages::de_json(&tout.en_json()), tout);
    }
}
