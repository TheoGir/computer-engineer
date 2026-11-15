//! La langue de l'interface : français ou anglais.
//!
//! Le texte français écrit dans le code sert de clé : `tr!("Ouvrir")` rend « Ouvrir » en
//! français et sa traduction en anglais, rangée dans `locales/en.json` et intégrée à
//! l'exécutable par `rust-i18n`. Une phrase qui n'a pas encore de traduction reste en français
//! plutôt que de montrer une clé : rien ne manque jamais à l'écran.
//!
//! Les données du parcours (quêtes, fiches, chapitres) gardent leurs textes français dans le
//! code et sont traduites à l'affichage avec [`traduire`], comme les descriptions des briques
//! livrées avec l'application. Ce que l'utilisateur écrit lui-même ne se traduit pas : sa clé
//! n'existe pas, il reste tel quel.
//!
//! Le test `traductions` (dans `tests/`) vérifie que chaque texte du code et des données a sa
//! traduction, avec les mêmes trous `{nom}` à remplir. `outils/traductions.py` liste ce qui
//! manque.

use std::borrow::Cow;
use std::cell::Cell;
use std::fmt::Display;
use std::sync::atomic::{AtomicU8, Ordering};

use serde::{Deserialize, Serialize};

/// Une langue de l'interface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Langue {
    /// Le français, langue d'origine de l'application.
    #[default]
    Francais,
    /// L'anglais.
    Anglais,
}

impl Langue {
    /// Toutes les langues proposées, dans l'ordre des réglages.
    pub const TOUTES: [Langue; 2] = [Langue::Francais, Langue::Anglais];

    /// Le nom de la langue, écrit dans cette langue (on reconnaît la sienne).
    pub fn nom(self) -> &'static str {
        match self {
            Langue::Francais => "Français",
            Langue::Anglais => "English",
        }
    }

    /// Le code de la langue dans `locales/`.
    pub fn code(self) -> &'static str {
        match self {
            Langue::Francais => "fr",
            Langue::Anglais => "en",
        }
    }

    /// La langue du système, au premier lancement : le français si le système est en
    /// français, l'anglais sinon.
    pub fn du_systeme() -> Langue {
        Langue::de_code(&sys_locale::get_locale().unwrap_or_default())
    }

    /// La langue d'un code comme `fr-FR`, `fr_CA.UTF-8` ou `en-US` : le français pour
    /// tout code en `fr`, l'anglais sinon.
    pub fn de_code(code: &str) -> Langue {
        let code = code.trim().to_ascii_lowercase();
        if code == "fr" || code.starts_with("fr-") || code.starts_with("fr_") {
            Langue::Francais
        } else {
            Langue::Anglais
        }
    }

    fn rang(self) -> u8 {
        match self {
            Langue::Francais => 0,
            Langue::Anglais => 1,
        }
    }

    fn de_rang(rang: u8) -> Langue {
        if rang == 1 {
            Langue::Anglais
        } else {
            Langue::Francais
        }
    }
}

/// La langue de toute l'application.
static LANGUE: AtomicU8 = AtomicU8::new(0);

thread_local! {
    /// Une langue propre à ce fil d'exécution, qui passe devant la langue de l'application :
    /// les tests s'en servent pour se dérouler en anglais sans gêner ceux d'à côté.
    static POUR_CE_FIL: Cell<Option<Langue>> = const { Cell::new(None) };
}

/// Change la langue de toute l'application ; si ce fil d'exécution a sa propre langue (un
/// test), c'est elle qui change, sans toucher aux autres.
pub fn choisir(langue: Langue) {
    if POUR_CE_FIL.with(Cell::get).is_some() {
        POUR_CE_FIL.with(|l| l.set(Some(langue)));
    } else {
        LANGUE.store(langue.rang(), Ordering::Relaxed);
    }
}

/// Change la langue de ce fil d'exécution seulement (`None` : celle de l'application).
pub fn choisir_pour_ce_fil(langue: Option<Langue>) {
    POUR_CE_FIL.with(|l| l.set(langue));
}

/// La langue en cours.
pub fn actuelle() -> Langue {
    POUR_CE_FIL
        .with(Cell::get)
        .unwrap_or_else(|| Langue::de_rang(LANGUE.load(Ordering::Relaxed)))
}

/// Vrai si l'interface est en français.
pub fn en_francais() -> bool {
    actuelle() == Langue::Francais
}

/// Le texte dans la langue en cours. Le texte français est la clé ; sans traduction, il
/// reste en français.
pub fn traduire(francais: &'static str) -> &'static str {
    traduire_en(actuelle(), francais)
}

/// Le texte dans cette langue. Voir [`traduire`].
pub fn traduire_en(langue: Langue, francais: &'static str) -> &'static str {
    if langue == Langue::Francais {
        return francais;
    }
    match traduction(langue, francais) {
        Some(Cow::Borrowed(t)) => t,
        // Les traductions intégrées sont toutes empruntées ; une autre (jamais vue) garde le
        // texte d'origine plutôt que de fuir de la mémoire.
        Some(Cow::Owned(_)) | None => francais,
    }
}

/// Le texte dans la langue en cours, pour un texte qui n'est pas écrit dans le code (la
/// description d'une brique lue dans un projet) : traduit s'il est connu, tel quel sinon.
pub fn traduire_texte(francais: &str) -> Cow<'_, str> {
    let langue = actuelle();
    if langue == Langue::Francais {
        return Cow::Borrowed(francais);
    }
    match traduction(langue, francais) {
        Some(Cow::Borrowed(t)) => Cow::Borrowed(t),
        Some(Cow::Owned(t)) => Cow::Owned(t),
        None => Cow::Borrowed(francais),
    }
}

/// La traduction intégrée de ce texte dans cette langue, s'il y en a une.
pub fn traduction(langue: Langue, francais: &str) -> Option<Cow<'static, str>> {
    crate::_rust_i18n_try_translate(langue.code(), francais)
}

