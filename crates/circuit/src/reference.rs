//! Bibliothèque de référence, construite uniquement à partir de portes NAND
//! (et de tampons trois états pour le sélecteur de bus).
//!
//! Elle sert de jeu d'essai au moteur : chaque composant est vérifié contre sa
//! définition mathématique dans `tests/reference.rs`. Elle servira aussi de
//! corrigé aux quêtes.
//!
//! | Composant | Broches | Portes |
//! |---|---|---|
//! | `NON` | `a` → `s` | 1 |
//! | `ET`, `OU` | `a`, `b` → `s` | 2, 3 |
//! | `NON_OU` | `a`, `b` → `s` | 4 |
//! | `OU_X` | `a`, `b` → `s` | 4 |
//! | `MUX` | `a`, `b`, `sel` → `s` (`sel = 0` choisit `a`) | 4 |
//! | `DEMI_ADD` | `a`, `b` → `s`, `r` | 5 |
//! | `ADD_COMPLET` | `a`, `b`, `re` → `s`, `rs` | 9 |
//! | `ADD8` | `a[8]`, `b[8]`, `re` → `s[8]`, `rs` | 72 |
//! | `VERROU_SR` | `s_n`, `r_n` → `q`, `q_n` | 2 |
//! | `VERROU_D` | `d`, `e` → `q`, `q_n` (transparent quand `e = 1`) | 4 |
//! | `BASCULE_D` | `d`, `clk` → `q`, `q_n` (front montant) | 9 |
//! | `REGISTRE8` | `d[8]`, `charge`, `clk` → `q[8]` | 104 |
//! | `COMPTEUR4` | `raz`, `clk` → `q[4]` (remise à zéro synchrone) | 65 |
//! | `OSCILLATEUR` | `marche` → `s` | 3 |
//! | `SELECTEUR_BUS` | `a`, `b`, `sel` → `s` (deux tampons trois états sur un même fil) | 1 + 2 |

use crate::modele::{Bibliotheque, Composant};

