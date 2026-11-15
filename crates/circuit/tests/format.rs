//! Format de sauvegarde JSON et messages d'erreur.

use ce_circuit::analyse::{pilotes_en_conflit, table_de_verite};
use ce_circuit::reference::bibliotheque;
use ce_circuit::{Banc, Bibliotheque, Circuit, Composant, Erreur, FORMAT};
use ce_moteur::Valeur;

#[test]
fn aller_retour_json() {
    let bib = bibliotheque();
    let texte = bib.vers_json();
    let relue = Bibliotheque::depuis_json(&texte).unwrap();
    assert_eq!(bib, relue);
    assert_eq!(texte, relue.vers_json(), "l'écriture est stable");
    // Les circuits mis à plat avant et après sauvegarde sont identiques, porte pour porte.
    for nom in bib.composants.keys() {
        let (a, b) = (
            Circuit::elaborer(&bib, nom).unwrap(),
            Circuit::elaborer(&relue, nom).unwrap(),
        );
        assert_eq!(a.netlist, b.netlist, "{nom}");
        assert_eq!(a.broches, b.broches, "{nom}");
    }
}

#[test]
fn json_ecrit_a_la_main() {
    let texte = r#"{
      "format": "computer-engineer/bibliotheque/1",
      "composants": {
        "NON": {
          "description": "Inverseur",
          "broches": [
            { "nom": "a", "sens": "entree" },
            { "nom": "s", "sens": "sortie" }
          ],
          "instances": [ { "nom": "n", "type": "NAND", "delai": 3 } ],
          "fils": [ ["a", "n.a", "n.b"], ["n.s", "s"] ]
        },
        "NON4": {
          "broches": [
            { "nom": "a", "sens": "entree", "largeur": 4 },
            { "nom": "s", "sens": "sortie", "largeur": 4 }
          ],
          "instances": [
            { "nom": "i0", "type": "NON" }, { "nom": "i1", "type": "NON" },
            { "nom": "i2", "type": "NON" }, { "nom": "i3", "type": "NON" }
          ],
          "fils": [
            ["a[0]", "i0.a"], ["a[1]", "i1.a"], ["a[2:3]", "i2.a"],
            ["i0.s", "s[0]"], ["i1.s", "s[1]"], ["i2.s", "s[2]"], ["i3.s", "s[3]"]
          ],
          "editeur": { "positions": { "i0": [10, 20] } }
        }
      }
    }"#;
    // a[2:3] a deux bits, i2.a un seul : le fil est refusé.
    let bib = Bibliotheque::depuis_json(texte).unwrap();
    assert!(matches!(
        Circuit::elaborer(&bib, "NON4"),
        Err(Erreur::LargeursDifferentes { .. })
    ));
    let corrige = texte.replace(
        r#"["a[2:3]", "i2.a"]"#,
        r#"["a[2]", "i2.a"], ["a[3]", "i3.a"]"#,
    );
    let bib = Bibliotheque::depuis_json(&corrige).unwrap();
    let mut banc = Banc::nouveau(&bib, "NON4").unwrap();
    banc.ecrire("a", 0b0101).unwrap();
    assert_eq!(banc.lire("s").unwrap(), None);
    banc.avancer(2);
    assert_eq!(
        banc.lire("s").unwrap(),
        None,
        "le délai de 3 n'est pas écoulé"
    );
    banc.avancer(1);
    assert_eq!(banc.lire("s").unwrap(), Some(0b1010));
    // Les données de l'éditeur sont conservées telles quelles.
    let relue = Bibliotheque::depuis_json(&bib.vers_json()).unwrap();
    assert_eq!(
        relue.composants["NON4"].editeur.as_ref().unwrap()["positions"]["i0"][1],
        20
    );
}