/// Les textes des données, traduits à l'affichage avec [`traduire`] ou [`traduire_texte`] :
/// quêtes, chapitres, fiches des composants, tailles de texte, exemples, et descriptions des
/// briques livrées avec l'application. Le test `traductions` vérifie qu'ils sont tous traduits.
pub fn textes_des_donnees() -> Vec<String> {
    use crate::quete::PARCOURS;
    use crate::savoir::{CHAPITRES, SAVOIRS};
    let mut textes: Vec<&str> = Vec::new();
    for f in PARCOURS {
        textes.extend([f.chapitre, f.titre, f.consigne, f.felicitations]);
    }
    for c in CHAPITRES {
        textes.extend([c.nom, c.apprend]);
    }
    for s in SAVOIRS {
        textes.extend([s.sert, s.marche, s.place, s.vraie_vie, s.anecdote, s.essaie]);
    }
    textes.extend(crate::reglages::TAILLES_TEXTE.iter().map(|(_, n)| *n));
    for e in &crate::exemples::EXEMPLES {
        textes.extend([e.nom, e.resume, e.accueil]);
    }
    let mut tous: Vec<String> = textes.into_iter().map(str::to_owned).collect();
    tous.extend(
        crate::quete::corriges()
            .composants
            .values()
            .map(|c| c.description.clone()),
    );
    for e in &crate::exemples::EXEMPLES {
        if let Ok(projet) = crate::exemples::ordinateur(e.nom, e.programme) {
            tous.extend(projet.modeles.iter().map(|m| m.description.clone()));
        }
    }
    tous.retain(|t| !t.trim().is_empty());
    tous.sort();
    tous.dedup();
    tous
}

/// Remplit les trous `{nom}` d'un modèle avec les valeurs données ; `{{` et `}}` donnent une
/// accolade. Un trou sans valeur reste tel quel, pour se voir plutôt que de disparaître.
pub fn remplir(modele: &str, valeurs: &[(&str, &dyn Display)]) -> String {
    let mut texte = String::with_capacity(modele.len() + 16);
    let mut reste = modele;
    while let Some(i) = reste.find(['{', '}']) {
        texte.push_str(&reste[..i]);
        let suite = &reste[i..];
        if suite.starts_with("{{") || suite.starts_with("}}") {
            texte.push_str(&suite[..1]);
            reste = &suite[2..];
            continue;
        }
        if let Some(fin) = suite.strip_prefix('{').and_then(|s| s.find('}')) {
            let nom = &suite[1..=fin];
            if let Some((_, v)) = valeurs.iter().find(|(n, _)| *n == nom) {
                use std::fmt::Write as _;
                let _ = write!(texte, "{v}");
                reste = &suite[fin + 2..];
                continue;
            }
        }
        texte.push_str(&suite[..1]);
        reste = &suite[1..];
    }
    texte.push_str(reste);
    texte
}

/// Les trous `{nom}` d'un modèle, dans l'ordre, sans les accolades doublées.
pub fn trous(modele: &str) -> Vec<&str> {
    let mut noms = Vec::new();
    let mut reste = modele;
    while let Some(i) = reste.find(['{', '}']) {
        let suite = &reste[i..];
        if suite.starts_with("{{") || suite.starts_with("}}") {
            reste = &suite[2..];
            continue;
        }
        match suite.strip_prefix('{').and_then(|s| s.find('}')) {
            Some(fin) => {
                noms.push(&suite[1..=fin]);
                reste = &suite[fin + 2..];
            }
            None => reste = &suite[1..],
        }
    }
    noms
}

/// Le texte dans la langue en cours.
///
/// - `tr!("Ouvrir")` : un `&'static str` traduit ;
/// - `tr!("{n} briques", n = total)` : une `String` dont les trous `{n}` sont remplis, après
///   traduction du modèle.
#[macro_export]
macro_rules! tr {
    ($texte:literal) => {
        $crate::langue::traduire($texte)
    };
    ($texte:literal, $($nom:ident = $valeur:expr),+ $(,)?) => {
        $crate::langue::remplir(
            $crate::langue::traduire($texte),
            &[$((stringify!($nom), &$valeur as &dyn ::std::fmt::Display)),+],
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_modele_se_remplit() {
        let n = 3;
        assert_eq!(remplir("{n} briques", &[("n", &n)]), "3 briques");
        assert_eq!(
            remplir("{a} et {b}, {{a}} reste", &[("a", &1), ("b", &"deux")]),
            "1 et deux, {a} reste"
        );
        assert_eq!(remplir("{inconnu} {", &[]), "{inconnu} {");
        assert_eq!(trous("{a} {{b}} {c}"), vec!["a", "c"]);
    }

    #[test]
    fn la_langue_du_systeme_se_reconnait() {
        assert_eq!(Langue::de_code("fr-FR"), Langue::Francais);
        assert_eq!(Langue::de_code("fr_CA.UTF-8"), Langue::Francais);
        assert_eq!(Langue::de_code("FR"), Langue::Francais);
        assert_eq!(Langue::de_code("en-US"), Langue::Anglais);
        assert_eq!(Langue::de_code("frisian"), Langue::Anglais);
        assert_eq!(Langue::de_code(""), Langue::Anglais);
    }

    #[test]
    fn sans_traduction_le_francais_reste() {
        choisir_pour_ce_fil(Some(Langue::Anglais));
        assert_eq!(traduire("Une phrase qui n'existe nulle part."), "Une phrase qui n'existe nulle part.");
        choisir_pour_ce_fil(None);
    }
}
