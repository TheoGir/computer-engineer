//! Le matériel : la bibliothèque de composants de l'ordinateur miniature.
//!
//! Tout est fait de portes NAND, à l'exception de l'horloge (la primitive `HORLOGE`, qui joue
//! le rôle du quartz). Les composants s'emboîtent :
//!
//! ```text
//! ORDINATEUR
//! ├── horloge                      HORLOGE
//! ├── cpu                          PROCESSEUR
//! │   ├── dec                      DECODEUR            code de l'instruction → commandes
//! │   ├── source, choix            MUX8
//! │   ├── ual                      UAL                 ADD8, OU_X, MUX, NAND
//! │   ├── acc                      REGISTRE8_RAZ       8 × CELLULE (MUX, ET, BASCULE_D)
//! │   ├── zero                     NUL8
//! │   ├── rz, rc                   CELLULE
//! │   └── cp                       COMPTEUR_PROGRAMME  INCR8, MUX8, REGISTRE8_RAZ
//! ├── rom                          MEMOIRE_MORTE       2 × DECODEUR4, une NAND par mot, TOUS_n
//! └── ram                          RAM16 = 2 × RAM8 = 4 × RAM4 = … = 16 × REGISTRE8
//! ```
//!
//! La mémoire morte est fabriquée à partir du programme : une ligne par instruction non nulle,
//! une colonne par bit.

use std::collections::BTreeMap;

use crate::erreur::Erreur;
use crate::modele::{Bibliotheque, Composant};
use crate::reference;

use super::TAILLE_PROGRAMME;

/// Les réglages de fabrication de la machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reglages {
    /// La mémoire vive contient `2^bits_ram` octets (de 1 à 8).
    pub bits_ram: u32,
    /// Durée de chaque niveau de l'horloge, en unités de temps du moteur (une porte = 1).
    pub demi_periode: u64,
}

impl Default for Reglages {
    fn default() -> Self {
        Reglages {
            bits_ram: 4,
            demi_periode: DEMI_PERIODE,
        }
    }
}

/// Demi-période par défaut de l'horloge. Le test `l_horloge_laisse_le_temps_au_plus_long_chemin`
/// mesure le temps d'établissement réel et vérifie la marge.
pub const DEMI_PERIODE: u64 = 64;

/// Des fils en cours de construction : chaque source avec toutes ses arrivées.
#[derive(Default)]
struct Fils(BTreeMap<String, Vec<String>>);

impl Fils {
    fn relier(&mut self, source: impl Into<String>, arrivee: impl Into<String>) {
        self.0
            .entry(source.into())
            .or_default()
            .push(arrivee.into());
    }

    fn poser(self, mut c: Composant) -> Composant {
        for (source, mut arrivees) in self.0 {
            arrivees.insert(0, source);
            c = c.fil(&arrivees);
        }
        c
    }
}

/// `TOUS_n` : `s` vaut 1 quand les `n` bits de `e` valent 1. Ajoute aussi les tailles plus
/// petites dont il a besoin.
fn tous(bib: &mut Bibliotheque, n: u32) -> String {
    assert!(n >= 2);
    let nom = format!("TOUS_{n}");
    if bib.composants.contains_key(&nom) {
        return nom;
    }
    let (gauche, droite) = (n.div_ceil(2), n / 2);
    let mut c = Composant::nouveau(&format!("s = 1 quand les {n} bits de e valent 1"))
        .entree("e", n)
        .sortie("s", 1)
        .inst("et", "ET")
        .fil(&["et.s", "s"]);
    for (cote, broche, debut, taille) in [("g", "a", 0, gauche), ("d", "b", gauche, droite)] {
        if taille == 1 {
            c = c.fil(&[format!("e[{debut}]"), format!("et.{broche}")]);
        } else {
            let sous = tous(bib, taille);
            c = c
                .inst(cote, &sous)
                .fil(&[
                    format!("e[{debut}:{}]", debut + taille - 1),
                    format!("{cote}.e"),
                ])
                .fil(&[format!("{cote}.s"), format!("et.{broche}")]);
        }
    }
    bib.ajouter(&nom, c);
    nom
}

