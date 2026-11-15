//! Chaque composant de la bibliothèque de référence est comparé à sa définition
//! mathématique. Les circuits combinatoires le sont exhaustivement.

use ce_circuit::analyse::{chemin_critique, equivalence, table_de_verite, Equivalence};
use ce_circuit::reference::bibliotheque;
use ce_circuit::{Banc, Bibliotheque, Circuit, Composant};
use ce_moteur::alea::Alea;
use ce_moteur::Valeur;

/// Vérifie la table de vérité de `nom` contre `f(entrées) = sorties`, les deux
/// codés en entiers comme dans `TableDeVerite::lignes`.
fn verifier_table(nom: &str, f: impl Fn(u64) -> u64) -> u64 {
    let table = table_de_verite(&bibliotheque(), nom).unwrap();
    for (i, ligne) in table.lignes.iter().enumerate() {
        let attendu = f(i as u64);
        for (k, v) in ligne.iter().enumerate() {
            assert_eq!(
                *v,
                Valeur::de_bool((attendu >> k) & 1 == 1),
                "{nom} : entrées {i:#b}, bit de sortie {k}"
            );
        }
    }
    table.lignes.len() as u64
}

#[test]
fn portes_de_base() {
    assert_eq!(verifier_table("NON", |e| !e & 1), 2);
    assert_eq!(verifier_table("ET", |e| (e & (e >> 1)) & 1), 4);
    assert_eq!(verifier_table("OU", |e| (e | (e >> 1)) & 1), 4);
    assert_eq!(verifier_table("NON_OU", |e| !(e | (e >> 1)) & 1), 4);
    assert_eq!(verifier_table("OU_X", |e| (e ^ (e >> 1)) & 1), 4);
    // Entrées : a (bit 0), b (bit 1), sel (bit 2).
    assert_eq!(
        verifier_table("MUX", |e| if e & 4 == 0 { e & 1 } else { (e >> 1) & 1 }),
        8
    );
    assert_eq!(
        verifier_table("SELECTEUR_BUS", |e| if e & 4 == 0 {
            e & 1
        } else {
            (e >> 1) & 1
        }),
        8
    );
}

#[test]
fn additionneurs_un_bit() {
    // Sorties : s (bit 0), r (bit 1) : la somme arithmétique elle-même.
    verifier_table("DEMI_ADD", |e| (e & 1) + ((e >> 1) & 1));
    verifier_table("ADD_COMPLET", |e| (e & 1) + ((e >> 1) & 1) + ((e >> 2) & 1));
}

/// Les 131 072 additions possibles sur 8 bits avec retenue entrante.
#[test]
fn additionneur_8_bits_exhaustif() {
    // Entrées : a (bits 0-7), b (bits 8-15), re (bit 16). Sorties : s (0-7), rs (8).
    let n = verifier_table("ADD8", |e| (e & 0xFF) + ((e >> 8) & 0xFF) + ((e >> 16) & 1));
    assert_eq!(n, 1 << 17);
}

#[test]
fn nombres_de_portes() {
    let bib = bibliotheque();
    let attendu = [
        ("NON", 1),
        ("ET", 2),
        ("OU", 3),
        ("NON_OU", 4),
        ("OU_X", 4),
        ("MUX", 4),
        ("DEMI_ADD", 5),
        ("ADD_COMPLET", 9),
        ("ADD8", 72),
        ("VERROU_SR", 2),
        ("VERROU_D", 4),
        ("BASCULE_D", 9),
        ("REGISTRE8", 104),
        ("COMPTEUR4", 65),
        ("OSCILLATEUR", 3),
        ("SELECTEUR_BUS", 3),
    ];
    assert_eq!(bib.composants.len(), attendu.len());
    for (nom, n) in attendu {
        let c = Circuit::elaborer(&bib, nom).unwrap();
        assert_eq!(c.nb_portes(), n, "{nom}");
        assert!(
            c.avertissements.is_empty(),
            "{nom} : {:?}",
            c.avertissements
        );
    }
    bib.valider().unwrap();
}

