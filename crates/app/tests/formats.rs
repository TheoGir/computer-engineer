//! Les fichiers enregistrés restent lisibles d'une version à l'autre (`docs/formats.md`).
//!
//! - Le corpus `tests/donnees/format-<n>/` garde de vrais fichiers de chaque format : ils
//!   doivent toujours s'ouvrir, quelle que soit la version de l'application.
//! - La forme de ce qu'écrit l'application (tous ses chemins de clés) est comparée à son relevé
//!   `tests/donnees/forme-*.txt` : un champ ajouté, retiré ou renommé fait échouer l'épreuve
//!   tant que le numéro de format n'a pas été augmenté avec sa migration.
//! - Les clés des quêtes réussies, enregistrées dans `parcours.json`, ne disparaissent jamais.
//! - Un fichier abîmé (octets changés, coupé, mélangé) ne fait jamais planter la lecture.
//!
//! `CE_ECRIRE_DONNEES=1 cargo test -p ce-app --test formats` écrit ce qui manque (corpus du
//! format actuel, relevés, liste des clés) sans jamais remplacer un fichier existant.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use ce_app::apparence::Apparence;
use ce_app::brique::{self, Projet};
use ce_app::document::{Document, Element, Point};
use ce_app::etat::Atelier;
use ce_app::exemples;
use ce_app::format::{self, Lecture};
use ce_app::parcours::Parcours;
use ce_app::quete::PARCOURS;
use ce_app::reglages::Reglages;
use ce_app::sauvegarde::Dossier;

fn donnees() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("donnees")
}

fn ecrire_si_demande(chemin: &Path, texte: &str) -> bool {
    if std::env::var_os("CE_ECRIRE_DONNEES").is_some() && !chemin.exists() {
        fs::create_dir_all(chemin.parent().unwrap()).unwrap();
        fs::write(chemin, texte).unwrap();
        eprintln!("écrit : {}", chemin.display());
        return true;
    }
    false
}

/// Un projet qui contient tout ce qu'un projet peut contenir : chaque genre d'élément, des
/// briques emboîtées, des bus, une quête.
fn projet_complet() -> Projet {
    let a = Apparence::default();
    let mut at = Atelier::avec(Projet::nouveau("Tout", Document::demi_additionneur()), &a);
    let ids: Vec<_> = at
        .doc
        .elements
        .iter()
        .filter(|e| brique::groupable(e.genre))
        .map(|e| e.id)
        .collect();
    let mut plan = at.doc.clone();
    let (modele, _) = brique::regrouper(&mut plan, &ids, "DEMI").expect("regroupement");
    at.projet.ajouter(modele);
    let mut principal = plan;
    principal.poser(Element::clavier("clavier"), Point::nouveau(40.0, 400.0));
    principal.poser(Element::ecran("ecran"), Point::nouveau(600.0, 400.0));
    principal.poser(Element::impulsion("raz", 10), Point::nouveau(40.0, 500.0));
    // Les aspects : un bouton poussoir et un afficheur 7 segments, dans le plan principal et
    // dans une brique.
    let mut affichage = Document::nouveau();
    affichage.poser(Element::poussoir("p"), Point::nouveau(40.0, 40.0));
    affichage.poser(
        Element::afficheur("seg", "chiffre"),
        Point::nouveau(240.0, 40.0),
    );
    principal.poser(Element::poussoir("ok"), Point::nouveau(40.0, 600.0));
    principal.poser(
        Element::afficheur("chiffre", ""),
        Point::nouveau(600.0, 600.0),
    );
    let mut projet = at.projet.clone();
    projet.ajouter(brique::modele_du_plan(&affichage, "AFFICHAGE"));
    projet.principal = principal;
    projet.quete = Some(PARCOURS[0].cle.to_owned());
    // Les bancs d'essai : celui du plan principal et celui d'une brique.
    projet
        .bancs
        .insert(String::new(), "ok -> chiffre\n0 -> -\n".to_owned());
    projet
        .bancs
        .insert("DEMI".to_owned(), "a b -> s\n1 1 -> 0\n".to_owned());
    // L'atelier de l'ordinateur apporte les briques à plusieurs niveaux, découpeurs et
    // jonctions compris.
    let ordi = exemples::atelier_de_l_ordinateur(
        "o",
        ce_circuit::ordinateur::programmes::CALCULATRICE,
        false,
    )
    .expect("atelier de l'ordinateur");
    for m in ordi.modeles {
        if projet.modele(&m.nom).is_none() {
            projet.ajouter(m);
        }
    }
    projet
}