fn mux8() -> Composant {
    let mut c = Composant::nouveau("Multiplexeur 8 bits : s = a si sel = 0, b si sel = 1")
        .entree("a", 8)
        .entree("b", 8)
        .entree("sel", 1)
        .sortie("s", 8);
    let mut sels = vec!["sel".to_string()];
    for i in 0..8 {
        c = c
            .inst(&format!("m{i}"), "MUX")
            .fil(&[format!("a[{i}]"), format!("m{i}.a")])
            .fil(&[format!("b[{i}]"), format!("m{i}.b")])
            .fil(&[format!("m{i}.s"), format!("s[{i}]")]);
        sels.push(format!("m{i}.sel"));
    }
    c.fil(&sels)
}

fn nul8() -> Composant {
    Composant::nouveau("z = 1 quand les 8 bits de e valent 0")
        .entree("e", 8)
        .sortie("z", 1)
        .inst("o0", "OU")
        .inst("o1", "OU")
        .inst("o2", "OU")
        .inst("o3", "OU")
        .inst("p0", "OU")
        .inst("p1", "OU")
        .inst("fin", "NON_OU")
        .fil(&["e[0]", "o0.a"])
        .fil(&["e[1]", "o0.b"])
        .fil(&["e[2]", "o1.a"])
        .fil(&["e[3]", "o1.b"])
        .fil(&["e[4]", "o2.a"])
        .fil(&["e[5]", "o2.b"])
        .fil(&["e[6]", "o3.a"])
        .fil(&["e[7]", "o3.b"])
        .fil(&["o0.s", "p0.a"])
        .fil(&["o1.s", "p0.b"])
        .fil(&["o2.s", "p1.a"])
        .fil(&["o3.s", "p1.b"])
        .fil(&["p0.s", "fin.a"])
        .fil(&["p1.s", "fin.b"])
        .fil(&["fin.s", "z"])
}

fn cellule() -> Composant {
    Composant::nouveau(
        "Un bit de registre : au front montant de clk, q devient 0 si nraz = 0, d si charge = 1, et se conserve sinon",
    )
    .entree("d", 1)
    .entree("charge", 1)
    .entree("nraz", 1)
    .entree("clk", 1)
    .sortie("q", 1)
    .inst("mux", "MUX")
    .inst("et", "ET")
    .inst("b", "BASCULE_D")
    .fil(&["d", "mux.b"])
    .fil(&["charge", "mux.sel"])
    .fil(&["mux.s", "et.a"])
    .fil(&["nraz", "et.b"])
    .fil(&["et.s", "b.d"])
    .fil(&["clk", "b.clk"])
    .fil(&["b.q", "mux.a", "q"])
}

fn registre8_raz() -> Composant {
    let mut c = Composant::nouveau("Registre 8 bits avec chargement et remise à zéro (nraz = 0)")
        .entree("d", 8)
        .entree("charge", 1)
        .entree("nraz", 1)
        .entree("clk", 1)
        .sortie("q", 8);
    let mut f = Fils::default();
    for i in 0..8 {
        c = c
            .inst(&format!("c{i}"), "CELLULE")
            .fil(&[format!("d[{i}]"), format!("c{i}.d")])
            .fil(&[format!("c{i}.q"), format!("q[{i}]")]);
        for broche in ["charge", "nraz", "clk"] {
            f.relier(broche, format!("c{i}.{broche}"));
        }
    }
    f.poser(c)
}