#[test]
fn chemins_critiques() {
    let bib = bibliotheque();
    let critique = |nom: &str| chemin_critique(&Circuit::elaborer(&bib, nom).unwrap().netlist);
    assert_eq!(critique("NON"), Some(1));
    assert_eq!(critique("OU_X"), Some(3));
    assert_eq!(critique("ADD_COMPLET"), Some(6));
    // Retenue : 3 portes pour a0 xor b0, puis 2 par étage jusqu'à l'étage 7 (2 × 7),
    // puis 3 pour la somme du dernier étage ou 2 pour la retenue sortante.
    assert_eq!(critique("ADD8"), Some(3 + 2 * 7 + 3));
    for sequentiel in [
        "VERROU_SR",
        "VERROU_D",
        "BASCULE_D",
        "REGISTRE8",
        "COMPTEUR4",
        "OSCILLATEUR",
    ] {
        assert_eq!(
            critique(sequentiel),
            None,
            "{sequentiel} contient une boucle"
        );
        assert!(table_de_verite(&bib, sequentiel).is_err());
    }
}

/// Après un changement d'entrées, un circuit sans boucle est figé au plus tard
/// après son chemin critique, et il existe un cas où il met exactement ce temps.
#[test]
fn le_chemin_critique_borne_le_temps_d_etablissement() {
    let bib = bibliotheque();
    let mut banc = Banc::nouveau(&bib, "ADD8").unwrap();
    let critique = chemin_critique(&banc.circuit.netlist).unwrap();
    let mut alea = Alea::nouveau(42);
    let mut pire = 0;
    for _ in 0..3000 {
        banc.ecrire("a", alea.sous(256)).unwrap();
        banc.ecrire("b", alea.sous(256)).unwrap();
        banc.ecrire("re", alea.sous(2)).unwrap();
        let t0 = banc.sim.temps();
        assert!(banc.stabiliser(1000).est_stable());
        let duree = banc.sim.temps() - t0;
        assert!(duree <= critique, "{duree} > {critique}");
        pire = pire.max(duree);
    }
    // Propagation de retenue la plus longue : 0xFF + 0x00, puis re passe de 0 à 1.
    banc.ecrire("a", 0xFF).unwrap();
    banc.ecrire("b", 0).unwrap();
    banc.ecrire("re", 0).unwrap();
    banc.stabiliser(1000);
    banc.ecrire("re", 1).unwrap();
    let t0 = banc.sim.temps();
    banc.stabiliser(1000);
    assert_eq!(banc.lire("s").unwrap(), Some(0));
    assert_eq!(banc.lire("rs").unwrap(), Some(1));
    assert_eq!(
        banc.sim.temps() - t0,
        2 * 7 + 3,
        "la retenue traverse les 8 étages"
    );
    assert!(pire >= 10);
}

/// Un bit inconnu ne contamine que ce qui en dépend vraiment.
#[test]
fn un_x_ne_contamine_que_ce_qui_en_depend() {
    let bib = bibliotheque();
    let mut banc = Banc::nouveau(&bib, "ADD8").unwrap();
    let mut a = vec![Valeur::Zero; 8];
    a[0] = Valeur::Un;
    a[7] = Valeur::X;
    banc.ecrire_valeurs("a", &a).unwrap();
    banc.ecrire("b", 1).unwrap();
    banc.ecrire("re", 0).unwrap();
    assert!(banc.stabiliser(1000).est_stable());
    assert_eq!(banc.lire_texte("s").unwrap(), "x0000010");
    assert_eq!(banc.lire("s[0:6]").unwrap(), Some(2));
    assert_eq!(banc.lire("s").unwrap(), None);
    // La retenue sortante vaut 0 quel que soit a[7], et le simulateur le sait.
    assert_eq!(banc.lire("rs").unwrap(), Some(0));
}

#[test]
fn chemins_hierarchiques() {
    let bib = bibliotheque();
    let mut banc = Banc::nouveau(&bib, "ADD8").unwrap();
    banc.ecrire("a", 0b0000_1000).unwrap();
    banc.ecrire("b", 0).unwrap();
    banc.ecrire("re", 0).unwrap();
    banc.stabiliser(1000);
    let c = &banc.circuit;
    // n4 de l'étage 3 calcule a[3] xor b[3].
    assert_eq!(banc.lire("add3.n4.s").unwrap(), Some(1));
    assert_eq!(banc.lire("add2.n4.s").unwrap(), Some(0));
    // Une broche d'instance et le fil du parent qui y est relié sont le même net.
    assert_eq!(c.net("add3.s").unwrap(), c.net("s[3]").unwrap());
    assert_eq!(c.net("add3.a").unwrap(), c.net("a[3]").unwrap());
    assert_eq!(c.net("add3.n1.a").unwrap(), c.net("a[3]").unwrap());
    assert_eq!(c.net("add3.rs").unwrap(), c.net("add4.re").unwrap());
    assert_eq!(c.net("add4.n5.b").unwrap(), c.net("add3.n9.s").unwrap());
    assert_eq!(c.nets("s").unwrap().len(), 8);
    assert_eq!(
        c.nets("s[7:6]").unwrap(),
        vec![c.net("s[7]").unwrap(), c.net("s[6]").unwrap()]
    );
    // Tous les nets internes sont distincts.
    let mut vus = std::collections::HashSet::new();
    for i in 0..8 {
        for n in 1..=9 {
            assert!(vus.insert(c.net(&format!("add{i}.n{n}.s")).unwrap()));
        }
    }
    assert_eq!(vus.len(), 72);
    for mauvais in ["add9.s", "add3.zz", "add3.n4.q", "s[8]", "add3.n4", "s", ""] {
        assert!(c.net(mauvais).is_err(), "« {mauvais} » devrait être refusé");
    }
}

