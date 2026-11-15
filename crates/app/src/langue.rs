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

/// La marque du pluriel après le nombre `n` : « s » ou rien. En français, 0 et 1 sont au
/// singulier (« 0 porte ») ; en anglais, seul 1 l'est (« 0 gates »).
pub fn s<N: TryInto<u64>>(n: N) -> &'static str {
    let n: u64 = n.try_into().unwrap_or(u64::MAX);
    let pluriel = if en_francais() { n > 1 } else { n != 1 };
    if pluriel {
        "s"
    } else {
        ""
    }
}

/// Les noms des briques livrées et de leurs broches, en anglais. Ce sont des noms de circuit :
/// enregistrés en français dans les projets et vérifiés par les corrigés, ils ne sont traduits
/// qu'à l'affichage ([`nom_affiche`]), pour qu'un projet s'ouvre à l'identique dans les deux
/// langues. Les lettres seules (`a`, `s`, `q`…) et les noms déjà anglais (`clk`, `irq`) ne
/// changent pas.
pub const NOMS_ANGLAIS: &[(&str, &str)] = &[
    // Briques.
    ("ADD8_RAPIDE", "ADD8_FAST"),
    ("ADD_COMPLET", "FULL_ADD"),
    ("ADD_COMPLET_REPARE", "FULL_ADD_FIXED"),
    ("ARBITRE", "ARBITER"),
    ("AUTOMATE_30", "AUTOMATON_30"),
    ("BALAYAGE", "SCAN"),
    ("BANC_REGISTRES", "REG_FILE"),
    ("BASCULE_D", "D_FLIPFLOP"),
    ("BASCULE_JK", "JK_FLIPFLOP"),
    ("BASCULE_T", "T_FLIPFLOP"),
    ("BINAIRE_DCB", "BIN_TO_BCD"),
    ("Brique", "Brick"),
    ("CADENCE", "PACER"),
    ("CARRE", "SQUARE"),
    ("CARTE_RESEAU", "NETWORK_CARD"),
    ("CELLULE", "CELL"),
    ("CELLULE_VIE", "LIFE_CELL"),
    ("CESAR", "CAESAR"),
    ("CHRONOMETRE", "STOPWATCH"),
    ("COMPARATEUR4", "COMPARATOR4"),
    ("COMPTEUR4", "COUNTER4"),
    ("COMPTEUR_24", "COUNTER_24"),
    ("COMPTEUR_60", "COUNTER_60"),
    ("COMPTEUR_BCD", "BCD_COUNTER"),
    ("COMPTEUR_CASCADE", "CASCADE_COUNTER"),
    ("COMPTEUR_HB4", "UPDOWN_COUNTER4"),
    ("COMPTEUR_PROGRAMME", "PROGRAM_COUNTER"),
    ("CONTROLEUR_ECRAN", "SCREEN_CONTROLLER"),
    ("CONTROLE_PARITE", "PARITY_CHECK"),
    ("DE", "DIE"),
    ("DECALAGE8", "SHIFT_REG8"),
    ("DECALEUR8", "SHIFTER8"),
    ("DECODEUR", "DECODER"),
    ("DECODEUR2", "DECODER2"),
    ("DECODEUR3", "DECODER3"),
    ("DECODEUR4", "DECODER4"),
    ("DEMI_ADD", "HALF_ADD"),
    ("DEMI_ADD_REPARE", "HALF_ADD_FIXED"),
    ("DETECTEUR_FRONT", "EDGE_DETECTOR"),
    ("DIVISEUR4", "DIVIDER4"),
    ("EGAL4", "EQUAL4"),
    ("EMETTEUR_SERIE", "SERIAL_TX"),
    ("ENCODEUR4", "ENCODER4"),
    ("EQUIV", "XNOR"),
    ("ET", "AND"),
    ("FEU", "TRAFFIC_LIGHT"),
    ("FILE", "QUEUE"),
    ("HAMMING_CORRECTEUR", "HAMMING_FIX"),
    ("INTERRUPTIONS", "INTERRUPTS"),
    ("LIAISON_SERIE", "SERIAL_LINK"),
    ("MAJORITE", "MAJORITY"),
    ("MASQUE_XOR", "XOR_MASK"),
    ("MEILLEUR_SCORE", "HIGH_SCORE"),
    ("MEMOIRE_MORTE", "ROM"),
    ("MEMOIRE_VIDEO", "VIDEO_RAM"),
    ("MINUTERIE", "TIMER"),
    ("MONTRE", "WATCH"),
    ("MORPION", "TIC_TAC_TOE"),
    ("MOTIF_101", "PATTERN_101"),
    ("MUX_REPARE", "MUX_FIXED"),
    ("NEGATIF8", "NEGATE8"),
    ("NON", "NOT"),
    ("NON_OU", "NOR"),
    ("NUL8", "ZERO8"),
    ("ORDINATEUR", "COMPUTER"),
    ("OSCILLATEUR", "OSCILLATOR"),
    ("OU", "OR"),
    ("OU_X", "XOR"),
    ("OU_X_REPARE", "XOR_FIXED"),
    ("PARITE8", "PARITY8"),
    ("PGCD", "GCD"),
    ("PILE", "STACK"),
    ("POLICE", "FONT"),
    ("PREMIER", "PRIME"),
    ("PROCESSEUR", "PROCESSOR"),
    ("RECEPTEUR_SERIE", "SERIAL_RX"),
    ("REGISTRE8", "REGISTER8"),
    ("REGISTRE8_RAZ", "REGISTER8_RST"),
    ("REVEIL", "ALARM_CLOCK"),
    ("SELECTEUR_BUS", "BUS_SELECTOR"),
    ("SEPT_SEGMENTS", "SEVEN_SEGMENT"),
    ("SERIALISEUR", "SERIALIZER"),
    ("SOMME_CONTROLE", "CHECKSUM"),
    ("SOUSTRACTEUR4", "SUBTRACTOR4"),
    ("TOUS_10", "ALL_10"),
    ("TOUS_11", "ALL_11"),
    ("TOUS_12", "ALL_12"),
    ("TOUS_13", "ALL_13"),
    ("TOUS_14", "ALL_14"),
    ("TOUS_15", "ALL_15"),
    ("TOUS_18", "ALL_18"),
    ("TOUS_2", "ALL_2"),
    ("TOUS_21", "ALL_21"),
    ("TOUS_25", "ALL_25"),
    ("TOUS_3", "ALL_3"),
    ("TOUS_4", "ALL_4"),
    ("TOUS_5", "ALL_5"),
    ("TOUS_6", "ALL_6"),
    ("TOUS_7", "ALL_7"),
    ("TOUS_8", "ALL_8"),
    ("TOUS_9", "ALL_9"),
    ("UAL", "ALU"),
    ("VERROU_D", "D_LATCH"),
    ("VERROU_SR", "SR_LATCH"),
    // Broches et éléments.
    ("adresse", "addr"),
    ("ajouter", "add"),
    ("arret", "stopped"),
    ("arreter", "stop"),
    ("charge", "load"),
    ("charge_c", "load_c"),
    ("charger", "do_load"),
    ("cible", "target"),
    ("clavier", "keyboard"),
    ("depiler", "pop"),
    ("dispo", "avail"),
    ("droite", "right"),
    ("ecran", "screen"),
    ("ecran_ecrit", "screen_wr"),
    ("ecrire", "write"),
    ("ecrit_a", "wrote_a"),
    ("ecrit_ram", "ram_wr"),
    ("egal", "eq"),
    ("egalite", "equal"),
    ("empiler", "push"),
    ("emprunt", "borrow"),
    ("entree", "input"),
    ("envoyer", "send"),
    ("erreur", "error"),
    ("fin", "end"),
    ("fin_image", "frame_end"),
    ("fin_ligne", "line_end"),
    ("fini", "done"),
    ("front", "edge"),
    ("gagne", "wins"),
    ("gagne_a", "a_wins"),
    ("gagne_b", "b_wins"),
    ("gel", "freeze"),
    ("haut", "up"),
    ("inf", "less"),
    ("lancer", "roll"),
    ("ligne", "line"),
    ("ligne_e", "line_e"),
    ("lire", "read"),
    ("lu", "rdata"),
    ("marche", "run"),
    ("motif", "pattern"),
    ("nouveau", "new"),
    ("nraz", "nreset"),
    ("occupe", "busy"),
    ("orange", "amber"),
    ("plein", "full"),
    ("pret", "ready"),
    ("raz", "reset"),
    ("recu", "received"),
    ("regler", "set"),
    ("retenue", "carry"),
    ("rouge", "red"),
    ("rs", "co"),
    ("saut", "jump"),
    ("somme", "sum"),
    ("sommet", "top"),
    ("sonne", "ring"),
    ("sortie", "out"),
    ("sup", "greater"),
    ("tic", "tick"),
    ("valide", "valid"),
    ("valider", "confirm"),
    ("vert", "green"),
    ("vide", "empty"),
    ("vu", "seen"),
];