fn ual() -> Composant {
    let mut c = Composant::nouveau(
        "Unité de calcul : r = b (op 0), a + b (op 1), a − b (op 2), non (a et b) (op 3) ; rs = retenue",
    )
    .entree("a", 8)
    .entree("b", 8)
    .entree("op", 2)
    .sortie("r", 8)
    .sortie("rs", 1)
    .inst("nop0", "NON")
    .inst("soustrait", "ET")
    .inst("calcule", "OU_X")
    .inst("add", "ADD8");
    let mut f = Fils::default();
    f.relier("op[0]", "nop0.a");
    f.relier("op[0]", "calcule.a");
    f.relier("op[1]", "soustrait.a");
    f.relier("op[1]", "calcule.b");
    f.relier("nop0.s", "soustrait.b");
    f.relier("soustrait.s", "add.re");
    f.relier("add.rs", "rs");
    f.relier("a", "add.a");
    for i in 0..8 {
        c = c
            .inst(&format!("bx{i}"), "OU_X")
            .inst(&format!("nand{i}"), "NAND")
            .inst(&format!("y{i}"), "MUX")
            .inst(&format!("r{i}"), "MUX");
        // b inversé pour la soustraction : a − b = a + non b + 1.
        f.relier(format!("b[{i}]"), format!("bx{i}.a"));
        f.relier("soustrait.s", format!("bx{i}.b"));
        f.relier(format!("bx{i}.s"), format!("add.b[{i}]"));
        f.relier(format!("a[{i}]"), format!("nand{i}.a"));
        f.relier(format!("b[{i}]"), format!("nand{i}.b"));
        // op 0 : b ; op 3 : non-et ; op 1 et 2 : la somme.
        f.relier(format!("b[{i}]"), format!("y{i}.a"));
        f.relier(format!("nand{i}.s"), format!("y{i}.b"));
        f.relier("op[0]", format!("y{i}.sel"));
        f.relier(format!("y{i}.s"), format!("r{i}.a"));
        f.relier(format!("add.s[{i}]"), format!("r{i}.b"));
        f.relier("calcule.s", format!("r{i}.sel"));
        f.relier(format!("r{i}.s"), format!("r[{i}]"));
    }
    f.poser(c)
}

fn incr8() -> Composant {
    let mut c = Composant::nouveau("s = e + 1 sur 8 bits")
        .entree("e", 8)
        .sortie("s", 8)
        .inst("inv", "NON");
    let mut f = Fils::default();
    f.relier("e[0]", "inv.a");
    f.relier("inv.s", "s[0]");
    for i in 1..8 {
        c = c.inst(&format!("h{i}"), "DEMI_ADD");
        f.relier(format!("e[{i}]"), format!("h{i}.a"));
        if i == 1 {
            f.relier("e[0]", "h1.b");
        } else {
            f.relier(format!("h{}.r", i - 1), format!("h{i}.b"));
        }
        f.relier(format!("h{i}.s"), format!("s[{i}]"));
    }
    f.poser(c)
}

fn compteur_programme() -> Composant {
    Composant::nouveau(
        "Compteur de programme : au front montant, pc devient 0 si nraz = 0, se conserve si gel = 1, prend cible si saut = 1, pc + 1 sinon",
    )
    .entree("cible", 8)
    .entree("saut", 1)
    .entree("gel", 1)
    .entree("nraz", 1)
    .entree("clk", 1)
    .sortie("pc", 8)
    .inst("incr", "INCR8")
    .inst("choix", "MUX8")
    .inst("ngel", "NON")
    .inst("reg", "REGISTRE8_RAZ")
    .fil(&["reg.q", "incr.e", "pc"])
    .fil(&["incr.s", "choix.a"])
    .fil(&["cible", "choix.b"])
    .fil(&["saut", "choix.sel"])
    .fil(&["choix.s", "reg.d"])
    .fil(&["gel", "ngel.a"])
    .fil(&["ngel.s", "reg.charge"])
    .fil(&["nraz", "reg.nraz"])
    .fil(&["clk", "reg.clk"])
}