#[test]
fn verrou_sr() {
    let mut banc = Banc::nouveau(&bibliotheque(), "VERROU_SR").unwrap();
    let mut appliquer = |s_n: u64, r_n: u64| {
        banc.ecrire("s_n", s_n).unwrap();
        banc.ecrire("r_n", r_n).unwrap();
        assert!(banc.stabiliser(100).est_stable());
        (banc.lire("q").unwrap(), banc.lire("q_n").unwrap())
    };
    assert_eq!(appliquer(1, 1), (None, None), "jamais écrit : inconnu");
    assert_eq!(appliquer(0, 1), (Some(1), Some(0)), "mise à 1");
    assert_eq!(appliquer(1, 1), (Some(1), Some(0)), "mémoire");
    assert_eq!(appliquer(1, 0), (Some(0), Some(1)), "mise à 0");
    assert_eq!(appliquer(1, 1), (Some(0), Some(1)), "mémoire");
    assert_eq!(appliquer(0, 0), (Some(1), Some(1)), "état interdit");
}

#[test]
fn verrou_d() {
    let mut banc = Banc::nouveau(&bibliotheque(), "VERROU_D").unwrap();
    let mut appliquer = |d: u64, e: u64| {
        banc.ecrire("d", d).unwrap();
        banc.ecrire("e", e).unwrap();
        assert!(banc.stabiliser(100).est_stable());
        (banc.lire("q").unwrap(), banc.lire("q_n").unwrap())
    };
    assert_eq!(appliquer(1, 0), (None, None), "jamais écrit");
    assert_eq!(appliquer(1, 1), (Some(1), Some(0)), "transparent");
    assert_eq!(appliquer(0, 1), (Some(0), Some(1)), "transparent");
    assert_eq!(appliquer(0, 0), (Some(0), Some(1)), "fermé");
    assert_eq!(appliquer(1, 0), (Some(0), Some(1)), "fermé : d ignoré");
    assert_eq!(appliquer(1, 1), (Some(1), Some(0)));
    assert_eq!(appliquer(1, 0), (Some(1), Some(0)));
    assert_eq!(appliquer(0, 0), (Some(1), Some(0)));
}

/// La bascule ne recopie `d` qu'au front montant, jamais ailleurs.
#[test]
fn bascule_d_sur_front_montant() {
    let mut banc = Banc::nouveau(&bibliotheque(), "BASCULE_D").unwrap();
    let mut alea = Alea::nouveau(7);
    banc.ecrire("clk", 0).unwrap();
    banc.ecrire("d", 0).unwrap();
    banc.avancer(20);
    assert_eq!(
        banc.lire("q").unwrap(),
        None,
        "inconnue avant le premier front"
    );
    let mut q_attendu = None;
    for _ in 0..500 {
        // Niveau bas : d bouge librement, q ne doit pas bouger.
        for _ in 0..3 {
            banc.ecrire("d", alea.sous(2)).unwrap();
            banc.avancer(alea.entre(1, 6));
            assert_eq!(banc.lire("q").unwrap(), q_attendu);
        }
        let d = alea.sous(2);
        banc.ecrire("d", d).unwrap();
        banc.avancer(10);
        // Front montant.
        banc.ecrire("clk", 1).unwrap();
        banc.avancer(10);
        q_attendu = Some(d);
        assert_eq!(banc.lire("q").unwrap(), q_attendu);
        assert_eq!(banc.lire("q_n").unwrap(), Some(1 - d));
        // Niveau haut : d bouge librement, q ne doit pas bouger.
        for _ in 0..3 {
            banc.ecrire("d", alea.sous(2)).unwrap();
            banc.avancer(alea.entre(1, 6));
            assert_eq!(banc.lire("q").unwrap(), q_attendu);
        }
        // Front descendant : rien ne change.
        banc.ecrire("clk", 0).unwrap();
        banc.avancer(10);
        assert_eq!(banc.lire("q").unwrap(), q_attendu);
    }
}