fn parcours_complet() -> Parcours {
    let mut p = Parcours::default();
    for f in PARCOURS.iter().take(3) {
        p.faites.insert(f.cle.to_owned());
    }
    p.briques = projet_complet().modeles;
    p
}

fn forme_du_texte(texte: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(texte).unwrap();
    let mut s: String = format::forme(&v).into_iter().map(|l| l + "\n").collect();
    if s.is_empty() {
        s.push('\n');
    }
    s
}

fn comparer_la_forme(nom: &str, texte: &str, identifiant: &str) {
    let chemin = donnees().join(format!("forme-{nom}.txt"));
    let attendue = format!("# {identifiant}\n{}", forme_du_texte(texte));
    ecrire_si_demande(&chemin, &attendue);
    let relevee = fs::read_to_string(&chemin).unwrap_or_else(|_| {
        panic!(
            "relevé {} absent : lancer CE_ECRIRE_DONNEES=1 cargo test -p ce-app --test formats",
            chemin.display()
        )
    });
    assert!(
        relevee == attendue,
        "La forme écrite d'un fichier « {nom} » a changé.\n\
         Si c'est voulu : augmenter le numéro de format ({identifiant}), écrire la migration \
         dans crates/app/src/format.rs, effacer {} puis relancer avec CE_ECRIRE_DONNEES=1 \
         (cela écrit aussi le corpus du nouveau format). Voir docs/formats.md.\n\
         Relevé :\n{relevee}\nAujourd'hui :\n{attendue}",
        chemin.display()
    );
}

#[test]
fn la_forme_des_projets_n_a_pas_change_sans_nouveau_format() {
    comparer_la_forme(
        "projet",
        &projet_complet().en_json(),
        &format::PROJET.identifiant(),
    );
}

#[test]
fn la_forme_du_parcours_n_a_pas_change_sans_nouveau_format() {
    comparer_la_forme(
        "parcours",
        &parcours_complet().en_json(),
        &format::PARCOURS.identifiant(),
    );
}

/// Les fichiers du corpus, par format : `(numéro, chemin)`.
fn corpus(prefixe: &str) -> Vec<(u32, PathBuf)> {
    let mut tous = Vec::new();
    let Ok(dossiers) = fs::read_dir(donnees()) else {
        return tous;
    };
    for d in dossiers.flatten() {
        let nom = d.file_name().to_string_lossy().into_owned();
        let Some(n) = nom.strip_prefix("format-").and_then(|n| n.parse().ok()) else {
            continue;
        };
        for f in fs::read_dir(d.path()).unwrap().flatten() {
            let fichier = f.file_name().to_string_lossy().into_owned();
            if fichier.starts_with(prefixe) && fichier.ends_with(".json") {
                tous.push((n, f.path()));
            }
        }
    }
    tous.sort();
    tous
}

