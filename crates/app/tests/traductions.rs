//! Chaque texte de l'interface a sa traduction anglaise, avec les mêmes trous à remplir.
//!
//! Les textes viennent de deux endroits : les appels `tr!("…")` du code (lus ici dans
//! `src/`) et les données traduites à l'affichage (`langue::textes_des_donnees`). Les
//! traductions sont dans `locales/en.json`.
//!
//! `CE_ECRIRE_TRADUCTIONS=fichier.json` écrit la liste des textes sans traduction, à
//! compléter avec `outils/traductions.py`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use ce_app::langue::{self, Langue};

/// Un appel `tr!` trouvé dans le code.
struct Appel {
    fichier: String,
    ligne: usize,
    texte: String,
    arguments: Vec<String>,
}

/// La valeur d'un littéral Rust `"…"` qui commence à `debut` (sur le guillemet), et la
/// position juste après.
fn litteral(src: &str, debut: usize) -> (String, usize) {
    let octets = src.as_bytes();
    assert_eq!(octets[debut], b'"');
    let mut texte = String::new();
    let mut i = debut + 1;
    loop {
        let c = src[i..].chars().next().expect("littéral non fermé");
        match c {
            '"' => return (texte, i + 1),
            '\\' => {
                let e = src[i + 1..].chars().next().expect("échappement");
                i += 1 + e.len_utf8();
                match e {
                    'n' => texte.push('\n'),
                    't' => texte.push('\t'),
                    'r' => texte.push('\r'),
                    '0' => texte.push('\0'),
                    '\\' | '"' | '\'' => texte.push(e),
                    'u' => {
                        let fin = src[i..].find('}').expect("\\u{…}") + i;
                        let code = u32::from_str_radix(&src[i + 1..fin], 16).expect("\\u");
                        texte.push(char::from_u32(code).expect("caractère"));
                        i = fin + 1;
                    }
                    '\n' => {
                        while src[i..].starts_with(char::is_whitespace) {
                            i += src[i..].chars().next().map_or(1, char::len_utf8);
                        }
                    }
                    autre => panic!("échappement inconnu \\{autre}"),
                }
            }
            c => {
                texte.push(c);
                i += c.len_utf8();
            }
        }
    }
}

/// Les noms `nom = …` des arguments d'un appel, du début des arguments à la parenthèse
/// fermante.
fn arguments(src: &str, debut: usize) -> Vec<String> {
    let mut noms = Vec::new();
    let mut profondeur = 0i32;
    let mut morceau = String::new();
    let mut i = debut;
    let mut fini = |morceau: &mut String, noms: &mut Vec<String>| {
        let m = morceau.trim();
        if let Some((nom, _)) = m.split_once('=') {
            let nom = nom.trim();
            if !nom.is_empty() && nom.chars().all(|c| c.is_alphanumeric() || c == '_') {
                noms.push(nom.to_owned());
            }
        }
        morceau.clear();
    };
    while i < src.len() {
        let c = src[i..].chars().next().unwrap();
        match c {
            '"' => {
                let (_, fin) = litteral(src, i);
                morceau.push_str("\"\"");
                i = fin;
                continue;
            }
            '\'' => {
                // Un caractère ('a', '\n') ; pas de durées de vie dans ces arguments.
                if let Some(fin) = src[i + 1..].find('\'') {
                    let fin = if src[i + 1..].starts_with('\\') {
                        src[i + 3..].find('\'').map_or(fin, |f| f + 2)
                    } else {
                        fin
                    };
                    i += fin + 2;
                    continue;
                }
            }
            '(' | '[' | '{' => profondeur += 1,
            ')' | ']' | '}' => {
                if profondeur == 0 {
                    fini(&mut morceau, &mut noms);
                    return noms;
                }
                profondeur -= 1;
            }
            ',' if profondeur == 0 => {
                fini(&mut morceau, &mut noms);
                i += 1;
                continue;
            }
            _ => {}
        }
        if profondeur == 0 {
            morceau.push(c);
        }
        i += c.len_utf8();
    }
    panic!("appel tr! non fermé");
}

/// Tous les appels `tr!("…")` du code de l'application, hors tests et commentaires.
fn appels() -> Vec<Appel> {
    let racine = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut fichiers: Vec<_> = std::fs::read_dir(&racine)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
        .collect();
    fichiers.sort();
    let mut tous = Vec::new();
    for chemin in fichiers {
        let mut src = std::fs::read_to_string(&chemin).unwrap();
        if let Some(i) = src.find("#[cfg(test)]\nmod ") {
            src.truncate(i);
        }
        let nom = chemin.file_name().unwrap().to_string_lossy().into_owned();
        let mut depart = 0;
        while let Some(i) = src[depart..].find("tr!(") {
            let i = depart + i;
            depart = i + 4;
            let debut_ligne = src[..i].rfind('\n').map_or(0, |n| n + 1);
            if src[debut_ligne..i].contains("//") || src[..i].ends_with("macro_rules! ") {
                continue;
            }
            // Le `tr` doit être un mot entier (pas `str!(`, `attr!(`).
            if src[..i]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_')
            {
                continue;
            }
            let apres = src[i + 4..].trim_start();
            if !apres.starts_with('"') {
                continue;
            }
            let guillemet = src.len() - apres.len();
            let (texte, fin) = litteral(&src, guillemet);
            let arguments = arguments(&src, fin);
            tous.push(Appel {
                fichier: nom.clone(),
                ligne: src[..i].matches('\n').count() + 1,
                texte,
                arguments,
            });
        }
    }
    tous
}