/// Le nom anglais d'un nom de circuit, s'il en a un.
fn nom_anglais(nom: &str) -> Option<&'static str> {
    NOMS_ANGLAIS
        .binary_search_by(|(fr, _)| fr.cmp(&nom))
        .ok()
        .map(|i| NOMS_ANGLAIS[i].1)
}

/// Un nom de circuit (brique, broche, élément, signal) tel qu'on l'affiche : en anglais, les
/// noms livrés sont traduits, morceau par morceau (`cpu.sortie`, `d[3]`, `ET 2`, `ecran1`) ;
/// un nom inconnu, choisi par l'utilisateur, reste tel quel. Le nom enregistré ne change
/// jamais : seul l'affichage suit la langue.
pub fn nom_affiche(nom: &str) -> Cow<'_, str> {
    if en_francais() || nom.is_empty() {
        return Cow::Borrowed(nom);
    }
    let mut sortie = String::with_capacity(nom.len() + 8);
    let mut change = false;
    for (i, morceau) in nom.split('.').enumerate() {
        if i > 0 {
            sortie.push('.');
        }
        // Ce qui suit le nom : un indice de bus, ou un numéro.
        let fin_nom = morceau.find(['[', ' ']).unwrap_or(morceau.len());
        let (tete, queue) = morceau.split_at(fin_nom);
        // `ecran1`, `clavier2` : le numéro qu'ajoute l'application reste derrière le nom.
        let racine = tete.trim_end_matches(|c: char| c.is_ascii_digit());
        let numero = &tete[racine.len()..];
        match nom_anglais(tete) {
            Some(t) => {
                sortie.push_str(t);
                change = true;
            }
            None => match nom_anglais(racine) {
                Some(t) if !numero.is_empty() && racine.chars().all(|c| c.is_ascii_lowercase()) => {
                    sortie.push_str(t);
                    sortie.push_str(numero);
                    change = true;
                }
                _ => sortie.push_str(tete),
            },
        }
        sortie.push_str(queue);
    }
    if change {
        Cow::Owned(sortie)
    } else {
        Cow::Borrowed(nom)
    }
}