#[test]
fn le_corpus_du_format_actuel_existe() {
    let ici = donnees().join(format!("format-{}", format::PROJET.actuel));
    ecrire_si_demande(&ici.join("projet-tout.json"), &projet_complet().en_json());
    ecrire_si_demande(
        &ici.join("projet-demi-additionneur.json"),
        &Projet::nouveau("Demi-additionneur", Document::demi_additionneur()).en_json(),
    );
    // L'avancement a son propre numéro : son corpus va dans le dossier de ce numéro.
    let ici_parcours = donnees().join(format!("format-{}", format::PARCOURS.actuel));
    ecrire_si_demande(
        &ici_parcours.join("parcours.json"),
        &parcours_complet().en_json(),
    );
    // Les réglages n'ont pas de numéro : un seul exemplaire, dans format-1, suffit.
    assert!(corpus("projet")
        .iter()
        .any(|(n, _)| *n == format::PROJET.actuel));
    assert!(corpus("parcours")
        .iter()
        .any(|(n, _)| *n == format::PARCOURS.actuel));
}

#[test]
fn tous_les_projets_du_corpus_s_ouvrent_et_se_relisent_a_l_identique() {
    let a = Apparence::default();
    let projets = corpus("projet");
    assert!(projets.len() >= 2, "corpus vide : {:?}", donnees());
    for (n, chemin) in projets {
        let texte = fs::read_to_string(&chemin).unwrap();
        let (projet, lecture) =
            Projet::de_json_lu(&texte).unwrap_or_else(|e| panic!("{} : {e}", chemin.display()));
        let attendu = if n == format::PROJET.actuel {
            Lecture::Actuel
        } else {
            Lecture::Migre(n)
        };
        assert_eq!(lecture, attendu, "{}", chemin.display());
        assert_eq!(projet.format, format::PROJET.identifiant());
        let relu = Projet::de_json(&projet.en_json()).unwrap();
        assert_eq!(relu, projet, "{}", chemin.display());
        // Il s'ouvre aussi dans l'atelier, simulation comprise.
        let at = Atelier::avec(projet, &a);
        assert_eq!(at.projet.nom, relu.nom);
    }
}

#[test]
fn tous_les_parcours_du_corpus_se_relisent() {
    let parcours = corpus("parcours");
    assert!(!parcours.is_empty());
    for (_, chemin) in parcours {
        let texte = fs::read_to_string(&chemin).unwrap();
        let p = Parcours::de_json(&texte).unwrap_or_else(|e| panic!("{} : {e}", chemin.display()));
        assert!(!p.faites.is_empty());
        for cle in &p.faites {
            assert!(
                PARCOURS.iter().any(|f| f.cle == cle),
                "{} : la quête « {cle} » n'existe plus",
                chemin.display()
            );
        }
    }
    for (_, chemin) in corpus("reglages") {
        let (_, complet) = Reglages::lire(&fs::read_to_string(&chemin).unwrap());
        assert!(complet, "{}", chemin.display());
    }
}

#[test]
fn les_cles_des_quetes_ne_disparaissent_jamais() {
    let chemin = donnees().join("cles-de-quetes.txt");
    let aujourd_hui: BTreeSet<&str> = PARCOURS.iter().map(|f| f.cle).collect();
    let texte: String = PARCOURS.iter().map(|f| format!("{}\n", f.cle)).collect();
    ecrire_si_demande(&chemin, &texte);
    let gelees = fs::read_to_string(&chemin).expect("cles-de-quetes.txt");
    let disparues: Vec<&str> = gelees
        .lines()
        .filter(|c| !c.is_empty() && !aujourd_hui.contains(c))
        .collect();
    assert!(
        disparues.is_empty(),
        "Ces quêtes ont disparu ou changé de clé : {disparues:?}. Leur réussite, notée dans \
         parcours.json, serait perdue : garder la clé (le titre peut changer), ou écrire une \
         migration du parcours."
    );
    // Les nouvelles quêtes s'ajoutent à la fin de la liste, dans l'ordre du parcours.
    let mut manquantes: Vec<&str> = PARCOURS
        .iter()
        .map(|f| f.cle)
        .filter(|c| !gelees.lines().any(|l| l == *c))
        .collect();
    if std::env::var_os("CE_ECRIRE_DONNEES").is_some() && !manquantes.is_empty() {
        let ajout: String = manquantes.iter().map(|c| format!("{c}\n")).collect();
        fs::write(&chemin, format!("{gelees}{ajout}")).unwrap();
        eprintln!("ajouté à {} : {manquantes:?}", chemin.display());
        manquantes.clear();
    }
    assert!(
        manquantes.is_empty(),
        "Nouvelles quêtes à ajouter à la fin de {} : {manquantes:?}",
        chemin.display()
    );
}