fn decodeur() -> Composant {
    let c = Composant::nouveau(
        "Décodeur d'instruction : du code o et des drapeaux z et c vers les commandes du processeur",
    )
    .entree("o", 4)
    .entree("z", 1)
    .entree("c", 1)
    .sortie("imm", 1)
    .sortie("op", 2)
    .sortie("ecrit_a", 1)
    .sortie("entree", 1)
    .sortie("ecrit_ram", 1)
    .sortie("sortie", 1)
    .sortie("saut", 1)
    .sortie("arret", 1)
    .sortie("charge_c", 1)
    .inst("n0", "NON")
    .inst("n1", "NON")
    .inst("n2", "NON")
    .inst("n3", "NON")
    // Les quatre familles : 00xx divers, 01xx calcul avec la mémoire, 10xx calcul immédiat, 11xx sauts.
    .inst("calcul", "OU_X")
    .inst("divers", "ET")
    .inst("sauts", "ET")
    // Les deux bits de poids faible.
    .inst("l00", "ET")
    .inst("l01", "ET")
    .inst("l10", "ET")
    .inst("l11", "ET")
    .inst("sta", "ET")
    .inst("ent", "ET")
    .inst("sor", "ET")
    .inst("ecrit", "OU")
    .inst("arith", "OU_X")
    .inst("retenue", "ET")
    .inst("si_z", "ET")
    .inst("si_c", "ET")
    .inst("cond", "OU")
    .inst("pris", "OU")
    .inst("saute", "ET")
    .inst("halte", "ET");
    let mut f = Fils::default();
    for i in 0..4 {
        f.relier(format!("o[{i}]"), format!("n{i}.a"));
    }
    f.relier("o[3]", "imm");
    f.relier("o[0:1]", "op");
    f.relier("o[3]", "calcul.a");
    f.relier("o[2]", "calcul.b");
    f.relier("n3.s", "divers.a");
    f.relier("n2.s", "divers.b");
    f.relier("o[3]", "sauts.a");
    f.relier("o[2]", "sauts.b");
    for (nom, haut, bas) in [
        ("l00", "n1.s", "n0.s"),
        ("l01", "n1.s", "o[0]"),
        ("l10", "o[1]", "n0.s"),
        ("l11", "o[1]", "o[0]"),
    ] {
        f.relier(haut, format!("{nom}.a"));
        f.relier(bas, format!("{nom}.b"));
    }
    for (nom, bas, broche) in [
        ("sta", "l01.s", "ecrit_ram"),
        ("ent", "l10.s", "entree"),
        ("sor", "l11.s", "sortie"),
    ] {
        f.relier("divers.s", format!("{nom}.a"));
        f.relier(bas, format!("{nom}.b"));
        f.relier(format!("{nom}.s"), broche);
    }
    f.relier("calcul.s", "ecrit.a");
    f.relier("ent.s", "ecrit.b");
    f.relier("ecrit.s", "ecrit_a");
    // La retenue n'est rangée que par l'addition et la soustraction.
    f.relier("o[0]", "arith.a");
    f.relier("o[1]", "arith.b");
    f.relier("calcul.s", "retenue.a");
    f.relier("arith.s", "retenue.b");
    f.relier("retenue.s", "charge_c");
    // Saut pris : JMP toujours, JZ si z, JC si c.
    f.relier("l01.s", "si_z.a");
    f.relier("z", "si_z.b");
    f.relier("l10.s", "si_c.a");
    f.relier("c", "si_c.b");
    f.relier("si_z.s", "cond.a");
    f.relier("si_c.s", "cond.b");
    f.relier("l00.s", "pris.a");
    f.relier("cond.s", "pris.b");
    f.relier("sauts.s", "saute.a");
    f.relier("pris.s", "saute.b");
    f.relier("saute.s", "saut");
    f.relier("sauts.s", "halte.a");
    f.relier("l11.s", "halte.b");
    f.relier("halte.s", "arret");
    f.poser(c)
}