/// Le nombre de caractères le plus grand d'un nom de circuit dans toutes les langues : la
/// taille d'une brique sur le plan ne dépend pas de la langue.
pub fn longueur_nom(nom: &str) -> usize {
    let anglais = nom_anglais(nom).map_or(0, |a| a.chars().count());
    nom.chars().count().max(anglais)
}

/// Le nom enregistré d'un nom de circuit tapé par l'utilisateur, en français ou en anglais,
/// parmi les noms possibles : `reset` désigne la broche `raz`.
pub fn nom_saisi<'a>(saisi: &str, possibles: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let mut anglais = None;
    for p in possibles {
        if p == saisi {
            return Some(p);
        }
        if anglais.is_none() && nom_anglais(p) == Some(saisi) {
            anglais = Some(p);
        }
    }
    anglais
}

/// Le nom anglais d'un nom de circuit, en toute langue (pour une recherche qui accepte les
/// deux) : `None` s'il n'en a pas.
pub fn nom_en_anglais(nom: &str) -> Option<&'static str> {
    nom_anglais(nom)
}

/// Un nombre décimal écrit avec un point (`format!("{x:.1}")`), avec la virgule en français :
/// « 1,7 » en français, « 1.7 » en anglais.
pub fn decimal(nombre: String) -> String {
    if en_francais() {
        nombre.replace('.', ",")
    } else {
        nombre
    }
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

/// Un programme en assembleur dont les commentaires (après `;`) sont traduits quand leur
/// traduction existe ; le code ne change pas, il s'assemble donc à l'identique. Sert aux
/// programmes livrés, montrés dans l'éditeur dans la langue de l'interface.
pub fn traduire_programme(texte: &str) -> String {
    if en_francais() {
        return texte.to_owned();
    }
    let mut sortie = String::with_capacity(texte.len());
    for (i, ligne) in texte.split('\n').enumerate() {
        if i > 0 {
            sortie.push('\n');
        }
        match commentaire(ligne) {
            Some(debut) => {
                let (code, note) = ligne.split_at(debut);
                let note = note[1..].trim();
                let espace = &ligne[debut + 1..ligne.len() - ligne[debut + 1..].trim_start().len()];
                sortie.push_str(code);
                sortie.push(';');
                sortie.push_str(espace);
                sortie.push_str(&traduire_texte(note));
            }
            None => sortie.push_str(ligne),
        }
    }
    sortie
}

/// La position du `;` qui ouvre le commentaire d'une ligne d'assembleur, hors caractères
/// entre apostrophes (`';'`).
fn commentaire(ligne: &str) -> Option<usize> {
    let mut dans_un_caractere = false;
    for (i, c) in ligne.char_indices() {
        match c {
            '\'' => dans_un_caractere = !dans_un_caractere,
            ';' if !dans_un_caractere => return Some(i),
            _ => {}
        }
    }
    None
}

/// Les commentaires des programmes livrés, à traduire.
fn commentaires_des_programmes() -> Vec<&'static str> {
    crate::programme::LIVRES
        .iter()
        .flat_map(|texte| texte.lines())
        .filter_map(|ligne| commentaire(ligne).map(|i| ligne[i + 1..].trim()))
        .filter(|note| !note.is_empty())
        .collect()
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
    // L'aide (F1), le lexique et les instructions.
    {
        use crate::fenetres as f;
        textes.extend([
            f::PREMIERS_PAS,
            f::RACCOURCIS,
            f::PROGRAMMER,
            f::FICHIERS,
            f::ALGEBRE,
        ]);
        for (mot, sens) in f::LEXIQUE {
            textes.extend([*mot, *sens]);
        }
        textes.extend(f::INSTRUCTIONS.iter().map(|(_, sens)| *sens));
    }
    textes.extend(crate::programme::PROGRAMMES.iter().map(|(nom, _)| *nom));
    textes.extend([
        crate::etat::ASTUCE_DU_PLAN,
        crate::raccourci::DESCRIPTION,
        crate::raccourci::MESSAGE_ARCHIVE,
        crate::automate::EXEMPLE,
    ]);
    textes.extend(commentaires_des_programmes());
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
                // Une valeur qui est un nom de circuit livré (`raz`, `ET`) s'affiche dans la
                // langue de l'interface.
                let valeur = v.to_string();
                texte.push_str(&nom_affiche(&valeur));
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
    fn les_programmes_traduits_s_assemblent_a_l_identique() {
        use ce_circuit::ordinateur::assembler;
        choisir_pour_ce_fil(Some(Langue::Anglais));
        for texte in crate::programme::LIVRES {
            let anglais = traduire_programme(texte);
            assert_ne!(anglais, *texte, "le programme n'a pas été traduit");
            assert!(!anglais.contains("touche"), "{anglais}");
            assert_eq!(assembler(&anglais).unwrap(), assembler(texte).unwrap());
        }
        choisir_pour_ce_fil(None);
        assert_eq!(traduire_programme("LDI ';' ; un"), "LDI ';' ; un");
        assert_eq!(commentaire("LDI ';' ; un"), Some(8));
    }

    #[test]
    fn les_noms_de_circuit_s_affichent_en_anglais() {
        let tries: Vec<_> = NOMS_ANGLAIS.iter().map(|(fr, _)| *fr).collect();
        let mut attendu = tries.clone();
        attendu.sort_unstable();
        attendu.dedup();
        assert_eq!(
            tries, attendu,
            "NOMS_ANGLAIS doit rester trié, sans doublon"
        );
        choisir_pour_ce_fil(Some(Langue::Anglais));
        assert_eq!(nom_affiche("ET"), "AND");
        assert_eq!(nom_affiche("raz"), "reset");
        assert_eq!(nom_affiche("cpu.sortie"), "cpu.out");
        assert_eq!(nom_affiche("d[3]"), "d[3]");
        assert_eq!(nom_affiche("ecran[2]"), "screen[2]");
        assert_eq!(nom_affiche("ecran1"), "screen1");
        assert_eq!(nom_affiche("ET 2"), "AND 2");
        assert_eq!(nom_affiche("TOUS_8"), "ALL_8");
        assert_eq!(nom_affiche("Mon circuit"), "Mon circuit");
        assert_eq!(nom_affiche("g12"), "g12");
        assert_eq!(remplir("brique {b}", &[("b", &"OU_X")]), "brique XOR");
        assert_eq!(nom_saisi("reset", ["a", "raz"]), Some("raz"));
        assert_eq!(nom_saisi("raz", ["raz"]), Some("raz"));
        assert_eq!(nom_saisi("zzz", ["raz"]), None);
        choisir_pour_ce_fil(None);
        assert_eq!(nom_affiche("ET"), "ET");
        assert_eq!(longueur_nom("ET"), 3);
    }

    /// Dans chaque brique livrée, les noms anglais des broches restent tous différents, et
    /// deux briques livrées n'ont pas le même nom anglais.
    #[test]
    fn les_noms_anglais_ne_se_confondent_pas() {
        choisir_pour_ce_fil(Some(Langue::Anglais));
        let bib = crate::quete::corriges();
        let mut briques = std::collections::BTreeMap::new();
        for (nom, c) in &bib.composants {
            let anglais = nom_affiche(nom).into_owned();
            if let Some(autre) = briques.insert(anglais.clone(), nom.clone()) {
                panic!("{nom} et {autre} s'appellent tous deux {anglais}");
            }
            let mut broches = std::collections::BTreeSet::new();
            for b in &c.broches {
                assert!(
                    broches.insert(nom_affiche(&b.nom).into_owned()),
                    "{nom} : deux broches s'appellent {} en anglais",
                    nom_affiche(&b.nom)
                );
            }
        }
        choisir_pour_ce_fil(None);
    }

    #[test]
    fn sans_traduction_le_francais_reste() {
        choisir_pour_ce_fil(Some(Langue::Anglais));
        assert_eq!(
            traduire("Une phrase qui n'existe nulle part."),
            "Une phrase qui n'existe nulle part."
        );
        choisir_pour_ce_fil(None);
    }
}