/// Construit la bibliothèque de référence.
pub fn bibliotheque() -> Bibliotheque {
    let mut bib = Bibliotheque::nouvelle();

    bib.ajouter(
        "NON",
        Composant::nouveau("Inverseur : s = non a")
            .entree("a", 1)
            .sortie("s", 1)
            .inst("n", "NAND")
            .fil(&["a", "n.a", "n.b"])
            .fil(&["n.s", "s"]),
    );

    bib.ajouter(
        "ET",
        Composant::nouveau("s = a et b")
            .entree("a", 1)
            .entree("b", 1)
            .sortie("s", 1)
            .inst("n", "NAND")
            .inst("inv", "NON")
            .fil(&["a", "n.a"])
            .fil(&["b", "n.b"])
            .fil(&["n.s", "inv.a"])
            .fil(&["inv.s", "s"]),
    );

    bib.ajouter(
        "OU",
        Composant::nouveau("s = a ou b")
            .entree("a", 1)
            .entree("b", 1)
            .sortie("s", 1)
            .inst("na", "NON")
            .inst("nb", "NON")
            .inst("n", "NAND")
            .fil(&["a", "na.a"])
            .fil(&["b", "nb.a"])
            .fil(&["na.s", "n.a"])
            .fil(&["nb.s", "n.b"])
            .fil(&["n.s", "s"]),
    );

    bib.ajouter(
        "NON_OU",
        Composant::nouveau("s = non (a ou b)")
            .entree("a", 1)
            .entree("b", 1)
            .sortie("s", 1)
            .inst("ou", "OU")
            .inst("inv", "NON")
            .fil(&["a", "ou.a"])
            .fil(&["b", "ou.b"])
            .fil(&["ou.s", "inv.a"])
            .fil(&["inv.s", "s"]),
    );

    bib.ajouter(
        "OU_X",
        Composant::nouveau("Ou exclusif en quatre NAND : s = a xor b")
            .entree("a", 1)
            .entree("b", 1)
            .sortie("s", 1)
            .inst("n1", "NAND")
            .inst("n2", "NAND")
            .inst("n3", "NAND")
            .inst("n4", "NAND")
            .fil(&["a", "n1.a", "n2.a"])
            .fil(&["b", "n1.b", "n3.b"])
            .fil(&["n1.s", "n2.b", "n3.a"])
            .fil(&["n2.s", "n4.a"])
            .fil(&["n3.s", "n4.b"])
            .fil(&["n4.s", "s"]),
    );

    bib.ajouter(
        "MUX",
        Composant::nouveau("Multiplexeur : s = a si sel = 0, b si sel = 1")
            .entree("a", 1)
            .entree("b", 1)
            .entree("sel", 1)
            .sortie("s", 1)
            .inst("nsel", "NON")
            .inst("pa", "NAND")
            .inst("pb", "NAND")
            .inst("n", "NAND")
            .fil(&["sel", "nsel.a", "pb.b"])
            .fil(&["a", "pa.a"])
            .fil(&["nsel.s", "pa.b"])
            .fil(&["b", "pb.a"])
            .fil(&["pa.s", "n.a"])
            .fil(&["pb.s", "n.b"])
            .fil(&["n.s", "s"]),
    );

    bib.ajouter(
        "DEMI_ADD",
        Composant::nouveau("Demi-additionneur : a + b = 2 r + s")
            .entree("a", 1)
            .entree("b", 1)
            .sortie("s", 1)
            .sortie("r", 1)
            .inst("n1", "NAND")
            .inst("n2", "NAND")
            .inst("n3", "NAND")
            .inst("n4", "NAND")
            .inst("inv", "NON")
            .fil(&["a", "n1.a", "n2.a"])
            .fil(&["b", "n1.b", "n3.b"])
            .fil(&["n1.s", "n2.b", "n3.a", "inv.a"])
            .fil(&["n2.s", "n4.a"])
            .fil(&["n3.s", "n4.b"])
            .fil(&["n4.s", "s"])
            .fil(&["inv.s", "r"]),
    );

    bib.ajouter(
        "ADD_COMPLET",
        Composant::nouveau("Additionneur complet en neuf NAND : a + b + re = 2 rs + s")
            .entree("a", 1)
            .entree("b", 1)
            .entree("re", 1)
            .sortie("s", 1)
            .sortie("rs", 1)
            .inst("n1", "NAND")
            .inst("n2", "NAND")
            .inst("n3", "NAND")
            .inst("n4", "NAND")
            .inst("n5", "NAND")
            .inst("n6", "NAND")
            .inst("n7", "NAND")
            .inst("n8", "NAND")
            .inst("n9", "NAND")
            .fil(&["a", "n1.a", "n2.a"])
            .fil(&["b", "n1.b", "n3.b"])
            .fil(&["n1.s", "n2.b", "n3.a", "n9.a"])
            .fil(&["n2.s", "n4.a"])
            .fil(&["n3.s", "n4.b"])
            // n4.s = a xor b
            .fil(&["n4.s", "n5.a", "n6.a"])
            .fil(&["re", "n5.b", "n7.b"])
            .fil(&["n5.s", "n6.b", "n7.a", "n9.b"])
            .fil(&["n6.s", "n8.a"])
            .fil(&["n7.s", "n8.b"])
            .fil(&["n8.s", "s"])
            .fil(&["n9.s", "rs"]),
    );

    let mut add8 = Composant::nouveau("Additionneur 8 bits à propagation de retenue")
        .entree("a", 8)
        .entree("b", 8)
        .entree("re", 1)
        .sortie("s", 8)
        .sortie("rs", 1);
    for i in 0..8 {
        add8 = add8
            .inst(&format!("add{i}"), "ADD_COMPLET")
            .fil(&[format!("a[{i}]"), format!("add{i}.a")])
            .fil(&[format!("b[{i}]"), format!("add{i}.b")])
            .fil(&[format!("add{i}.s"), format!("s[{i}]")]);
        add8 = if i == 0 {
            add8.fil(&["re", "add0.re"])
        } else {
            add8.fil(&[format!("add{}.rs", i - 1), format!("add{i}.re")])
        };
    }
    bib.ajouter("ADD8", add8.fil(&["add7.rs", "rs"]));

    bib.ajouter(
        "VERROU_SR",
        Composant::nouveau("Verrou SR en NAND croisées, commandes actives à 0")
            .entree("s_n", 1)
            .entree("r_n", 1)
            .sortie("q", 1)
            .sortie("q_n", 1)
            .inst("n1", "NAND")
            .inst("n2", "NAND")
            .fil(&["s_n", "n1.a"])
            .fil(&["r_n", "n2.a"])
            .fil(&["n1.s", "n2.b", "q"])
            .fil(&["n2.s", "n1.b", "q_n"]),
    );

    bib.ajouter(
        "VERROU_D",
        Composant::nouveau("Verrou D : q suit d quand e = 1, mémorise quand e = 0")
            .entree("d", 1)
            .entree("e", 1)
            .sortie("q", 1)
            .sortie("q_n", 1)
            .inst("n1", "NAND")
            .inst("n2", "NAND")
            .inst("sr", "VERROU_SR")
            .fil(&["d", "n1.a"])
            .fil(&["e", "n1.b", "n2.b"])
            .fil(&["n1.s", "n2.a", "sr.s_n"])
            .fil(&["n2.s", "sr.r_n"])
            .fil(&["sr.q", "q"])
            .fil(&["sr.q_n", "q_n"]),
    );

    bib.ajouter(
        "BASCULE_D",
        Composant::nouveau(
            "Bascule D maître-esclave : q prend la valeur de d au front montant de clk",
        )
        .entree("d", 1)
        .entree("clk", 1)
        .sortie("q", 1)
        .sortie("q_n", 1)
        .inst("nclk", "NON")
        .inst("maitre", "VERROU_D")
        .inst("esclave", "VERROU_D")
        .fil(&["clk", "nclk.a", "esclave.e"])
        .fil(&["nclk.s", "maitre.e"])
        .fil(&["d", "maitre.d"])
        .fil(&["maitre.q", "esclave.d"])
        .fil(&["esclave.q", "q"])
        .fil(&["esclave.q_n", "q_n"]),
    );

    let mut registre = Composant::nouveau(
        "Registre 8 bits : au front montant de clk, q prend d si charge = 1 et se conserve sinon",
    )
    .entree("d", 8)
    .entree("charge", 1)
    .entree("clk", 1)
    .sortie("q", 8);
    let mut horloges = vec!["clk".to_string()];
    let mut charges = vec!["charge".to_string()];
    for i in 0..8 {
        registre = registre
            .inst(&format!("mux{i}"), "MUX")
            .inst(&format!("b{i}"), "BASCULE_D")
            .fil(&[format!("d[{i}]"), format!("mux{i}.b")])
            .fil(&[format!("mux{i}.s"), format!("b{i}.d")])
            .fil(&[format!("b{i}.q"), format!("mux{i}.a"), format!("q[{i}]")]);
        horloges.push(format!("b{i}.clk"));
        charges.push(format!("mux{i}.sel"));
    }
    bib.ajouter("REGISTRE8", registre.fil(&horloges).fil(&charges));

    let mut compteur = Composant::nouveau(
        "Compteur 4 bits : à chaque front montant de clk, q devient 0 si raz = 1, q + 1 sinon",
    )
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("q", 4)
    .inst("un", "UN")
    .inst("nraz", "NON");
    let mut horloges = vec!["clk".to_string()];
    let mut autorisations = vec!["nraz.s".to_string()];
    for i in 0..4 {
        compteur = compteur
            .inst(&format!("inc{i}"), "DEMI_ADD")
            .inst(&format!("et{i}"), "ET")
            .inst(&format!("b{i}"), "BASCULE_D")
            .fil(&[format!("b{i}.q"), format!("inc{i}.a"), format!("q[{i}]")])
            .fil(&[format!("inc{i}.s"), format!("et{i}.a")])
            .fil(&[format!("et{i}.s"), format!("b{i}.d")]);
        compteur = if i == 0 {
            compteur.fil(&["un.s", "inc0.b"])
        } else {
            compteur.fil(&[format!("inc{}.r", i - 1), format!("inc{i}.b")])
        };
        horloges.push(format!("b{i}.clk"));
        autorisations.push(format!("et{i}.b"));
    }
    bib.ajouter(
        "COMPTEUR4",
        compteur
            .fil(&["raz", "nraz.a"])
            .fil(&horloges)
            .fil(&autorisations),
    );

    bib.ajouter(
        "OSCILLATEUR",
        Composant::nouveau("Anneau de trois inverseurs, arrêté quand marche = 0")
            .entree("marche", 1)
            .sortie("s", 1)
            .inst("n", "NAND")
            .inst("i1", "NON")
            .inst("i2", "NON")
            .fil(&["marche", "n.a"])
            .fil(&["n.s", "i1.a", "s"])
            .fil(&["i1.s", "i2.a"])
            .fil(&["i2.s", "n.b"]),
    );

    bib.ajouter(
        "SELECTEUR_BUS",
        Composant::nouveau(
            "Deux tampons trois états sur un même fil : s = a si sel = 0, b si sel = 1",
        )
        .entree("a", 1)
        .entree("b", 1)
        .entree("sel", 1)
        .sortie("s", 1)
        .inst("nsel", "NON")
        .inst("ta", "TRI")
        .inst("tb", "TRI")
        .fil(&["sel", "nsel.a", "tb.e"])
        .fil(&["nsel.s", "ta.e"])
        .fil(&["a", "ta.a"])
        .fil(&["b", "tb.a"])
        .fil(&["ta.s", "tb.s", "s"]),
    );

    bib
}