fn processeur() -> Composant {
    Composant::nouveau(
        "Processeur : un accumulateur, un compteur de programme, deux drapeaux, une instruction par cycle",
    )
    .entree("instr", 12)
    .entree("lu", 8)
    .entree("clavier", 8)
    .entree("nraz", 1)
    .entree("clk", 1)
    .sortie("pc", 8)
    .sortie("a", 8)
    .sortie("ecrit_ram", 1)
    .sortie("sortie", 1)
    .sortie("arret", 1)
    .inst("dec", "DECODEUR")
    .inst("source", "MUX8")
    .inst("ual", "UAL")
    .inst("choix", "MUX8")
    .inst("acc", "REGISTRE8_RAZ")
    .inst("zero", "NUL8")
    .inst("rz", "CELLULE")
    .inst("rc", "CELLULE")
    .inst("cp", "COMPTEUR_PROGRAMME")
    .fil(&["instr[8:11]", "dec.o"])
    .fil(&["instr[0:7]", "source.b", "cp.cible"])
    .fil(&["lu", "source.a"])
    .fil(&["dec.imm", "source.sel"])
    .fil(&["source.s", "ual.b"])
    .fil(&["acc.q", "ual.a", "a"])
    .fil(&["dec.op", "ual.op"])
    .fil(&["ual.r", "choix.a"])
    .fil(&["clavier", "choix.b"])
    .fil(&["dec.entree", "choix.sel"])
    .fil(&["choix.s", "acc.d", "zero.e"])
    .fil(&["dec.ecrit_a", "acc.charge", "rz.charge"])
    .fil(&["zero.z", "rz.d"])
    .fil(&["rz.q", "dec.z"])
    .fil(&["ual.rs", "rc.d"])
    .fil(&["dec.charge_c", "rc.charge"])
    .fil(&["rc.q", "dec.c"])
    .fil(&["dec.saut", "cp.saut"])
    .fil(&["dec.arret", "cp.gel", "arret"])
    .fil(&["cp.pc", "pc"])
    .fil(&["dec.ecrit_ram", "ecrit_ram"])
    .fil(&["dec.sortie", "sortie"])
    .fil(&["nraz", "acc.nraz", "rz.nraz", "rc.nraz", "cp.nraz"])
    .fil(&["clk", "acc.clk", "rz.clk", "rc.clk", "cp.clk"])
}

/// `RAMn` : mémoire vive de `n = 2^bits` octets, faite de deux moitiés. Renvoie son nom.
/// Ajoute à `bib` la mémoire vive de `2^bits` octets, faite de deux mémoires deux fois plus
/// petites, et renvoie son nom (`RAM16`, `RAM256`…).
pub fn ram(bib: &mut Bibliotheque, bits: u32) -> String {
    assert!(bits >= 1);
    let nom = format!("RAM{}", 1u32 << bits);
    if bib.composants.contains_key(&nom) {
        return nom;
    }
    let haut = bits - 1;
    let mut c = Composant::nouveau(&format!(
        "Mémoire vive de {} octets : q = contenu de la case adresse ; au front montant de clk, cette case prend d si ecrire = 1",
        1u32 << bits
    ))
    .entree("adresse", bits)
    .entree("d", 8)
    .entree("ecrire", 1)
    .entree("clk", 1)
    .sortie("q", 8)
    .inst("nsel", "NON")
    .inst("eb", "ET")
    .inst("eh", "ET")
    .inst("mux", "MUX8")
    .fil(&[format!("adresse[{haut}]"), "nsel.a".into(), "eh.b".into(), "mux.sel".into()])
    .fil(&["ecrire", "eb.a", "eh.a"])
    .fil(&["nsel.s", "eb.b"])
    .fil(&["d", "bas.d", "haut.d"])
    .fil(&["clk", "bas.clk", "haut.clk"])
    .fil(&["bas.q", "mux.a"])
    .fil(&["haut.q", "mux.b"])
    .fil(&["mux.s", "q"]);
    if bits == 1 {
        c = c
            .inst("bas", "REGISTRE8")
            .inst("haut", "REGISTRE8")
            .fil(&["eb.s", "bas.charge"])
            .fil(&["eh.s", "haut.charge"]);
    } else {
        let moitie = ram(bib, bits - 1);
        c = c
            .inst("bas", &moitie)
            .inst("haut", &moitie)
            .fil(&["eb.s", "bas.ecrire"])
            .fil(&["eh.s", "haut.ecrire"])
            .fil(&[
                format!("adresse[0:{}]", haut - 1),
                "bas.adresse".into(),
                "haut.adresse".into(),
            ]);
    }
    bib.ajouter(&nom, c);
    nom
}