#[test]
fn un_fichier_plus_recent_n_est_ni_ouvert_ni_ecrase() {
    let racine = std::env::temp_dir().join(format!("ce-formats-recent-{}", std::process::id()));
    let _ = fs::remove_dir_all(&racine);
    let dossier = Dossier::dans(&racine);
    let projet = Projet::nouveau("Futur", Document::demi_additionneur());
    let fichier = dossier.enregistrer(&projet).unwrap();
    let chemin = racine.join("projets").join(format!("{fichier}.json"));
    let futur = projet.en_json().replace(
        &format::PROJET.identifiant(),
        &format!("{}{}", format::PROJET.prefixe, format::PROJET.actuel + 1),
    );
    fs::write(&chemin, &futur).unwrap();
    let e = dossier.ouvrir(&fichier).unwrap_err();
    assert!(e.contains("plus récente"), "{e}");
    assert_eq!(fs::read_to_string(&chemin).unwrap(), futur);
    // Rouvert au lancement comme dernier projet : l'application le laisse tel quel.
    dossier.noter_dernier(&fichier).unwrap();
    let mut app = ce_app::app::Application::nouvelle(
        Apparence::default(),
        ce_app::apparence::Surveillance::default(),
    )
    .avec_dossier(Dossier::dans(&racine));
    app.enregistrer();
    assert_eq!(fs::read_to_string(&chemin).unwrap(), futur);
    let _ = fs::remove_dir_all(&racine);
}