/// Les traductions anglaises de `locales/en.json`.
fn anglais() -> BTreeMap<String, String> {
    let chemin = Path::new(env!("CARGO_MANIFEST_DIR")).join("locales/en.json");
    let valeur: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(chemin).unwrap()).unwrap();
    valeur
        .as_object()
        .unwrap()
        .iter()
        .filter(|(cle, _)| !cle.starts_with('_'))
        .map(|(cle, v)| (cle.clone(), v.as_str().unwrap().to_owned()))
        .collect()
}

/// Vrai si ce texte est le résultat d'un modèle `tr!` rempli (une description écrite par le
/// code avec un nombre dedans) : il n'est pas une clé à lui seul.
fn rempli_par_un_modele(texte: &str, modeles: &[String]) -> bool {
    modeles.iter().any(|m| {
        let trous = langue::trous(m);
        if trous.is_empty() {
            return false;
        }
        // Les morceaux fixes du modèle, dans l'ordre.
        let mut reste = texte;
        let mut morceaux = Vec::new();
        let mut m2 = m.as_str();
        for t in &trous {
            let marque = format!("{{{t}}}");
            let (avant, apres) = m2.split_once(&marque).unwrap();
            morceaux.push(avant);
            m2 = apres;
        }
        morceaux.push(m2);
        let (premier, autres) = morceaux.split_first().unwrap();
        let Some(r) = reste.strip_prefix(premier) else {
            return false;
        };
        reste = r;
        for (k, morceau) in autres.iter().enumerate() {
            let dernier = k + 1 == autres.len();
            if dernier {
                return reste.ends_with(morceau);
            }
            match reste.find(morceau) {
                Some(p) if !morceau.is_empty() => reste = &reste[p + morceau.len()..],
                _ => return false,
            }
        }
        true
    })
}

#[test]
fn chaque_texte_a_sa_traduction_anglaise() {
    let appels = appels();
    assert!(
        appels.len() > 500,
        "seulement {} appels tr! trouvés",
        appels.len()
    );
    let mut erreurs = Vec::new();
    // Les trous du modèle et les arguments donnés se correspondent.
    for a in &appels {
        let trous: BTreeSet<&str> = langue::trous(&a.texte).into_iter().collect();
        let donnes: BTreeSet<&str> = a.arguments.iter().map(String::as_str).collect();
        if trous != donnes {
            erreurs.push(format!(
                "{}:{} : trous {:?}, arguments {:?} dans « {} »",
                a.fichier, a.ligne, trous, donnes, a.texte
            ));
        }
    }
    let modeles: Vec<String> = appels.iter().map(|a| a.texte.clone()).collect();
    let mut cles: BTreeSet<String> = modeles.iter().cloned().collect();
    cles.extend(
        langue::textes_des_donnees()
            .into_iter()
            .filter(|t| !rempli_par_un_modele(t, &modeles)),
    );
    let en = anglais();
    let mut manquants = Vec::new();
    for cle in &cles {
        match en.get(cle) {
            None => manquants.push(cle.clone()),
            Some(t) => {
                let a: BTreeSet<&str> = langue::trous(cle).into_iter().collect();
                let b: BTreeSet<&str> = langue::trous(t).into_iter().collect();
                if a != b {
                    erreurs.push(format!("trous différents : « {cle} » → « {t} »"));
                }
                if t.trim().is_empty() {
                    erreurs.push(format!("traduction vide : « {cle} »"));
                }
            }
        }
    }
    let inutiles: Vec<&String> = en.keys().filter(|c| !cles.contains(*c)).collect();
    if let Ok(chemin) = std::env::var("CE_ECRIRE_TRADUCTIONS") {
        std::fs::write(&chemin, serde_json::to_string_pretty(&manquants).unwrap()).unwrap();
        eprintln!("{} textes sans traduction écrits dans {chemin}", manquants.len());
    }
    for c in &inutiles {
        erreurs.push(format!("traduction d'un texte qui n'existe plus : « {c} »"));
    }
    if !manquants.is_empty() {
        erreurs.push(format!(
            "{} textes sans traduction anglaise, par exemple « {} » (CE_ECRIRE_TRADUCTIONS=fichier.json pour la liste)",
            manquants.len(),
            manquants[0]
        ));
    }
    assert!(erreurs.is_empty(), "{}", erreurs.join("\n"));
}

#[test]
fn les_traductions_integrees_sont_celles_du_fichier() {
    let en = anglais();
    for (cle, t) in en.iter().take(50) {
        assert_eq!(
            langue::traduction(Langue::Anglais, cle).as_deref(),
            Some(t.as_str()),
            "{cle}"
        );
        assert_eq!(langue::traduire_texte(cle), cle.as_str());
    }
}