fn decodeur4(bib: &mut Bibliotheque) -> Composant {
    let tous4 = tous(bib, 4);
    let mut c = Composant::nouveau("Décodeur 4 vers 16 : s[v] = 1 quand e vaut v")
        .entree("e", 4)
        .sortie("s", 16);
    let mut f = Fils::default();
    for k in 0..4 {
        c = c.inst(&format!("inv{k}"), "NON");
        f.relier(format!("e[{k}]"), format!("inv{k}.a"));
    }
    for v in 0..16 {
        c = c.inst(&format!("t{v}"), &tous4);
        for k in 0..4 {
            let source = if v >> k & 1 == 1 {
                format!("e[{k}]")
            } else {
                format!("inv{k}.s")
            };
            f.relier(source, format!("t{v}.e[{k}]"));
        }
        f.relier(format!("t{v}.s"), format!("s[{v}]"));
    }
    f.poser(c)
}

/// La mémoire morte qui contient `programme` : `d` = le mot rangé à l'adresse `a`
/// (0 au-delà de la fin du programme).
fn memoire_morte(bib: &mut Bibliotheque, programme: &[u16]) -> Composant {
    let mut c = Composant::nouveau(&format!(
        "Mémoire morte : {} instructions de 12 bits, une porte NAND par instruction non nulle",
        programme.len()
    ))
    .entree("a", 8)
    .sortie("d", 12)
    .inst("bas", "DECODEUR4")
    .inst("haut", "DECODEUR4")
    .fil(&["a[0:3]", "bas.e"])
    .fil(&["a[4:7]", "haut.e"]);
    let mut f = Fils::default();
    // Une ligne par mot non nul : elle passe à 0 quand l'adresse est la sienne.
    for (i, mot) in programme.iter().enumerate() {
        if mot & 0xFFF != 0 {
            c = c.inst(&format!("ligne{i}"), "NAND");
            f.relier(format!("haut.s[{}]", i >> 4), format!("ligne{i}.a"));
            f.relier(format!("bas.s[{}]", i & 15), format!("ligne{i}.b"));
        }
    }
    // Une colonne par bit : il vaut 1 quand une des lignes qui le portent est à 0.
    for bit in 0..12 {
        let lignes: Vec<usize> = (0..programme.len())
            .filter(|i| programme[*i] >> bit & 1 == 1)
            .collect();
        match lignes.len() {
            0 => {
                // Aucun mot ne porte ce bit : un 0 fabriqué avec deux NAND.
                c = c
                    .inst(&format!("jamais{bit}"), "NAND")
                    .inst(&format!("col{bit}"), "NON");
                f.relier("bas.s[0]", format!("jamais{bit}.a"));
                f.relier("bas.s[1]", format!("jamais{bit}.b"));
                f.relier(format!("jamais{bit}.s"), format!("col{bit}.a"));
            }
            1 => {
                c = c.inst(&format!("col{bit}"), "NON");
                f.relier(format!("ligne{}.s", lignes[0]), format!("col{bit}.a"));
            }
            n => {
                let tous_n = tous(bib, n as u32);
                c = c
                    .inst(&format!("tous{bit}"), &tous_n)
                    .inst(&format!("col{bit}"), "NON");
                for (k, i) in lignes.iter().enumerate() {
                    f.relier(format!("ligne{i}.s"), format!("tous{bit}.e[{k}]"));
                }
                f.relier(format!("tous{bit}.s"), format!("col{bit}.a"));
            }
        }
        f.relier(format!("col{bit}.s"), format!("d[{bit}]"));
    }
    f.poser(c)
}