fn coup_d_horloge(banc: &mut Banc) {
    banc.avancer(40);
    banc.ecrire("clk", 1).unwrap();
    banc.avancer(40);
    banc.ecrire("clk", 0).unwrap();
}

#[test]
fn registre_8_bits() {
    let mut banc = Banc::nouveau(&bibliotheque(), "REGISTRE8").unwrap();
    let mut alea = Alea::nouveau(11);
    banc.ecrire("clk", 0).unwrap();
    banc.ecrire("charge", 0).unwrap();
    banc.ecrire("d", 0x5A).unwrap();
    coup_d_horloge(&mut banc);
    assert_eq!(banc.lire_texte("q").unwrap(), "xxxxxxxx", "jamais chargé");
    let mut contenu = None;
    for _ in 0..300 {
        let d = alea.sous(256);
        let charge = alea.sous(2);
        banc.ecrire("d", d).unwrap();
        banc.ecrire("charge", charge).unwrap();
        coup_d_horloge(&mut banc);
        if charge == 1 {
            contenu = Some(d);
        }
        assert_eq!(banc.lire("q").unwrap(), contenu);
    }
    assert!(contenu.is_some());
}

#[test]
fn compteur_4_bits() {
    let mut banc = Banc::nouveau(&bibliotheque(), "COMPTEUR4").unwrap();
    banc.ecrire("clk", 0).unwrap();
    banc.ecrire("raz", 0).unwrap();
    coup_d_horloge(&mut banc);
    coup_d_horloge(&mut banc);
    assert_eq!(
        banc.lire_texte("q").unwrap(),
        "xxxx",
        "sans remise à zéro, l'état reste inconnu"
    );
    banc.ecrire("raz", 1).unwrap();
    coup_d_horloge(&mut banc);
    assert_eq!(banc.lire("q").unwrap(), Some(0));
    banc.ecrire("raz", 0).unwrap();
    for k in 1..=40u64 {
        coup_d_horloge(&mut banc);
        assert_eq!(banc.lire("q").unwrap(), Some(k % 16));
    }
    banc.ecrire("raz", 1).unwrap();
    coup_d_horloge(&mut banc);
    assert_eq!(banc.lire("q").unwrap(), Some(0));
}

/// Même compteur, mais cadencé par une horloge interne au circuit.
#[test]
fn compteur_cadence_par_une_horloge() {
    let mut bib = bibliotheque();
    bib.ajouter(
        "MONTAGE",
        Composant::nouveau("Compteur et son horloge")
            .entree("raz", 1)
            .sortie("q", 4)
            .sortie("clk", 1)
            .horloge("h", 40)
            .inst("c", "COMPTEUR4")
            .fil(&["h.s", "c.clk", "clk"])
            .fil(&["raz", "c.raz"])
            .fil(&["c.q", "q"]),
    );
    let mut banc = Banc::nouveau(&bib, "MONTAGE").unwrap();
    banc.ecrire("raz", 1).unwrap();
    banc.avancer(100); // un front montant à t=40 : remise à zéro
    assert_eq!(banc.lire("q").unwrap(), Some(0));
    banc.ecrire("raz", 0).unwrap();
    // Fronts montants à t = 120, 200, 280, … : le k-ième est à 40 + 80 k.
    for k in 1..=50u64 {
        banc.avancer(80);
        assert_eq!(banc.sim.temps(), 100 + 80 * k);
        assert_eq!(banc.lire("q").unwrap(), Some(k % 16), "après {k} fronts");
    }
    // Un million d'unités de temps plus tard : 12 500 fronts de plus.
    banc.avancer(1_000_000);
    assert_eq!(banc.lire("q").unwrap(), Some((50 + 12_500) % 16));
}

#[test]
fn oscillateur() {
    let mut banc = Banc::nouveau(&bibliotheque(), "OSCILLATEUR").unwrap();
    banc.ecrire("marche", 0).unwrap();
    assert!(banc.stabiliser(100).est_stable());
    assert_eq!(banc.lire("s").unwrap(), Some(1));
    banc.ecrire("marche", 1).unwrap();
    assert!(
        !banc.stabiliser(10_000).est_stable(),
        "il oscille : jamais de repos"
    );
    banc.ecrire("marche", 0).unwrap();
    assert!(banc.stabiliser(100).est_stable());
}