/// Petit générateur pseudo-aléatoire reproductible (xorshift).
struct Alea(u64);
impl Alea {
    fn suivant(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn sous(&mut self, n: usize) -> usize {
        (self.suivant() % n.max(1) as u64) as usize
    }
}

/// Une version abîmée de `texte` : quelques octets changés, coupés, dupliqués ou permutés.
fn abimer(texte: &[u8], alea: &mut Alea) -> Vec<u8> {
    let mut t = texte.to_vec();
    for _ in 0..1 + alea.sous(4) {
        if t.is_empty() {
            break;
        }
        let i = alea.sous(t.len());
        match alea.sous(6) {
            0 => t[i] = alea.suivant() as u8,
            1 => t[i] = b"{}[],:\"0123456789-e.tfn"[alea.sous(23)],
            2 => t.truncate(i),
            3 => {
                let j = (i + alea.sous(200)).min(t.len());
                t.drain(i..j);
            }
            4 => {
                let j = (i + alea.sous(200)).min(t.len());
                let copie = t[i..j].to_vec();
                let k = alea.sous(t.len());
                t.splice(k..k, copie);
            }
            _ => {
                // Un nombre remplacé par un très grand ou un négatif.
                let pris = if alea.sous(2) == 0 {
                    "4294967296"
                } else {
                    "-1"
                };
                if t[i].is_ascii_digit() {
                    t.splice(i..i + 1, pris.bytes());
                }
            }
        }
    }
    t
}

#[test]
fn des_fichiers_abimes_ne_font_jamais_planter() {
    let a = Apparence::default();
    let mut alea = Alea(0x9e37_79b9_7f4a_7c15);
    let fichiers: Vec<(String, Vec<u8>)> = ["projet", "parcours", "reglages"]
        .iter()
        .flat_map(|p| corpus(p))
        .map(|(_, c)| {
            (
                c.file_name().unwrap().to_string_lossy().into_owned(),
                fs::read(c).unwrap(),
            )
        })
        .collect();
    assert!(!fichiers.is_empty());
    let mut ouverts = 0;
    for essai in 0..600 {
        let (nom, original) = &fichiers[essai % fichiers.len()];
        let abime = abimer(original, &mut alea);
        let texte = String::from_utf8_lossy(&abime);
        if nom.starts_with("projet") {
            if let Ok(p) = Projet::de_json(&texte) {
                ouverts += 1;
                // Ce qui s'ouvre doit aussi s'afficher et se simuler sans planter.
                if essai % 3 == 0 {
                    let _ = Atelier::avec(p, &a);
                }
            }
        } else if nom.starts_with("parcours") {
            let _ = Parcours::de_json(&texte);
        } else {
            let _ = Reglages::lire(&texte);
        }
    }
    eprintln!("{ouverts} projets abîmés encore lisibles, aucun plantage");
}

/// Un dossier de données sous un nom d'utilisateur accentué (`C:\Users\Zoé`), un projet au
/// nom accentué et avec des signes : tout s'enregistre, se liste et se rouvre.
#[test]
fn chemins_et_noms_accentues() {
    let racine = std::env::temp_dir()
        .join(format!("ce-formats-accents-{}", std::process::id()))
        .join("Users")
        .join("Zoé Ünïcødé 名前");
    let _ = fs::remove_dir_all(&racine);
    let dossier = Dossier::dans(&racine);
    let mut projet = Projet::nouveau(
        "Additionneur à retenue — essai n°2 ✓",
        Document::demi_additionneur(),
    );
    projet.quete = None;
    let fichier = dossier.enregistrer(&projet).unwrap();
    assert!(dossier.lister().contains(&fichier));
    let relu = dossier.ouvrir(&fichier).unwrap();
    assert_eq!(relu.principal, projet.principal);
    let rapports = ce_app::incident::dossier_des_rapports(&racine);
    let p = ce_app::incident::ecrire_un_rapport(&rapports, "plantage", "é").unwrap();
    assert_eq!(fs::read_to_string(p).unwrap(), "é");
    let _ = fs::remove_dir_all(racine.parent().unwrap().parent().unwrap());
}

/// Migration 1 → 2 : la mémoire morte d'un projet du format 1 retrouve son programme, et
/// l'original est gardé dans `avant-migration` avant la première réécriture.
#[test]
fn un_projet_du_format_1_migre_garde_son_original_et_retrouve_son_programme() {
    let original = fs::read_to_string(donnees().join("format-1").join("projet-tout.json")).unwrap();
    let (p, lecture) = Projet::de_json_lu(&original).unwrap();
    assert_eq!(lecture, Lecture::Migre(1));
    assert_eq!(
        p.modele(ce_app::programme::MEMOIRE_MORTE)
            .and_then(|m| m.programme.as_deref()),
        Some(ce_circuit::ordinateur::programmes::CALCULATRICE)
    );
    assert!(p.modeles.iter().filter(|m| m.programme.is_some()).count() == 1);

    let racine = std::env::temp_dir().join(format!("ce-formats-migration-{}", std::process::id()));
    let _ = fs::remove_dir_all(&racine);
    fs::create_dir_all(racine.join("projets")).unwrap();
    fs::write(racine.join("projets").join("Tout.json"), &original).unwrap();
    let dossier = Dossier::dans(&racine);
    let ouvert = dossier.ouvrir("Tout").unwrap();
    let garde = dossier.avant_migration().join("Tout.format-1.json");
    assert_eq!(fs::read_to_string(&garde).unwrap(), original);
    dossier.enregistrer_sous(&ouvert, "Tout").unwrap();
    // Rouvert, il est au format actuel ; la copie d'origine reste telle quelle.
    let (_, lecture) =
        Projet::de_json_lu(&fs::read_to_string(racine.join("projets").join("Tout.json")).unwrap())
            .unwrap();
    assert_eq!(lecture, Lecture::Actuel);
    assert_eq!(fs::read_to_string(&garde).unwrap(), original);
    let _ = fs::remove_dir_all(&racine);
}