/// Fabrique la bibliothèque de l'ordinateur qui exécute `programme`. Le composant principal
/// s'appelle `ORDINATEUR`.
pub fn bibliotheque(programme: &[u16], reglages: Reglages) -> Result<Bibliotheque, Erreur> {
    let parametre = |raison: String| Erreur::Parametre {
        composant: "ORDINATEUR".to_string(),
        instance: String::new(),
        raison,
    };
    if !(1..=8).contains(&reglages.bits_ram) {
        return Err(parametre(format!(
            "la mémoire vive a de 1 à 8 bits d'adresse, pas {}",
            reglages.bits_ram
        )));
    }
    if programme.len() > TAILLE_PROGRAMME {
        return Err(parametre(format!(
            "le programme a {} instructions, la mémoire morte en contient au plus {TAILLE_PROGRAMME}",
            programme.len()
        )));
    }
    if reglages.demi_periode == 0 {
        return Err(parametre(
            "la demi-période doit valoir au moins 1".to_string(),
        ));
    }
    let mut bib = reference::bibliotheque();
    bib.ajouter("MUX8", mux8());
    bib.ajouter("NUL8", nul8());
    bib.ajouter("CELLULE", cellule());
    bib.ajouter("REGISTRE8_RAZ", registre8_raz());
    bib.ajouter("UAL", ual());
    bib.ajouter("INCR8", incr8());
    bib.ajouter("COMPTEUR_PROGRAMME", compteur_programme());
    bib.ajouter("DECODEUR", decodeur());
    bib.ajouter("PROCESSEUR", processeur());
    let d4 = decodeur4(&mut bib);
    bib.ajouter("DECODEUR4", d4);
    let rom = memoire_morte(&mut bib, programme);
    bib.ajouter("MEMOIRE_MORTE", rom);
    let nom_ram = ram(&mut bib, reglages.bits_ram);

    let r = reglages.bits_ram;
    let ordinateur = Composant::nouveau(&format!(
        "Ordinateur miniature : processeur 8 bits, {} octets de mémoire vive, {} instructions en mémoire morte, écran et clavier",
        1u32 << r,
        programme.len()
    ))
    .entree("raz", 1)
    .entree("clavier", 8)
    .sortie("ecran", 8)
    .sortie("ecran_ecrit", 1)
    .sortie("arret", 1)
    .sortie("pc", 8)
    .horloge("horloge", reglages.demi_periode)
    .inst("nraz", "NON")
    .inst("nclk", "NON")
    .inst("cpu", "PROCESSEUR")
    .inst("rom", "MEMOIRE_MORTE")
    .inst("ram", &nom_ram)
    .inst("ecrit", "ET")
    .fil(&["horloge.s", "cpu.clk", "ram.clk", "nclk.a"])
    .fil(&["raz", "nraz.a"])
    .fil(&["nraz.s", "cpu.nraz"])
    .fil(&["cpu.pc", "rom.a", "pc"])
    .fil(&["rom.d", "cpu.instr"])
    .fil(&[format!("rom.d[0:{}]", r - 1), "ram.adresse".to_string()])
    .fil(&["cpu.a", "ram.d", "ecran"])
    .fil(&["cpu.ecrit_ram", "ram.ecrire"])
    .fil(&["ram.q", "cpu.lu"])
    .fil(&["clavier", "cpu.clavier"])
    // L'écran prend le caractère pendant la seconde moitié du cycle, quand tout est établi.
    .fil(&["cpu.sortie", "ecrit.a"])
    .fil(&["nclk.s", "ecrit.b"])
    .fil(&["ecrit.s", "ecran_ecrit"])
    .fil(&["cpu.arret", "arret"]);
    bib.ajouter("ORDINATEUR", ordinateur);
    bib.valider()?;
    Ok(bib)
}

/// Comme [`bibliotheque`], avec en plus un ordinateur `ORDINATEUR_<nom>` dont la mémoire morte
/// s'appelle `MEMOIRE_MORTE_<nom>` : plusieurs ordinateurs aux programmes différents peuvent
/// ainsi vivre dans le même projet.
pub fn bibliotheque_nommee(
    programme: &[u16],
    reglages: Reglages,
    nom: &str,
) -> Result<Bibliotheque, Erreur> {
    let mut bib = bibliotheque(programme, reglages)?;
    let rom = bib.composants["MEMOIRE_MORTE"].clone();
    let mut pc = bib.composants["ORDINATEUR"].clone();
    let nom_rom = format!("MEMOIRE_MORTE_{nom}");
    for i in &mut pc.instances {
        if i.genre == "MEMOIRE_MORTE" {
            i.genre.clone_from(&nom_rom);
        }
    }
    pc.description = format!("{} (ordinateur {nom})", pc.description);
    bib.ajouter(&nom_rom, rom);
    bib.ajouter(&format!("ORDINATEUR_{nom}"), pc);
    bib.valider()?;
    Ok(bib)
}