#[test]
fn equivalence_de_deux_ou_exclusifs() {
    let mut bib = bibliotheque();
    bib.ajouter(
        "OU_X_BIS",
        Composant::nouveau("(a ou b) et non (a et b)")
            .entree("a", 1)
            .entree("b", 1)
            .sortie("s", 1)
            .inst("ou", "OU")
            .inst("nand", "NAND")
            .inst("et", "ET")
            .fil(&["a", "ou.a", "nand.a"])
            .fil(&["b", "ou.b", "nand.b"])
            .fil(&["ou.s", "et.a"])
            .fil(&["nand.s", "et.b"])
            .fil(&["et.s", "s"]),
    );
    assert_eq!(
        equivalence(&bib, "OU_X", &bib, "OU_X_BIS").unwrap(),
        Equivalence::Prouvee { combinaisons: 4 }
    );
    assert_eq!(
        equivalence(&bib, "OU_X", &bib, "OU").unwrap(),
        Equivalence::Differents {
            entrees: 3,
            sorties_a: vec![Valeur::Zero],
            sorties_b: vec![Valeur::Un],
        }
    );
    assert_eq!(
        equivalence(&bib, "MUX", &bib, "SELECTEUR_BUS").unwrap(),
        Equivalence::Prouvee { combinaisons: 8 }
    );
    assert!(
        equivalence(&bib, "OU_X", &bib, "MUX").is_err(),
        "broches différentes"
    );
}

/// Un composant qui relie directement une entrée à une sortie fusionne deux nets
/// de son parent.
#[test]
fn fil_traversant_un_composant() {
    let mut bib = bibliotheque();
    bib.ajouter(
        "TRAVERSEE",
        Composant::nouveau("Un simple fil")
            .entree("a", 1)
            .sortie("s", 1)
            .fil(&["a", "s"]),
    );
    bib.ajouter(
        "DOUBLE",
        Composant::nouveau("non x, à travers deux fils")
            .entree("x", 1)
            .sortie("y", 1)
            .inst("t1", "TRAVERSEE")
            .inst("n", "NON")
            .inst("t2", "TRAVERSEE")
            .fil(&["x", "t1.a"])
            .fil(&["t1.s", "n.a"])
            .fil(&["n.s", "t2.a"])
            .fil(&["t2.s", "y"]),
    );
    let table = table_de_verite(&bib, "DOUBLE").unwrap();
    assert_eq!(table.lignes, vec![vec![Valeur::Un], vec![Valeur::Zero]]);
    let c = Circuit::elaborer(&bib, "DOUBLE").unwrap();
    assert_eq!(c.netlist.nb_nets, 2);
    assert_eq!(c.net("x").unwrap(), c.net("n.a").unwrap());
    assert_eq!(c.net("t1.s").unwrap(), c.net("x").unwrap());
    assert_eq!(c.net("y").unwrap(), c.net("n.n.s").unwrap());
    // Mis à plat seul, le fil relie directement son entrée à sa sortie.
    let table = table_de_verite(&bib, "TRAVERSEE").unwrap();
    assert_eq!(table.lignes, vec![vec![Valeur::Zero], vec![Valeur::Un]]);
    assert_eq!(table.chemin_critique, 0);
}

#[test]
fn entree_laissee_en_l_air() {
    let mut bib = Bibliotheque::nouvelle();
    bib.ajouter(
        "OUBLI",
        Composant::nouveau("b n'est pas reliée")
            .entree("a", 1)
            .sortie("s", 1)
            .inst("n", "NAND")
            .fil(&["a", "n.a"])
            .fil(&["n.s", "s"]),
    );
    let mut banc = Banc::nouveau(&bib, "OUBLI").unwrap();
    assert_eq!(
        banc.circuit.avertissements,
        vec!["dans « OUBLI » : l'entrée n.b n'est reliée à rien"]
    );
    banc.ecrire("a", 1).unwrap();
    banc.stabiliser(10);
    assert_eq!(
        banc.lire("s").unwrap(),
        None,
        "une entrée flottante est lue comme inconnue"
    );
    banc.ecrire("a", 0).unwrap();
    banc.stabiliser(10);
    assert_eq!(
        banc.lire("s").unwrap(),
        Some(1),
        "mais un 0 force la sortie"
    );
}