#[test]
fn fichiers_refuses() {
    assert!(matches!(
        Bibliotheque::depuis_json("{"),
        Err(Erreur::Json(_))
    ));
    assert!(matches!(
        Bibliotheque::depuis_json(r#"{"format": "autre/9", "composants": {}}"#),
        Err(Erreur::FormatInconnu(f)) if f == "autre/9"
    ));
    // Un champ inconnu est une faute de frappe probable : refusé plutôt qu'ignoré.
    let texte = format!(r#"{{"format": "{FORMAT}", "composants": {{"A": {{"broche": []}}}}}}"#);
    assert!(matches!(
        Bibliotheque::depuis_json(&texte),
        Err(Erreur::Json(_))
    ));
}

fn une(composant: Composant) -> Result<Circuit, Erreur> {
    let mut bib = bibliotheque();
    bib.ajouter("TEST", composant);
    Circuit::elaborer(&bib, "TEST")
}

fn base() -> Composant {
    Composant::nouveau("")
        .entree("a", 4)
        .sortie("s", 1)
        .inst("n", "NAND")
}

#[test]
fn erreurs_de_construction() {
    use Erreur::*;
    assert!(matches!(
        Circuit::elaborer(&bibliotheque(), "RIEN"),
        Err(ComposantInconnu { nom, dans: None }) if nom == "RIEN"
    ));
    assert!(matches!(
        une(base().inst("u", "RIEN")),
        Err(ComposantInconnu { nom, dans: Some(d) }) if nom == "RIEN" && d == "TEST"
    ));
    assert!(matches!(
        une(base().inst("n", "NAND")),
        Err(NomEnDouble { .. })
    ));
    assert!(matches!(
        une(base().entree("a", 1)),
        Err(NomEnDouble { .. })
    ));
    assert!(matches!(
        une(base().inst("x.y", "NAND")),
        Err(NomInvalide { .. })
    ));
    assert!(matches!(une(base().entree("", 1)), Err(NomInvalide { .. })));
    assert!(matches!(
        une(base().entree("z", 0)),
        Err(LargeurNulle { .. })
    ));
    assert!(matches!(
        une(base().fil(&["a[0]", "m.a"])),
        Err(InstanceInconnue { .. })
    ));
    assert!(matches!(
        une(base().fil(&["a[0]", "n.c"])),
        Err(BrocheInconnue { .. })
    ));
    assert!(matches!(
        une(base().fil(&["b", "n.a"])),
        Err(BrocheInconnue { .. })
    ));
    assert!(matches!(
        une(base().fil(&["a[4]", "n.a"])),
        Err(BitHorsBornes { largeur: 4, .. })
    ));
    assert!(matches!(
        une(base().fil(&["a", "n.a"])),
        Err(LargeursDifferentes { .. })
    ));
    assert!(matches!(
        une(base().fil(&["a[0", "n.a"])),
        Err(Syntaxe { .. })
    ));
    assert!(matches!(
        une(base().fil(&["a[z]", "n.a"])),
        Err(Syntaxe { .. })
    ));
    assert!(matches!(
        une(base().fil(&["n.a.b", "s"])),
        Err(Syntaxe { .. })
    ));
    assert!(matches!(
        une(base().inst_delai("m", "NAND", 0)),
        Err(Parametre { .. })
    ));
    assert!(matches!(
        une(base().inst_delai("m", "NON", 2)),
        Err(Parametre { .. })
    ));
    assert!(matches!(
        une(base().inst("h", "HORLOGE")),
        Err(Parametre { .. })
    ));
    assert!(matches!(une(base().horloge("h", 0)), Err(Parametre { .. })));
    let mut bib = bibliotheque();
    bib.ajouter("NAND", base());
    assert!(matches!(
        Circuit::elaborer(&bib, "NAND"),
        Err(NomReserve(_))
    ));
    // Le message nomme le composant fautif, en français.
    let message = une(base().fil(&["a[7]", "n.a"])).unwrap_err().to_string();
    assert_eq!(
        message,
        "dans « TEST » : « a[7] » dépasse la largeur de la broche (4 bits)"
    );
}

#[test]
fn dependance_circulaire() {
    let mut bib = Bibliotheque::nouvelle();
    let avec = |fils: &str| Composant::nouveau("").inst("u", fils);
    bib.ajouter("A", avec("B"));
    bib.ajouter("B", avec("C"));
    bib.ajouter("C", avec("A"));
    bib.ajouter("SEUL", avec("SEUL"));
    match Circuit::elaborer(&bib, "A") {
        Err(Erreur::DependanceCirculaire(cycle)) => assert_eq!(cycle, ["A", "B", "C", "A"]),
        autre => panic!("attendu un cycle, obtenu {autre:?}"),
    }
    assert_eq!(
        Circuit::elaborer(&bib, "SEUL").unwrap_err().to_string(),
        "dépendance circulaire : SEUL → SEUL"
    );
    assert!(bib.valider().is_err());
}

/// Un même point cité dans deux fils : les deux fils n'en font qu'un.
#[test]
fn deux_fils_qui_se_touchent_sont_un_seul_net() {
    let c = une(Composant::nouveau("")
        .entree("a", 1)
        .sortie("s", 1)
        .inst("n", "NAND")
        .fil(&["a", "n.a"])
        .fil(&["n.b", "n.a"])
        .fil(&["n.s", "s"]))
    .unwrap();
    assert_eq!(c.net("n.b").unwrap(), c.net("a").unwrap());
    assert_eq!(c.netlist.nb_nets, 2);
}

#[test]
fn plages_de_bits_et_ordre() {
    let mut bib = Bibliotheque::nouvelle();
    bib.ajouter(
        "RETOURNE",
        Composant::nouveau("Inverse l'ordre des 4 bits")
            .entree("a", 4)
            .sortie("s", 4)
            .fil(&["a[0:3]", "s[3:0]"]),
    );
    let table = table_de_verite(&bib, "RETOURNE").unwrap();
    for (i, ligne) in table.lignes.iter().enumerate() {
        assert_eq!(ligne.len(), 4);
        for (k, v) in ligne.iter().enumerate() {
            assert_eq!(*v, Valeur::de_bool((i >> (3 - k)) & 1 == 1));
        }
    }
}

#[test]
fn court_circuit_structurel_detecte() {
    let c = une(Composant::nouveau("Deux sorties de portes reliées")
        .entree("a", 1)
        .sortie("s", 1)
        .inst("n1", "NAND")
        .inst("n2", "NON")
        .fil(&["a", "n1.a", "n1.b", "n2.a"])
        .fil(&["n1.s", "n2.s", "s"]))
    .unwrap();
    assert_eq!(pilotes_en_conflit(&c.netlist), vec![c.net("s").unwrap()]);
    let sain = Circuit::elaborer(&bibliotheque(), "SELECTEUR_BUS").unwrap();
    assert!(
        pilotes_en_conflit(&sain.netlist).is_empty(),
        "des tampons trois états ne sont pas un conflit"
    );
}
