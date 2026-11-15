//! Les grands classiques de l'électronique numérique, tous construits en portes NAND : ce sont
//! les corrigés des quêtes de l'application.
//!
//! Chaque composant est vérifié contre sa définition mathématique dans `tests/classiques.rs` :
//! exhaustivement pour les circuits combinatoires, et pour les circuits à mémoire contre un
//! modèle écrit en Rust, sur de longues suites d'actions tirées au hasard.
//!
//! Aucun n'utilise de constante : quand il faut un 0 ou un 1, il est fabriqué à partir d'une
//! entrée `x` par `NAND(x, NON x)`, qui vaut 1 dès que `x` est connue. Un joueur peut donc
//! refaire chacun d'eux avec la seule palette de l'application.
//!
//! | Composant | Broches |
//! |---|---|
//! | `EQUIV` | `a`, `b` → `s` (1 quand a = b) |
//! | `MAJORITE` | `a`, `b`, `c` → `s` (1 quand au moins deux valent 1) |
//! | `DEMUX` | `e`, `sel` → `s0`, `s1` |
//! | `MUX4` | `d[4]`, `sel[2]` → `s` |
//! | `DECODEUR2`, `DECODEUR3` | `a[n]` → `s[2^n]` (un seul bit à 1) |
//! | `ENCODEUR4` | `d[4]` → `n[2]`, `valide` (priorité au plus grand) |
//! | `EGAL4` | `a[4]`, `b[4]` → `egal` |
//! | `COMPARATEUR4` | `a[4]`, `b[4]` → `inf`, `egal`, `sup` |
//! | `ADD4` | `a[4]`, `b[4]`, `re` → `s[4]`, `rs` |
//! | `ADD8_RAPIDE` | `a[8]`, `b[8]`, `re` → `s[8]`, `rs` (retenue sélectionnée) |
//! | `SOUSTRACTEUR4` | `a[4]`, `b[4]` → `d[4]`, `emprunt` |
//! | `NEGATIF8` | `a[8]` → `s[8]` (complément à deux) |
//! | `PARITE8` | `a[8]` → `p` |
//! | `DECALEUR8` | `a[8]`, `n[3]`, `droite` → `s[8]` |
//! | `MULT4` | `a[4]`, `b[4]` → `p[8]` |
//! | `DIVISEUR4` | `a[4]`, `b[4]` → `q[4]`, `r[4]` |
//! | `SEPT_SEGMENTS` | `n[4]` → `seg[7]` (chiffres hexadécimaux) |
//! | `BASCULE_T` | `t`, `raz`, `clk` → `q` |
//! | `BASCULE_JK` | `j`, `k`, `raz`, `clk` → `q` |
//! | `DECALAGE8` | `e`, `clk` → `q[8]` |
//! | `COMPTEUR_BCD` | `raz`, `clk` → `q[4]`, `fin` |
//! | `COMPTEUR_HB4` | `raz`, `haut`, `clk` → `q[4]` |
//! | `LFSR8` | `raz`, `clk` → `q[8]` |
//! | `MINUTERIE` | `charger`, `n[8]`, `clk` → `q[8]`, `fini` |
//! | `FEU` | `raz`, `clk` → `vert`, `orange`, `rouge` |
//! | `CHRONOMETRE` | `raz`, `clk` → `u[4]`, `d[4]` (de 00 à 59) |
//! | `BANC_REGISTRES` | `ecrire`, `ad_e[2]`, `d[8]`, `ad_a[2]`, `ad_b[2]`, `clk` → `a[8]`, `b[8]` |
//! | `PILE` | `d[8]`, `empiler`, `depiler`, `raz`, `clk` → `sommet[8]`, `vide`, `plein` |
//! | `FILE` | `d[8]`, `ecrire`, `lire`, `raz`, `clk` → `sortie[8]`, `vide`, `plein` |
//! | `EMETTEUR_SERIE` | `d[8]`, `envoyer`, `raz`, `clk` → `tx`, `occupe` |
//! | `INTERRUPTIONS` | `irq[4]`, `ack`, `raz`, `clk` → `int`, `num[2]` |
//! | `DETECTEUR_FRONT` | `e`, `clk` → `front` (e vaut 1, valait 0 au dernier front) |
//! | `CADENCE` | `raz`, `clk` → `q[3]`, `tic` (un tic tous les 8 fronts) |
//! | `CONTROLE_PARITE` | `d[8]`, `p` → `ok` (nombre pair de 1 en tout) |
//! | `RECEPTEUR_SERIE` | `rx`, `raz`, `clk` → `d[8]`, `pret`, `erreur` |
//! | `LIAISON_SERIE` | `d[8]`, `envoyer`, `raz`, `clk` → `recu[8]`, `pret`, `occupe` |
//! | `CARTE_RESEAU` | `d[8]`, `envoyer`, `rx`, `lire`, `raz`, `clk` → `tx`, `occupe`, `recu[8]`, `dispo`, `plein`, `erreur` |
//! | `COMPTEUR_CASCADE` | `en`, `raz`, `clk` → `q[4]`, `retenue` (0 à 9, avance si en = 1) |
//! | `COMPTEUR_60` | `en`, `raz`, `clk` → `q[8]` (00 à 59 en décimal codé binaire), `retenue` |
//! | `COMPTEUR_24` | `en`, `raz`, `clk` → `q[8]` (00 à 23), `retenue` |
//! | `REVEIL` | `h[8]`, `m[8]`, `regler`, `arreter`, `raz`, `clk` → `sonne` |
//! | `MONTRE` | `en`, `plus_m`, `plus_h`, `raz`, `clk` → `s[8]`, `m[8]`, `h[8]` |
//! | `BALAYAGE` | `raz`, `clk` → `x[3]`, `y[3]`, `fin_ligne`, `fin_image` |
//! | `POLICE` | `n[4]`, `ligne[3]` → `motif[3]` (chiffres hexadécimaux en 3 × 5) |
//! | `MEMOIRE_VIDEO` | `ligne_e[3]`, `d[8]`, `ecrire`, `x[3]`, `y[3]`, `clk` → `pixel`, `ligne[8]` |
//! | `SERIALISEUR` | `d[8]`, `charger`, `clk` → `pixel` |
//! | `CONTROLEUR_ECRAN` | `ligne_e[3]`, `d[8]`, `ecrire`, `raz`, `clk` → `x[3]`, `y[3]`, `pixel`, `fin_image` |
//! | `GRAY` | `b[4]` → `g[4]` |
//! | `MOTIF_101` | `e`, `raz`, `clk` → `vu` |
//! | `DIVISIBLE_3` | `e`, `raz`, `clk` → `div` (le nombre lu, poids fort d'abord, est multiple de 3) |
//! | `CELLULE_VIE` | `v`, `n[8]` → `s` (règle du jeu de la vie) |
//! | `AUTOMATE_30` | `d[8]`, `charger`, `clk` → `q[8]` (règle 30, en anneau) |
//! | `PGCD` | `a[4]`, `b[4]`, `charger`, `clk` → `x[4]`, `y[4]`, `fini` |
//! | `CESAR` | `d[4]`, `k[4]` → `c[4]` = (d + k) mod 16 |
//! | `MASQUE_XOR` | `d[8]`, `k[8]` → `c[8]` = d ⊕ k |
//! | `SOMME_CONTROLE` | `d[8]`, `ajouter`, `raz`, `clk` → `somme[8]` |
//! | `HAMMING_CODE` | `d[4]` → `h[7]` (code de Hamming (7, 4)) |
//! | `HAMMING_CORRECTEUR` | `h[7]` → `d[4]`, `erreur` (corrige une erreur d'un bit) |
//! | `ARBITRE` | `a[2]`, `b[2]` → `gagne_a`, `gagne_b`, `egalite` (pierre-feuille-ciseaux) |
//! | `MORPION` | `x[9]` → `gagne` (trois croix alignées) |
//! | `DE` | `lancer`, `raz`, `clk` → `face[3]` (de 1 à 6) |
//! | `MEILLEUR_SCORE` | `score[8]`, `valider`, `raz`, `clk` → `record[8]`, `nouveau` |
//! | `PREMIER` | `n[4]` → `p` (n est premier) |
//! | `CARRE` | `n[4]` → `c[8]` = n² |
//! | `BINAIRE_DCB` | `n[8]` → `c[4]`, `d[4]`, `u[4]` (centaines, dizaines, unités) |
//! | `FIBONACCI` | `raz`, `clk` → `f[8]` |

use crate::modele::{Bibliotheque, Composant};
use crate::ordinateur::{self, Reglages};

/// Un composant en construction, avec des noms d'instances fabriqués.
struct Plan {
    c: Composant,
    n: u32,
}

impl Plan {
    fn nouveau(description: &str) -> Self {
        Plan {
            c: Composant::nouveau(description),
            n: 0,
        }
    }

    fn changer(&mut self, f: impl FnOnce(Composant) -> Composant) {
        let c = std::mem::replace(&mut self.c, Composant::nouveau(""));
        self.c = f(c);
    }

    fn entree(mut self, nom: &str, largeur: u32) -> Self {
        self.changer(|c| c.entree(nom, largeur));
        self
    }

    fn sortie(mut self, nom: &str, largeur: u32) -> Self {
        self.changer(|c| c.sortie(nom, largeur));
        self
    }

    fn relier(&mut self, a: impl AsRef<str>, b: impl AsRef<str>) {
        let (a, b) = (a.as_ref().to_owned(), b.as_ref().to_owned());
        self.changer(|c| c.fil(&[a, b]));
    }

    /// Pose une instance de `genre`, chaque broche nommée reliée à son signal. Renvoie le nom
    /// de l'instance.
    fn poser(&mut self, genre: &str, liens: &[(&str, &str)]) -> String {
        self.n += 1;
        let nom = format!("{}{}", genre.to_lowercase(), self.n);
        let (n, g) = (nom.clone(), genre.to_owned());
        self.changer(|c| c.inst(&n, &g));
        for (broche, signal) in liens {
            self.relier(signal, format!("{nom}.{broche}"));
        }
        nom
    }

    fn porte(&mut self, genre: &str, liens: &[(&str, &str)]) -> String {
        format!("{}.s", self.poser(genre, liens))
    }

    fn non(&mut self, a: &str) -> String {
        self.porte("NON", &[("a", a)])
    }

    fn nand(&mut self, a: &str, b: &str) -> String {
        self.porte("NAND", &[("a", a), ("b", b)])
    }

    fn et(&mut self, a: &str, b: &str) -> String {
        self.porte("ET", &[("a", a), ("b", b)])
    }

    fn ou(&mut self, a: &str, b: &str) -> String {
        self.porte("OU", &[("a", a), ("b", b)])
    }

    fn oux(&mut self, a: &str, b: &str) -> String {
        self.porte("OU_X", &[("a", a), ("b", b)])
    }

    /// `a` si `sel` = 0, `b` si `sel` = 1.
    fn mux(&mut self, a: &str, b: &str, sel: &str) -> String {
        self.porte("MUX", &[("a", a), ("b", b), ("sel", sel)])
    }

    /// Un 1 fabriqué à partir du signal `x` : NAND(x, NON x).
    fn un(&mut self, x: &str) -> String {
        let n = self.non(x);
        self.nand(x, &n)
    }

    /// Un 0 fabriqué à partir du signal `x`.
    fn zero(&mut self, x: &str) -> String {
        let u = self.un(x);
        self.non(&u)
    }

    /// Réduit des termes deux à deux avec une porte à deux entrées.
    fn arbre(&mut self, genre: &str, mut termes: Vec<String>) -> String {
        assert!(!termes.is_empty());
        while termes.len() > 1 {
            let mut suivants = Vec::new();
            for paire in termes.chunks(2) {
                suivants.push(match paire {
                    [a, b] => self.porte(genre, &[("a", a), ("b", b)]),
                    [a] => a.clone(),
                    _ => unreachable!(),
                });
            }
            termes = suivants;
        }
        termes.pop().unwrap()
    }

    fn fin(self) -> Composant {
        self.c
    }
}

fn bits(nom: &str, n: u32) -> Vec<String> {
    (0..n).map(|i| format!("{nom}[{i}]")).collect()
}

fn equiv() -> Composant {
    let mut p = Plan::nouveau("s = 1 quand a et b sont égaux")
        .entree("a", 1)
        .entree("b", 1)
        .sortie("s", 1);
    let x = p.oux("a", "b");
    let s = p.non(&x);
    p.relier(s, "s");
    p.fin()
}

fn majorite() -> Composant {
    let mut p = Plan::nouveau("s = 1 quand au moins deux entrées valent 1")
        .entree("a", 1)
        .entree("b", 1)
        .entree("c", 1)
        .sortie("s", 1);
    let ab = p.nand("a", "b");
    let bc = p.nand("b", "c");
    let ca = p.nand("c", "a");
    let t = p.et(&ab, &bc);
    let s = p.nand(&t, &ca);
    p.relier(s, "s");
    p.fin()
}

fn demux() -> Composant {
    let mut p = Plan::nouveau("e part sur s0 si sel = 0, sur s1 si sel = 1 ; l'autre vaut 0")
        .entree("e", 1)
        .entree("sel", 1)
        .sortie("s0", 1)
        .sortie("s1", 1);
    let ns = p.non("sel");
    let s0 = p.et("e", &ns);
    let s1 = p.et("e", "sel");
    p.relier(s0, "s0");
    p.relier(s1, "s1");
    p.fin()
}

fn mux4() -> Composant {
    let mut p = Plan::nouveau("s = d[sel]")
        .entree("d", 4)
        .entree("sel", 2)
        .sortie("s", 1);
    let bas = p.mux("d[0]", "d[1]", "sel[0]");
    let haut = p.mux("d[2]", "d[3]", "sel[0]");
    let s = p.mux(&bas, &haut, "sel[1]");
    p.relier(s, "s");
    p.fin()
}

fn decodeur2() -> Composant {
    let mut p = Plan::nouveau("s[v] = 1 quand a vaut v, les autres bits à 0")
        .entree("a", 2)
        .sortie("s", 4);
    let n0 = p.non("a[0]");
    let n1 = p.non("a[1]");
    for (i, (x, y)) in [
        (n0.clone(), n1.clone()),
        ("a[0]".to_owned(), n1),
        (n0, "a[1]".to_owned()),
        ("a[0]".to_owned(), "a[1]".to_owned()),
    ]
    .into_iter()
    .enumerate()
    {
        let s = p.et(&x, &y);
        p.relier(s, format!("s[{i}]"));
    }
    p.fin()
}

fn decodeur3() -> Composant {
    let mut p = Plan::nouveau("s[v] = 1 quand a vaut v, les autres bits à 0")
        .entree("a", 3)
        .sortie("s", 8);
    let d = p.poser("DECODEUR2", &[("a", "a[0:1]")]);
    let n2 = p.non("a[2]");
    for i in 0..4 {
        let bas = p.et(&format!("{d}.s[{i}]"), &n2);
        let haut = p.et(&format!("{d}.s[{i}]"), "a[2]");
        p.relier(bas, format!("s[{i}]"));
        p.relier(haut, format!("s[{}]", i + 4));
    }
    p.fin()
}

fn encodeur4() -> Composant {
    let mut p = Plan::nouveau(
        "n = rang du plus grand bit de d à 1 ; valide = 1 si au moins un bit de d vaut 1",
    )
    .entree("d", 4)
    .sortie("n", 2)
    .sortie("valide", 1);
    let n1 = p.ou("d[3]", "d[2]");
    let nd2 = p.non("d[2]");
    let e1 = p.et(&nd2, "d[1]");
    let n0 = p.ou("d[3]", &e1);
    let v = p.ou("d[0]", "d[1]");
    let valide = p.ou(&v, &n1);
    p.relier(&n1, "n[1]");
    p.relier(n0, "n[0]");
    p.relier(valide, "valide");
    p.fin()
}

fn egal4() -> Composant {
    let mut p = Plan::nouveau("egal = 1 quand a = b")
        .entree("a", 4)
        .entree("b", 4)
        .sortie("egal", 1);
    let termes = (0..4)
        .map(|i| {
            p.porte(
                "EQUIV",
                &[("a", &format!("a[{i}]")), ("b", &format!("b[{i}]"))],
            )
        })
        .collect();
    let e = p.arbre("ET", termes);
    p.relier(e, "egal");
    p.fin()
}

fn add4() -> Composant {
    let mut p = Plan::nouveau("s + 16·rs = a + b + re")
        .entree("a", 4)
        .entree("b", 4)
        .entree("re", 1)
        .sortie("s", 4)
        .sortie("rs", 1);
    let mut retenue = "re".to_owned();
    for i in 0..4 {
        let add = p.poser(
            "ADD_COMPLET",
            &[
                ("a", &format!("a[{i}]")),
                ("b", &format!("b[{i}]")),
                ("re", &retenue),
            ],
        );
        p.relier(format!("{add}.s"), format!("s[{i}]"));
        retenue = format!("{add}.rs");
    }
    p.relier(retenue, "rs");
    p.fin()
}

/// L'additionneur à retenue sélectionnée : les quatre bits du haut sont calculés deux fois
/// en même temps que ceux du bas, une fois pour chaque retenue possible ; la retenue du bas
/// choisit le bon résultat au lieu d'avoir à traverser les quatre étages du haut.
fn add8_rapide() -> Composant {
    let mut p = Plan::nouveau("s + 256·rs = a + b + re, moitié haute calculée d'avance")
        .entree("a", 8)
        .entree("b", 8)
        .entree("re", 1)
        .sortie("s", 8)
        .sortie("rs", 1);
    let zero = p.zero("re");
    let un = p.non(&zero);
    let bas = p.poser("ADD4", &[("a", "a[0:3]"), ("b", "b[0:3]"), ("re", "re")]);
    p.relier(format!("{bas}.s"), "s[0:3]");
    let h0 = p.poser("ADD4", &[("a", "a[4:7]"), ("b", "b[4:7]"), ("re", &zero)]);
    let h1 = p.poser("ADD4", &[("a", "a[4:7]"), ("b", "b[4:7]"), ("re", &un)]);
    let choix = format!("{bas}.rs");
    for i in 0..4 {
        let s = p.mux(&format!("{h0}.s[{i}]"), &format!("{h1}.s[{i}]"), &choix);
        p.relier(s, format!("s[{}]", i + 4));
    }
    let rs = p.mux(&format!("{h0}.rs"), &format!("{h1}.rs"), &choix);
    p.relier(rs, "rs");
    p.fin()
}

fn comparateur4() -> Composant {
    let mut p = Plan::nouveau("inf, egal, sup : a < b, a = b, a > b (nombres sans signe)")
        .entree("a", 4)
        .entree("b", 4)
        .sortie("inf", 1)
        .sortie("egal", 1)
        .sortie("sup", 1);
    // a + non b + 1 = a − b + 16 : la retenue sortante vaut 1 quand a ≥ b.
    let un = p.un("a[0]");
    let nb: Vec<String> = (0..4).map(|i| p.non(&format!("b[{i}]"))).collect();
    let add = p.poser("ADD4", &[("a", "a"), ("re", &un)]);
    for (i, n) in nb.iter().enumerate() {
        p.relier(n, format!("{add}.b[{i}]"));
    }
    let ge = format!("{add}.rs");
    let egal = format!("{}.egal", p.poser("EGAL4", &[("a", "a"), ("b", "b")]));
    let ne = p.non(&egal);
    let sup = p.et(&ge, &ne);
    let inf = p.non(&ge);
    p.relier(&egal, "egal");
    p.relier(sup, "sup");
    p.relier(inf, "inf");
    p.fin()
}

fn soustracteur4() -> Composant {
    let mut p = Plan::nouveau("d = a − b modulo 16 ; emprunt = 1 quand a < b")
        .entree("a", 4)
        .entree("b", 4)
        .sortie("d", 4)
        .sortie("emprunt", 1);
    let un = p.un("a[0]");
    let add = p.poser("ADD4", &[("a", "a"), ("re", &un)]);
    for i in 0..4 {
        let n = p.non(&format!("b[{i}]"));
        p.relier(n, format!("{add}.b[{i}]"));
    }
    p.relier(format!("{add}.s"), "d");
    let e = p.non(&format!("{add}.rs"));
    p.relier(e, "emprunt");
    p.fin()
}

fn negatif8() -> Composant {
    let mut p = Plan::nouveau("s = −a en complément à deux (256 − a, modulo 256)")
        .entree("a", 8)
        .sortie("s", 8);
    let inc = p.poser("INCR8", &[("s", "s")]);
    for i in 0..8 {
        let n = p.non(&format!("a[{i}]"));
        p.relier(n, format!("{inc}.e[{i}]"));
    }
    p.fin()
}

fn parite8() -> Composant {
    let mut p = Plan::nouveau("p = 1 quand a contient un nombre impair de 1")
        .entree("a", 8)
        .sortie("p", 1);
    let s = p.arbre("OU_X", bits("a", 8));
    p.relier(s, "p");
    p.fin()
}

fn decaleur8() -> Composant {
    let mut p = Plan::nouveau(
        "s = a décalé de n rangs vers les poids forts (droite = 0) ou faibles (droite = 1), des 0 entrent",
    )
    .entree("a", 8)
    .entree("n", 3)
    .entree("droite", 1)
    .sortie("s", 8);
    let zero = p.zero("droite");
    // Vers la droite = retourner, décaler vers la gauche, retourner.
    let mut v: Vec<String> = (0..8)
        .map(|i| p.mux(&format!("a[{i}]"), &format!("a[{}]", 7 - i), "droite"))
        .collect();
    for etage in 0..3 {
        let pas = 1usize << etage;
        let sel = format!("n[{etage}]");
        v = (0..8)
            .map(|i| {
                let entrant = if i >= pas {
                    v[i - pas].clone()
                } else {
                    zero.clone()
                };
                p.mux(&v[i], &entrant, &sel)
            })
            .collect();
    }
    for i in 0..8 {
        let s = p.mux(&v[i], &v[7 - i], "droite");
        p.relier(s, format!("s[{i}]"));
    }
    p.fin()
}

fn mult4() -> Composant {
    let mut p = Plan::nouveau("p = a × b")
        .entree("a", 4)
        .entree("b", 4)
        .sortie("p", 8);
    let zero = p.zero("a[0]");
    let pp = |p: &mut Plan, i: usize, j: usize| p.et(&format!("a[{i}]"), &format!("b[{j}]"));
    let p00 = pp(&mut p, 0, 0);
    p.relier(p00, "p[0]");
    let mut acc: Vec<String> = (1..4).map(|i| pp(&mut p, i, 0)).collect();
    acc.push(zero.clone());
    for j in 1..4 {
        let add = p.poser("ADD4", &[("re", &zero)]);
        for (i, x) in acc.iter().enumerate() {
            p.relier(x, format!("{add}.a[{i}]"));
            let y = pp(&mut p, i, j);
            p.relier(y, format!("{add}.b[{i}]"));
        }
        p.relier(format!("{add}.s[0]"), format!("p[{j}]"));
        acc = (1..4).map(|i| format!("{add}.s[{i}]")).collect();
        acc.push(format!("{add}.rs"));
    }
    for (i, x) in acc.iter().enumerate() {
        p.relier(x, format!("p[{}]", 4 + i));
    }
    p.fin()
}

/// Division avec restauration : à chaque rang, on abaisse un bit de a et on soustrait b si
/// c'est possible. Si b = 0, q = 15 et r = a.
fn diviseur4() -> Composant {
    let mut p = Plan::nouveau("a = q × b + r avec r < b (si b = 0 : q = 15, r = a)")
        .entree("a", 4)
        .entree("b", 4)
        .sortie("q", 4)
        .sortie("r", 4);
    let zero = p.zero("a[0]");
    let un = p.non(&zero);
    let nb: Vec<String> = (0..4).map(|i| p.non(&format!("b[{i}]"))).collect();
    let mut reste: Vec<String> = vec![zero.clone(); 4];
    for rang in (0..4).rev() {
        // t = 2·reste + a[rang], sur 5 bits.
        let mut t = vec![format!("a[{rang}]")];
        t.extend(reste.iter().cloned());
        // t − b sur 8 bits : t + (non b sur 8 bits) + 1 ; la retenue vaut 1 quand t ≥ b.
        let add = p.poser("ADD8", &[("re", &un)]);
        for k in 0..8 {
            let x = t.get(k).cloned().unwrap_or_else(|| zero.clone());
            p.relier(x, format!("{add}.a[{k}]"));
            let y = nb.get(k).cloned().unwrap_or_else(|| un.clone());
            p.relier(y, format!("{add}.b[{k}]"));
        }
        let ge = format!("{add}.rs");
        p.relier(&ge, format!("q[{rang}]"));
        reste = (0..4)
            .map(|k| p.mux(&t[k], &format!("{add}.s[{k}]"), &ge))
            .collect();
    }
    for (k, x) in reste.iter().enumerate() {
        p.relier(x, format!("r[{k}]"));
    }
    p.fin()
}

/// Segments allumés pour chaque chiffre hexadécimal : bit 0 = a (haut), puis b, c, d, e, f,
/// et bit 6 = g (milieu).
pub const SEGMENTS: [u8; 16] = [
    0x3F, 0x06, 0x5B, 0x4F, 0x66, 0x6D, 0x7D, 0x07, 0x7F, 0x6F, 0x77, 0x7C, 0x39, 0x5E, 0x79, 0x71,
];

fn sept_segments() -> Composant {
    let mut p = Plan::nouveau("seg = segments à allumer pour écrire le chiffre hexadécimal n")
        .entree("n", 4)
        .sortie("seg", 7);
    let d = p.poser("DECODEUR4", &[("e", "n")]);
    for seg in 0..7 {
        // Un segment est allumé sauf pour les chiffres où il est éteint : on prend la liste
        // la plus courte.
        let eteints: Vec<String> = (0..16)
            .filter(|&v| SEGMENTS[v] >> seg & 1 == 0)
            .map(|v| format!("{d}.s[{v}]"))
            .collect();
        let un = eteints.is_empty();
        let s = if un {
            p.un("n[0]")
        } else {
            let o = p.arbre("OU", eteints);
            p.non(&o)
        };
        p.relier(s, format!("seg[{seg}]"));
    }
    p.fin()
}

fn bascule_t() -> Composant {
    let mut p = Plan::nouveau("au front montant de clk : q change si t = 1 ; q = 0 si raz = 1")
        .entree("t", 1)
        .entree("raz", 1)
        .entree("clk", 1)
        .sortie("q", 1);
    let b = p.poser("BASCULE_D", &[("clk", "clk")]);
    let x = p.oux("t", &format!("{b}.q"));
    let nr = p.non("raz");
    let d = p.et(&x, &nr);
    p.relier(d, format!("{b}.d"));
    p.relier(format!("{b}.q"), "q");
    p.fin()
}

fn bascule_jk() -> Composant {
    let mut p = Plan::nouveau(
        "au front montant de clk : j = 1 met q à 1, k = 1 le met à 0, les deux l'inversent ; raz = 1 le met à 0",
    )
    .entree("j", 1)
    .entree("k", 1)
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("q", 1);
    let b = p.poser("BASCULE_D", &[("clk", "clk")]);
    let q = format!("{b}.q");
    let nk = p.non("k");
    let suivant = p.mux("j", &nk, &q);
    let nr = p.non("raz");
    let d = p.et(&suivant, &nr);
    p.relier(d, format!("{b}.d"));
    p.relier(q, "q");
    p.fin()
}

fn decalage8() -> Composant {
    let mut p = Plan::nouveau("au front montant de clk, q se décale d'un rang et e entre en q[0]")
        .entree("e", 1)
        .entree("clk", 1)
        .sortie("q", 8);
    let mut precedent = "e".to_owned();
    for i in 0..8 {
        let b = p.poser("BASCULE_D", &[("d", &precedent), ("clk", "clk")]);
        precedent = format!("{b}.q");
        p.relier(&precedent, format!("q[{i}]"));
    }
    p.fin()
}

/// Ajoute 1 à `q` avec une chaîne de demi-additionneurs ; renvoie les bits de la somme et la
/// retenue finale.
fn plus_un(p: &mut Plan, q: &[String], un: &str) -> (Vec<String>, String) {
    let mut retenue = un.to_owned();
    let mut s = Vec::new();
    for x in q {
        let h = p.poser("DEMI_ADD", &[("a", x), ("b", &retenue)]);
        s.push(format!("{h}.s"));
        retenue = format!("{h}.r");
    }
    (s, retenue)
}

fn compteur_bcd() -> Composant {
    let mut p = Plan::nouveau(
        "au front montant de clk, q compte de 0 à 9 puis revient à 0 ; raz = 1 le remet à 0 ; fin = 1 quand q = 9",
    )
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("q", 4)
    .sortie("fin", 1);
    let bs: Vec<String> = (0..4)
        .map(|_| p.poser("BASCULE_D", &[("clk", "clk")]))
        .collect();
    let q: Vec<String> = bs.iter().map(|b| format!("{b}.q")).collect();
    let un = p.un("raz");
    let (s, _) = plus_un(&mut p, &q, &un);
    let fin = p.et(&q[0], &q[3]);
    let remise = p.ou("raz", &fin);
    let garde = p.non(&remise);
    for i in 0..4 {
        let d = p.et(&s[i], &garde);
        p.relier(d, format!("{}.d", bs[i]));
        p.relier(&q[i], format!("q[{i}]"));
    }
    p.relier(fin, "fin");
    p.fin()
}

fn compteur_hb4() -> Composant {
    let mut p = Plan::nouveau(
        "au front montant de clk, q augmente de 1 si haut = 1, diminue de 1 sinon (modulo 16) ; raz = 1 le remet à 0",
    )
    .entree("raz", 1)
    .entree("haut", 1)
    .entree("clk", 1)
    .sortie("q", 4);
    let bs: Vec<String> = (0..4)
        .map(|_| p.poser("BASCULE_D", &[("clk", "clk")]))
        .collect();
    // q + 1 = q + 0 + 1 ; q − 1 = q + 15 + 0.
    let nh = p.non("haut");
    let add = p.poser("ADD4", &[("re", "haut")]);
    let nr = p.non("raz");
    for (i, b) in bs.iter().enumerate() {
        p.relier(format!("{b}.q"), format!("{add}.a[{i}]"));
        p.relier(&nh, format!("{add}.b[{i}]"));
        let d = p.et(&format!("{add}.s[{i}]"), &nr);
        p.relier(d, format!("{b}.d"));
        p.relier(format!("{b}.q"), format!("q[{i}]"));
    }
    p.fin()
}

fn lfsr8() -> Composant {
    let mut p = Plan::nouveau(
        "générateur pseudo-aléatoire : au front montant de clk, q se décale et q[0] reçoit q7 ⊕ q5 ⊕ q4 ⊕ q3 ; raz = 1 met q à 1",
    )
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("q", 8);
    let bs: Vec<String> = (0..8)
        .map(|_| p.poser("BASCULE_D", &[("clk", "clk")]))
        .collect();
    let q: Vec<String> = bs.iter().map(|b| format!("{b}.q")).collect();
    let retour = p.arbre(
        "OU_X",
        vec![q[7].clone(), q[5].clone(), q[4].clone(), q[3].clone()],
    );
    let d0 = p.ou("raz", &retour);
    p.relier(d0, format!("{}.d", bs[0]));
    let nr = p.non("raz");
    for i in 1..8 {
        let d = p.et(&q[i - 1], &nr);
        p.relier(d, format!("{}.d", bs[i]));
    }
    for (i, x) in q.iter().enumerate() {
        p.relier(x, format!("q[{i}]"));
    }
    p.fin()
}

fn minuterie() -> Composant {
    let mut p = Plan::nouveau(
        "au front montant de clk : charger = 1 met q à n, sinon q descend de 1 jusqu'à 0 ; fini = 1 quand q = 0",
    )
    .entree("charger", 1)
    .entree("n", 8)
    .entree("clk", 1)
    .sortie("q", 8)
    .sortie("fini", 1);
    let bs: Vec<String> = (0..8)
        .map(|_| p.poser("BASCULE_D", &[("clk", "clk")]))
        .collect();
    let q: Vec<String> = bs.iter().map(|b| format!("{b}.q")).collect();
    let zero = p.zero("charger");
    let un = p.non(&zero);
    let moins = p.poser("ADD8", &[("re", &zero)]);
    let nul = p.poser("NUL8", &[]);
    for (i, x) in q.iter().enumerate() {
        p.relier(x, format!("{moins}.a[{i}]"));
        p.relier(&un, format!("{moins}.b[{i}]"));
        p.relier(x, format!("{nul}.e[{i}]"));
        p.relier(x, format!("q[{i}]"));
    }
    let fini = format!("{nul}.z");
    for (i, b) in bs.iter().enumerate() {
        let garde = p.mux(&format!("{moins}.s[{i}]"), &q[i], &fini);
        let d = p.mux(&garde, &format!("n[{i}]"), "charger");
        p.relier(d, format!("{b}.d"));
    }
    p.relier(fini, "fini");
    p.fin()
}

fn feu() -> Composant {
    let mut p = Plan::nouveau(
        "feu tricolore : un compteur de 0 à 7 avance à chaque front de clk ; vert de 0 à 2, orange à 3, rouge de 4 à 7 ; raz = 1 revient à 0",
    )
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("vert", 1)
    .sortie("orange", 1)
    .sortie("rouge", 1);
    let bs: Vec<String> = (0..3)
        .map(|_| p.poser("BASCULE_D", &[("clk", "clk")]))
        .collect();
    let q: Vec<String> = bs.iter().map(|b| format!("{b}.q")).collect();
    let un = p.un("raz");
    let (s, _) = plus_un(&mut p, &q, &un);
    let nr = p.non("raz");
    for i in 0..3 {
        let d = p.et(&s[i], &nr);
        p.relier(d, format!("{}.d", bs[i]));
    }
    let n2 = p.non(&q[2]);
    let trois = p.et(&q[0], &q[1]);
    let orange = p.et(&n2, &trois);
    let pas_trois = p.non(&trois);
    let vert = p.et(&n2, &pas_trois);
    p.relier(orange, "orange");
    p.relier(vert, "vert");
    p.relier(&q[2], "rouge");
    p.fin()
}

fn chronometre() -> Composant {
    let mut p = Plan::nouveau(
        "secondes de 00 à 59 : u (unités) avance à chaque front de clk, d (dizaines) quand u repasse de 9 à 0 ; raz = 1 remet tout à 0",
    )
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("u", 4)
    .sortie("d", 4);
    let u = p.poser(
        "COMPTEUR_BCD",
        &[("raz", "raz"), ("clk", "clk"), ("q", "u")],
    );
    // Les dizaines : un registre qui avance quand les unités finissent.
    let bs: Vec<String> = (0..4)
        .map(|_| p.poser("BASCULE_D", &[("clk", "clk")]))
        .collect();
    let q: Vec<String> = bs.iter().map(|b| format!("{b}.q")).collect();
    let un = p.un("raz");
    let (s, _) = plus_un(&mut p, &q, &un);
    let fin_u = format!("{u}.fin");
    // Cinq dizaines : 5 = 0101.
    let cinq = p.et(&q[0], &q[2]);
    let tout = p.et(&cinq, &fin_u);
    let remise = p.ou("raz", &tout);
    let garde = p.non(&remise);
    for i in 0..4 {
        let avance = p.mux(&q[i], &s[i], &fin_u);
        let d = p.et(&avance, &garde);
        p.relier(d, format!("{}.d", bs[i]));
        p.relier(&q[i], format!("d[{i}]"));
    }
    p.fin()
}

fn banc_registres() -> Composant {
    let mut p = Plan::nouveau(
        "quatre registres de 8 bits : a et b lisent les registres ad_a et ad_b ; au front montant de clk, si ecrire = 1, le registre ad_e prend d",
    )
    .entree("ecrire", 1)
    .entree("ad_e", 2)
    .entree("d", 8)
    .entree("ad_a", 2)
    .entree("ad_b", 2)
    .entree("clk", 1)
    .sortie("a", 8)
    .sortie("b", 8);
    let dec = p.poser("DECODEUR2", &[("a", "ad_e")]);
    let regs: Vec<String> = (0..4)
        .map(|i| {
            let charge = p.et("ecrire", &format!("{dec}.s[{i}]"));
            p.poser(
                "REGISTRE8",
                &[("d", "d"), ("charge", &charge), ("clk", "clk")],
            )
        })
        .collect();
    for (sortie, ad) in [("a", "ad_a"), ("b", "ad_b")] {
        let bas = p.poser(
            "MUX8",
            &[
                ("a", &format!("{}.q", regs[0])),
                ("b", &format!("{}.q", regs[1])),
                ("sel", &format!("{ad}[0]")),
            ],
        );
        let haut = p.poser(
            "MUX8",
            &[
                ("a", &format!("{}.q", regs[2])),
                ("b", &format!("{}.q", regs[3])),
                ("sel", &format!("{ad}[0]")),
            ],
        );
        p.poser(
            "MUX8",
            &[
                ("a", &format!("{bas}.s")),
                ("b", &format!("{haut}.s")),
                ("sel", &format!("{ad}[1]")),
                ("s", sortie),
            ],
        );
    }
    p.fin()
}

/// Quatre niveaux de 8 bits et un « thermomètre » de quatre bits qui dit lesquels sont
/// occupés. Les niveaux se décalent tous ensemble.
fn pile() -> Composant {
    let mut p = Plan::nouveau(
        "pile de 4 octets : au front montant de clk, empiler pose d au sommet, dépiler retire le sommet (les deux à la fois : rien) ; raz = 1 la vide",
    )
    .entree("d", 8)
    .entree("empiler", 1)
    .entree("depiler", 1)
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("sommet", 8)
    .sortie("vide", 1)
    .sortie("plein", 1);
    let un = p.un("raz");
    let zero = p.non(&un);
    let nd = p.non("depiler");
    let ne = p.non("empiler");
    let pousse = p.et("empiler", &nd);
    let tire = p.et("depiler", &ne);
    let niveaux: Vec<String> = (0..4)
        .map(|_| p.poser("REGISTRE8", &[("charge", &un), ("clk", "clk")]))
        .collect();
    for i in 0..4 {
        let dessus = if i == 0 {
            "d".to_owned()
        } else {
            format!("{}.q", niveaux[i - 1])
        };
        let ici = format!("{}.q", niveaux[i]);
        let m = p.poser("MUX8", &[("a", &ici), ("sel", &tire)]);
        // Sous le dernier niveau, des 0 remontent.
        if i < 3 {
            p.relier(format!("{}.q", niveaux[i + 1]), format!("{m}.b"));
        } else {
            for k in 0..8 {
                p.relier(&zero, format!("{m}.b[{k}]"));
            }
        }
        p.poser(
            "MUX8",
            &[
                ("a", &format!("{m}.s")),
                ("b", &dessus),
                ("sel", &pousse),
                ("s", &format!("{}.d", niveaux[i])),
            ],
        );
    }
    p.relier(format!("{}.q", niveaux[0]), "sommet");
    let t: Vec<String> = (0..4)
        .map(|_| p.poser("BASCULE_D", &[("clk", "clk")]))
        .collect();
    let nr = p.non("raz");
    for i in 0..4 {
        let ici = format!("{}.q", t[i]);
        let dessous = if i < 3 {
            format!("{}.q", t[i + 1])
        } else {
            zero.clone()
        };
        let dessus = if i == 0 {
            un.clone()
        } else {
            format!("{}.q", t[i - 1])
        };
        let m = p.mux(&ici, &dessous, &tire);
        let m = p.mux(&m, &dessus, &pousse);
        let d = p.et(&m, &nr);
        p.relier(d, format!("{}.d", t[i]));
    }
    let vide = p.non(&format!("{}.q", t[0]));
    p.relier(vide, "vide");
    p.relier(format!("{}.q", t[3]), "plein");
    p.fin()
}

/// Quatre cases de 8 bits ; la plus ancienne est toujours en case 0. Lire décale tout vers la
/// case 0 ; écrire remplit la première case libre.
fn file() -> Composant {
    let mut p = Plan::nouveau(
        "file de 4 octets (premier entré, premier sorti) : au front montant de clk, ecrire ajoute d si elle n'est pas pleine, lire retire le plus ancien si elle n'est pas vide (les deux à la fois : rien) ; raz = 1 la vide",
    )
    .entree("d", 8)
    .entree("ecrire", 1)
    .entree("lire", 1)
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("sortie", 8)
    .sortie("vide", 1)
    .sortie("plein", 1);
    let un = p.un("raz");
    let zero = p.non(&un);
    let t: Vec<String> = (0..4)
        .map(|_| p.poser("BASCULE_D", &[("clk", "clk")]))
        .collect();
    let tq: Vec<String> = t.iter().map(|b| format!("{b}.q")).collect();
    let vide = p.non(&tq[0]);
    let plein = tq[3].clone();
    let nl = p.non("lire");
    let ne = p.non("ecrire");
    let pas_plein = p.non(&plein);
    let ecrit = {
        let e = p.et("ecrire", &nl);
        p.et(&e, &pas_plein)
    };
    let lit = {
        let l = p.et("lire", &ne);
        p.et(&l, &tq[0])
    };
    let cases: Vec<String> = (0..4)
        .map(|_| p.poser("REGISTRE8", &[("charge", &un), ("clk", "clk")]))
        .collect();
    for i in 0..4 {
        let ici = format!("{}.q", cases[i]);
        let suivante = if i < 3 {
            format!("{}.q", cases[i + 1])
        } else {
            ici.clone()
        };
        // La première case libre : celle-ci est libre et la précédente occupée.
        let libre = p.non(&tq[i]);
        let premiere = if i == 0 {
            libre
        } else {
            p.et(&libre, &tq[i - 1])
        };
        let ici_ecrit = p.et(&ecrit, &premiere);
        let m = p.poser("MUX8", &[("a", &ici), ("b", &suivante), ("sel", &lit)]);
        p.poser(
            "MUX8",
            &[
                ("a", &format!("{m}.s")),
                ("b", "d"),
                ("sel", &ici_ecrit),
                ("s", &format!("{}.d", cases[i])),
            ],
        );
    }
    p.relier(format!("{}.q", cases[0]), "sortie");
    let nr = p.non("raz");
    for i in 0..4 {
        let dessous = if i < 3 {
            tq[i + 1].clone()
        } else {
            zero.clone()
        };
        let dessus = if i == 0 {
            un.clone()
        } else {
            tq[i - 1].clone()
        };
        let m = p.mux(&tq[i], &dessous, &lit);
        let m = p.mux(&m, &dessus, &ecrit);
        let d = p.et(&m, &nr);
        p.relier(d, format!("{}.d", t[i]));
    }
    p.relier(vide, "vide");
    p.relier(plein, "plein");
    p.fin()
}

/// Une trame série : un bit de départ à 0, les 8 bits de d (le plus faible d'abord), un bit
/// d'arrêt à 1. Dix bascules gardent la trame, dix autres comptent les bits qui restent.
fn emetteur_serie() -> Composant {
    let mut p = Plan::nouveau(
        "liaison série : au front montant de clk, si envoyer = 1 et rien n'est en cours, la trame part (0, d[0] … d[7], 1), un bit par front ; tx vaut 1 au repos ; raz = 1 arrête tout",
    )
    .entree("d", 8)
    .entree("envoyer", 1)
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("tx", 1)
    .sortie("occupe", 1);
    let un = p.un("raz");
    let zero = p.non(&un);
    let trame: Vec<String> = (0..10)
        .map(|_| p.poser("BASCULE_D", &[("clk", "clk")]))
        .collect();
    let reste: Vec<String> = (0..10)
        .map(|_| p.poser("BASCULE_D", &[("clk", "clk")]))
        .collect();
    let occupe = format!("{}.q", reste[0]);
    let libre = p.non(&occupe);
    let nr = p.non("raz");
    let part = {
        let e = p.et("envoyer", &libre);
        p.et(&e, &nr)
    };
    // Ce que la trame prend au départ.
    let mut neuve = vec![zero.clone()];
    neuve.extend(bits("d", 8));
    neuve.push(un.clone());
    for i in 0..10 {
        let decale = if i < 9 {
            format!("{}.q", trame[i + 1])
        } else {
            un.clone()
        };
        let d = p.mux(&decale, &neuve[i], &part);
        p.relier(d, format!("{}.d", trame[i]));
        let suivant = if i < 9 {
            format!("{}.q", reste[i + 1])
        } else {
            zero.clone()
        };
        let r = p.mux(&suivant, &un, &part);
        let r = p.et(&r, &nr);
        p.relier(r, format!("{}.d", reste[i]));
    }
    // tx = trame[0] pendant l'envoi, 1 au repos.
    let nt = p.non(&format!("{}.q", trame[0]));
    let tx = p.nand(&occupe, &nt);
    p.relier(tx, "tx");
    p.relier(&occupe, "occupe");
    p.fin()
}

fn interruptions() -> Composant {
    let mut p = Plan::nouveau(
        "contrôleur d'interruptions : une demande irq[i] reste en attente jusqu'à son acquittement ; int = 1 s'il y en a une, num = la plus prioritaire (le plus grand i) ; au front montant de clk, ack = 1 acquitte num ; raz = 1 efface tout",
    )
    .entree("irq", 4)
    .entree("ack", 1)
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("int", 1)
    .sortie("num", 2);
    let att: Vec<String> = (0..4)
        .map(|_| p.poser("BASCULE_D", &[("clk", "clk")]))
        .collect();
    let enc = p.poser("ENCODEUR4", &[("n", "num"), ("valide", "int")]);
    let dec = p.poser("DECODEUR2", &[("a", &format!("{enc}.n"))]);
    let nr = p.non("raz");
    for (i, a) in att.iter().enumerate() {
        let q = format!("{a}.q");
        p.relier(&q, format!("{enc}.d[{i}]"));
        let efface = p.et("ack", &format!("{dec}.s[{i}]"));
        let garde_bit = p.non(&efface);
        let garde = p.et(&q, &garde_bit);
        let d = p.ou(&format!("irq[{i}]"), &garde);
        let d = p.et(&d, &nr);
        p.relier(d, format!("{a}.d"));
    }
    p.fin()
}

/// Le détecteur de front : une bascule garde e tel qu'il était au dernier front de clk ;
/// front = e ET NON(ancien).
fn detecteur_front() -> Composant {
    let mut p = Plan::nouveau(
        "détecteur de front : front = 1 quand e vaut 1 alors qu'il valait 0 au dernier front montant de clk",
    )
    .entree("e", 1)
    .entree("clk", 1)
    .sortie("front", 1);
    let b = p.poser("BASCULE_D", &[("d", "e"), ("clk", "clk")]);
    let na = p.non(&format!("{b}.q"));
    let f = p.et("e", &na);
    p.relier(f, "front");
    p.fin()
}

/// La cadence : un compteur de 0 à 7 ; tic = 1 quand il vaut 7.
fn cadence() -> Composant {
    let mut p = Plan::nouveau(
        "cadence : au front montant de clk, q compte de 0 à 7 puis revient à 0 ; tic = 1 quand q = 7 ; raz = 1 remet q à 0",
    )
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("q", 3)
    .sortie("tic", 1);
    let bs: Vec<String> = (0..3)
        .map(|_| p.poser("BASCULE_D", &[("clk", "clk")]))
        .collect();
    let q: Vec<String> = bs.iter().map(|b| format!("{b}.q")).collect();
    let un = p.un("raz");
    let (s, _) = plus_un(&mut p, &q, &un);
    let nr = p.non("raz");
    for i in 0..3 {
        let d = p.et(&s[i], &nr);
        p.relier(d, format!("{}.d", bs[i]));
        p.relier(&q[i], format!("q[{i}]"));
    }
    let t = p.et(&q[0], &q[1]);
    let t = p.et(&t, &q[2]);
    p.relier(t, "tic");
    p.fin()
}

/// Le contrôle de parité : ok = NON(parité de d OU_X p).
fn controle_parite() -> Composant {
    let mut p = Plan::nouveau(
        "contrôle de parité : ok = 1 quand d et p ensemble contiennent un nombre pair de 1",
    )
    .entree("d", 8)
    .entree("p", 1)
    .sortie("ok", 1);
    let par = p.poser("PARITE8", &[("a", "d")]);
    let ok = p.porte("EQUIV", &[("a", &format!("{par}.p")), ("b", "p")]);
    p.relier(ok, "ok");
    p.fin()
}

/// Le récepteur série. Au repos, un 0 sur rx annonce une trame : neuf bascules `reste`
/// passent à 1 et se vident d'un rang par front. Tant qu'il en reste plus d'une, rx entre
/// dans le registre à décalage `sr` ; au dernier front (le bit d'arrêt), un 1 recopie `sr`
/// dans `d` et lève `pret`, un 0 lève `erreur`.
fn recepteur_serie() -> Composant {
    let mut p = Plan::nouveau(
        "récepteur série : au repos, un 0 sur rx au front montant de clk annonce une trame ; les 8 fronts suivants lisent d[0] à d[7], le suivant le bit d'arrêt : s'il vaut 1, d prend l'octet et pret vaut 1 jusqu'au front suivant, sinon erreur vaut 1 ; raz = 1 arrête tout et met d à 0",
    )
    .entree("rx", 1)
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("d", 8)
    .sortie("pret", 1)
    .sortie("erreur", 1);
    let un = p.un("raz");
    let zero = p.non(&un);
    let nr = p.non("raz");
    let reste: Vec<String> = (0..9)
        .map(|_| p.poser("BASCULE_D", &[("clk", "clk")]))
        .collect();
    let rq: Vec<String> = reste.iter().map(|b| format!("{b}.q")).collect();
    let actif = rq[0].clone();
    let repos = p.non(&actif);
    let nrx = p.non("rx");
    let debut = p.et(&repos, &nrx);
    let encore = rq[1].clone();
    let n_encore = p.non(&encore);
    let dernier = p.et(&actif, &n_encore);
    // Les bits qui restent : neuf 1 au départ, puis un décalage par front.
    for i in 0..9 {
        let suivant = if i < 8 {
            rq[i + 1].clone()
        } else {
            zero.clone()
        };
        let m = p.mux(&suivant, &un, &debut);
        let d = p.et(&m, &nr);
        p.relier(d, format!("{}.d", reste[i]));
    }
    // Le registre à décalage des données : rx entre par le haut tant qu'il reste des bits.
    let decale = p.et(&actif, &encore);
    let sr: Vec<String> = (0..8)
        .map(|_| p.poser("BASCULE_D", &[("clk", "clk")]))
        .collect();
    for i in 0..8 {
        let ici = format!("{}.q", sr[i]);
        let entre = if i < 7 {
            format!("{}.q", sr[i + 1])
        } else {
            "rx".to_owned()
        };
        let d = p.mux(&ici, &entre, &decale);
        p.relier(d, format!("{}.d", sr[i]));
    }
    // L'octet reçu, gardé jusqu'à la trame suivante.
    let bon = p.et(&dernier, "rx");
    let charge = p.ou(&bon, "raz");
    for (i, s) in sr.iter().enumerate() {
        let b = p.poser("BASCULE_D", &[("clk", "clk")]);
        let q = format!("{b}.q");
        let neuf = p.et(&format!("{s}.q"), &nr);
        let d = p.mux(&q, &neuf, &charge);
        p.relier(d, format!("{b}.d"));
        p.relier(&q, format!("d[{i}]"));
    }
    let pret = p.poser("BASCULE_D", &[("clk", "clk")]);
    let d = p.et(&bon, &nr);
    p.relier(d, format!("{pret}.d"));
    p.relier(format!("{pret}.q"), "pret");
    let erreur = p.poser("BASCULE_D", &[("clk", "clk")]);
    let mauvais = p.et(&dernier, &nrx);
    let d = p.et(&mauvais, &nr);
    p.relier(d, format!("{erreur}.d"));
    p.relier(format!("{erreur}.q"), "erreur");
    p.fin()
}

/// Deux bouts d'une liaison série reliés par un seul fil : l'émetteur parle, le récepteur
/// écoute.
fn liaison_serie() -> Composant {
    let mut p = Plan::nouveau(
        "liaison série : un émetteur et un récepteur reliés par un seul fil, sur la même horloge ; l'octet d envoyé arrive dans recu, et pret vaut 1 au front qui suit son bit d'arrêt",
    )
    .entree("d", 8)
    .entree("envoyer", 1)
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("recu", 8)
    .sortie("pret", 1)
    .sortie("occupe", 1);
    let em = p.poser(
        "EMETTEUR_SERIE",
        &[
            ("d", "d"),
            ("envoyer", "envoyer"),
            ("raz", "raz"),
            ("clk", "clk"),
            ("occupe", "occupe"),
        ],
    );
    p.poser(
        "RECEPTEUR_SERIE",
        &[
            ("rx", &format!("{em}.tx")),
            ("raz", "raz"),
            ("clk", "clk"),
            ("d", "recu"),
            ("pret", "pret"),
        ],
    );
    p.fin()
}

/// La carte réseau : un émetteur, un récepteur, et une file de 4 octets où attendent ceux
/// qui sont arrivés.
fn carte_reseau() -> Composant {
    let mut p = Plan::nouveau(
        "carte réseau : envoyer = 1 fait partir d sur tx comme l'émetteur série ; les octets reçus sur rx attendent dans une file de 4 ; recu montre le plus ancien, dispo = 1 s'il y en a un, lire = 1 au front montant de clk le retire ; plein = 1 quand la file est pleine ; erreur vaut 1 un front après une trame mal terminée ; raz = 1 remet tout à zéro",
    )
    .entree("d", 8)
    .entree("envoyer", 1)
    .entree("rx", 1)
    .entree("lire", 1)
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("tx", 1)
    .sortie("occupe", 1)
    .sortie("recu", 8)
    .sortie("dispo", 1)
    .sortie("plein", 1)
    .sortie("erreur", 1);
    p.poser(
        "EMETTEUR_SERIE",
        &[
            ("d", "d"),
            ("envoyer", "envoyer"),
            ("raz", "raz"),
            ("clk", "clk"),
            ("tx", "tx"),
            ("occupe", "occupe"),
        ],
    );
    let re = p.poser(
        "RECEPTEUR_SERIE",
        &[
            ("rx", "rx"),
            ("raz", "raz"),
            ("clk", "clk"),
            ("erreur", "erreur"),
        ],
    );
    let file = p.poser(
        "FILE",
        &[
            ("d", &format!("{re}.d")),
            ("ecrire", &format!("{re}.pret")),
            ("lire", "lire"),
            ("raz", "raz"),
            ("clk", "clk"),
            ("sortie", "recu"),
            ("plein", "plein"),
        ],
    );
    let dispo = p.non(&format!("{file}.vide"));
    p.relier(dispo, "dispo");
    p.fin()
}

/// `n` bascules D sur `clk` ; renvoie leurs noms et leurs sorties.
fn bascules(p: &mut Plan, n: usize) -> (Vec<String>, Vec<String>) {
    let bs: Vec<String> = (0..n)
        .map(|_| p.poser("BASCULE_D", &[("clk", "clk")]))
        .collect();
    let q = bs.iter().map(|b| format!("{b}.q")).collect();
    (bs, q)
}

/// Un compteur de 0 à `max` qui avance quand `en` vaut 1. Renvoie les bascules, leurs
/// sorties, la valeur à charger au front (sans remise à zéro) et `en ET (q = max)`.
///
/// `q = max` se lit sur les seuls bits à 1 de `max` : une valeur au plus égale à `max` qui
/// les a tous vaut au moins `max`, donc `max`.
fn compteur_jusqu_a(
    p: &mut Plan,
    en: &str,
    un: &str,
    n: usize,
    max: u64,
) -> (Vec<String>, Vec<String>, Vec<String>, String) {
    let (bs, q) = bascules(p, n);
    let (s, _) = plus_un(p, &q, un);
    let uns: Vec<String> = (0..n)
        .filter(|i| max >> i & 1 == 1)
        .map(|i| q[i].clone())
        .collect();
    let au_max = p.arbre("ET", uns);
    let pas_max = p.non(&au_max);
    let suivant = (0..n)
        .map(|i| {
            let plus = p.et(&s[i], &pas_max);
            p.mux(&q[i], &plus, en)
        })
        .collect();
    let fin = p.et(en, &au_max);
    (bs, q, suivant, fin)
}

fn compteur_cascade() -> Composant {
    let mut p = Plan::nouveau(
        "compteur en cascade : au front montant de clk, si en = 1, q avance de 0 à 9 puis revient à 0 ; retenue = en ET (q = 9) ; raz = 1 remet q à 0",
    )
    .entree("en", 1)
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("q", 4)
    .sortie("retenue", 1);
    let un = p.un("raz");
    let nr = p.non("raz");
    let (bs, q, suivant, fin) = compteur_jusqu_a(&mut p, "en", &un, 4, 9);
    for i in 0..4 {
        let d = p.et(&suivant[i], &nr);
        p.relier(d, format!("{}.d", bs[i]));
        p.relier(&q[i], format!("q[{i}]"));
    }
    p.relier(fin, "retenue");
    p.fin()
}

/// Unités (0 à 9) et dizaines (0 à `dizaines_max`) en décimal codé binaire dans `q[8]`.
/// Renvoie les bascules et la valeur à charger, la remise à zéro restant à faire, plus les
/// sorties et la retenue des dizaines.
fn deux_chiffres(
    p: &mut Plan,
    en: &str,
    un: &str,
    zero: &str,
    dizaines_max: u64,
    bits_dizaines: usize,
) -> (Vec<String>, Vec<String>, Vec<String>, String) {
    let (bu, qu, su, fu) = compteur_jusqu_a(p, en, un, 4, 9);
    let (bd, qd, sd, fd) = compteur_jusqu_a(p, &fu, un, bits_dizaines, dizaines_max);
    let mut bs = bu;
    bs.extend(bd);
    let mut suivant = su;
    suivant.extend(sd);
    let mut q = qu;
    q.extend(qd);
    while q.len() < 8 {
        q.push(zero.to_owned());
    }
    (bs, suivant, q, fd)
}

fn compteur_60() -> Composant {
    let mut p = Plan::nouveau(
        "compteur de 00 à 59 : au front montant de clk, si en = 1, q avance d'une unité ; q[0..4] = unités, q[4..8] = dizaines ; retenue = en ET (q = 59) ; raz = 1 remet à 00",
    )
    .entree("en", 1)
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("q", 8)
    .sortie("retenue", 1);
    let un = p.un("raz");
    let zero = p.non(&un);
    let nr = p.non("raz");
    let (bs, suivant, q, fin) = deux_chiffres(&mut p, "en", &un, &zero, 5, 3);
    for (b, s) in bs.iter().zip(&suivant) {
        let d = p.et(s, &nr);
        p.relier(d, format!("{b}.d"));
    }
    for (i, x) in q.iter().enumerate() {
        p.relier(x, format!("q[{i}]"));
    }
    p.relier(fin, "retenue");
    p.fin()
}

fn compteur_24() -> Composant {
    let mut p = Plan::nouveau(
        "compteur de 00 à 23 : au front montant de clk, si en = 1, q avance d'une unité ; q[0..4] = unités, q[4..8] = dizaines ; après 23 vient 00 ; retenue = en ET (q = 23) ; raz = 1 remet à 00",
    )
    .entree("en", 1)
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("q", 8)
    .sortie("retenue", 1);
    let un = p.un("raz");
    let zero = p.non(&un);
    let (bs, suivant, q, _) = deux_chiffres(&mut p, "en", &un, &zero, 2, 2);
    // 23 : dizaine 2 (q[5]) et unités 3 (q[0], q[1]) ; les unités ne dépassent pas 3 quand
    // la dizaine vaut 2.
    let a = p.et(&q[5], &q[0]);
    let vingt_trois = p.et(&a, &q[1]);
    let fin = p.et("en", &vingt_trois);
    let tout_a_zero = p.ou(&fin, "raz");
    let garde = p.non(&tout_a_zero);
    for (b, s) in bs.iter().zip(&suivant) {
        let d = p.et(s, &garde);
        p.relier(d, format!("{b}.d"));
    }
    for (i, x) in q.iter().enumerate() {
        p.relier(x, format!("q[{i}]"));
    }
    p.relier(fin, "retenue");
    p.fin()
}

/// Égalité de deux octets avec deux tests d'égalité de 4 bits.
fn egal8(p: &mut Plan, a: &str, b: &str) -> String {
    let mut moities = Vec::new();
    for m in 0..2 {
        let e = p.poser("EGAL4", &[]);
        for i in 0..4 {
            p.relier(format!("{a}[{}]", 4 * m + i), format!("{e}.a[{i}]"));
            p.relier(format!("{b}[{}]", 4 * m + i), format!("{e}.b[{i}]"));
        }
        moities.push(format!("{e}.egal"));
    }
    p.et(&moities[0], &moities[1])
}

fn reveil() -> Composant {
    let mut p = Plan::nouveau(
        "réveil : au front montant de clk, regler = 1 retient h et m comme heure du réveil et l'arme ; ensuite, au premier front où h et m valent l'heure retenue, sonne passe à 1 et y reste ; arreter = 1 l'éteint ; raz = 1 éteint et désarme",
    )
    .entree("h", 8)
    .entree("m", 8)
    .entree("regler", 1)
    .entree("arreter", 1)
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("sonne", 1);
    let rh = p.poser(
        "REGISTRE8",
        &[("d", "h"), ("charge", "regler"), ("clk", "clk")],
    );
    let rm = p.poser(
        "REGISTRE8",
        &[("d", "m"), ("charge", "regler"), ("clk", "clk")],
    );
    let eh = egal8(&mut p, "h", &format!("{rh}.q"));
    let em = egal8(&mut p, "m", &format!("{rm}.q"));
    let egal = p.et(&eh, &em);
    let nr = p.non("raz");
    let (bs, q) = bascules(&mut p, 2);
    let (arme, sonne) = (q[0].clone(), q[1].clone());
    let a = p.ou(&arme, "regler");
    let a = p.et(&a, &nr);
    p.relier(a, format!("{}.d", bs[0]));
    let atteint = p.et(&arme, &egal);
    let s = p.ou(&sonne, &atteint);
    let na = p.non("arreter");
    let s = p.et(&s, &na);
    let s = p.et(&s, &nr);
    p.relier(s, format!("{}.d", bs[1]));
    p.relier(&sonne, "sonne");
    p.fin()
}

fn montre() -> Composant {
    let mut p = Plan::nouveau(
        "montre : au front montant de clk, en = 1 ajoute une seconde ; les secondes et les minutes vont de 00 à 59, les heures de 00 à 23 ; plus_m = 1 avance les minutes, plus_h les heures ; raz = 1 remet tout à 00:00:00",
    )
    .entree("en", 1)
    .entree("plus_m", 1)
    .entree("plus_h", 1)
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("s", 8)
    .sortie("m", 8)
    .sortie("h", 8);
    let s = p.poser(
        "COMPTEUR_60",
        &[("en", "en"), ("raz", "raz"), ("clk", "clk"), ("q", "s")],
    );
    let em = p.ou(&format!("{s}.retenue"), "plus_m");
    let m = p.poser(
        "COMPTEUR_60",
        &[("en", &em), ("raz", "raz"), ("clk", "clk"), ("q", "m")],
    );
    let eh = p.ou(&format!("{m}.retenue"), "plus_h");
    p.poser(
        "COMPTEUR_24",
        &[("en", &eh), ("raz", "raz"), ("clk", "clk"), ("q", "h")],
    );
    p.fin()
}

fn balayage() -> Composant {
    let mut p = Plan::nouveau(
        "balayage d'un écran de 8 × 8 : à chaque front montant de clk, x avance de 0 à 7 ; après 7, x revient à 0 et y avance ; fin_ligne = 1 quand x = 7, fin_image = 1 quand x = 7 et y = 7 ; raz = 1 revient en (0, 0)",
    )
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("x", 3)
    .sortie("y", 3)
    .sortie("fin_ligne", 1)
    .sortie("fin_image", 1);
    let un = p.un("raz");
    let nr = p.non("raz");
    let (bx, qx, sx, fx) = compteur_jusqu_a(&mut p, &un, &un, 3, 7);
    let (by, qy, sy, fy) = compteur_jusqu_a(&mut p, &fx, &un, 3, 7);
    for (b, s) in bx.iter().zip(&sx).chain(by.iter().zip(&sy)) {
        let d = p.et(s, &nr);
        p.relier(d, format!("{b}.d"));
    }
    for i in 0..3 {
        p.relier(&qx[i], format!("x[{i}]"));
        p.relier(&qy[i], format!("y[{i}]"));
    }
    p.relier(fx, "fin_ligne");
    p.relier(fy, "fin_image");
    p.fin()
}

/// Les chiffres hexadécimaux en 3 × 5 pixels : cinq lignes de haut en bas, bit 0 = pixel de
/// gauche.
pub const POLICE: [[u8; 5]; 16] = {
    // Chaque ligne s'écrit de gauche à droite, comme on la voit.
    const fn l(t: &[u8; 3]) -> u8 {
        (t[0] == b'#') as u8 | ((t[1] == b'#') as u8) << 1 | ((t[2] == b'#') as u8) << 2
    }
    [
        [l(b"###"), l(b"#.#"), l(b"#.#"), l(b"#.#"), l(b"###")],
        [l(b".#."), l(b"##."), l(b".#."), l(b".#."), l(b"###")],
        [l(b"###"), l(b"..#"), l(b"###"), l(b"#.."), l(b"###")],
        [l(b"###"), l(b"..#"), l(b"###"), l(b"..#"), l(b"###")],
        [l(b"#.#"), l(b"#.#"), l(b"###"), l(b"..#"), l(b"..#")],
        [l(b"###"), l(b"#.."), l(b"###"), l(b"..#"), l(b"###")],
        [l(b"###"), l(b"#.."), l(b"###"), l(b"#.#"), l(b"###")],
        [l(b"###"), l(b"..#"), l(b"..#"), l(b"..#"), l(b"..#")],
        [l(b"###"), l(b"#.#"), l(b"###"), l(b"#.#"), l(b"###")],
        [l(b"###"), l(b"#.#"), l(b"###"), l(b"..#"), l(b"###")],
        [l(b".#."), l(b"#.#"), l(b"###"), l(b"#.#"), l(b"#.#")],
        [l(b"##."), l(b"#.#"), l(b"##."), l(b"#.#"), l(b"##.")],
        [l(b".##"), l(b"#.."), l(b"#.."), l(b"#.."), l(b".##")],
        [l(b"##."), l(b"#.#"), l(b"#.#"), l(b"#.#"), l(b"##.")],
        [l(b"###"), l(b"#.."), l(b"###"), l(b"#.."), l(b"###")],
        [l(b"###"), l(b"#.."), l(b"###"), l(b"#.."), l(b"#..")],
    ]
};

fn police() -> Composant {
    let mut p = Plan::nouveau(
        "police de caractères : motif = les 3 pixels de la ligne « ligne » (0 en haut) du chiffre hexadécimal n, motif[0] à gauche ; les lignes 5 à 7 sont vides",
    )
    .entree("n", 4)
    .entree("ligne", 3)
    .sortie("motif", 3);
    let dn = p.poser("DECODEUR4", &[("e", "n")]);
    let dl = p.poser("DECODEUR3", &[("a", "ligne")]);
    for bit in 0..3 {
        let mut termes = Vec::new();
        for ligne in 0..5 {
            let chiffres: Vec<String> = POLICE
                .iter()
                .enumerate()
                .filter(|(_, dessin)| dessin[ligne] >> bit & 1 == 1)
                .map(|(c, _)| format!("{dn}.s[{c}]"))
                .collect();
            if chiffres.is_empty() {
                continue;
            }
            let o = p.arbre("OU", chiffres);
            termes.push(p.et(&o, &format!("{dl}.s[{ligne}]")));
        }
        let s = p.arbre("OU", termes);
        p.relier(s, format!("motif[{bit}]"));
    }
    p.fin()
}

/// Un multiplexeur à 8 voies d'un bit, choisi par `sel[3]`.
fn mux_8_voies(p: &mut Plan, voies: &[String], sel: &str) -> String {
    let mut niveau: Vec<String> = voies.to_vec();
    for k in 0..3 {
        niveau = niveau
            .chunks(2)
            .map(|c| p.mux(&c[0], &c[1], &format!("{sel}[{k}]")))
            .collect();
    }
    niveau.pop().unwrap()
}

fn memoire_video() -> Composant {
    let mut p = Plan::nouveau(
        "mémoire vidéo de 8 lignes de 8 pixels : au front montant de clk, si ecrire = 1, la ligne ligne_e prend d ; ligne montre la ligne y, pixel son bit x",
    )
    .entree("ligne_e", 3)
    .entree("d", 8)
    .entree("ecrire", 1)
    .entree("x", 3)
    .entree("y", 3)
    .entree("clk", 1)
    .sortie("pixel", 1)
    .sortie("ligne", 8);
    let dec = p.poser("DECODEUR3", &[("a", "ligne_e")]);
    let lignes: Vec<String> = (0..8)
        .map(|i| {
            let charge = p.et("ecrire", &format!("{dec}.s[{i}]"));
            let r = p.poser(
                "REGISTRE8",
                &[("d", "d"), ("charge", &charge), ("clk", "clk")],
            );
            format!("{r}.q")
        })
        .collect();
    // Choisir la ligne y : trois étages de multiplexeurs d'octets.
    let mut niveau = lignes;
    for k in 0..3 {
        niveau = niveau
            .chunks(2)
            .map(|c| {
                let m = p.poser(
                    "MUX8",
                    &[("a", &c[0]), ("b", &c[1]), ("sel", &format!("y[{k}]"))],
                );
                format!("{m}.s")
            })
            .collect();
    }
    let ligne = niveau.pop().unwrap();
    p.relier(&ligne, "ligne");
    let bits: Vec<String> = (0..8).map(|i| format!("{ligne}[{i}]")).collect();
    let px = mux_8_voies(&mut p, &bits, "x");
    p.relier(px, "pixel");
    p.fin()
}

fn serialiseur() -> Composant {
    let mut p = Plan::nouveau(
        "sérialiseur de pixels : au front montant de clk, charger = 1 prend l'octet d, sinon les bits avancent d'un rang vers le bit 0 et un 0 entre en haut ; pixel montre le bit 0",
    )
    .entree("d", 8)
    .entree("charger", 1)
    .entree("clk", 1)
    .sortie("pixel", 1);
    let zero = p.zero("charger");
    let (bs, q) = bascules(&mut p, 8);
    for i in 0..8 {
        let dessus = if i < 7 {
            q[i + 1].clone()
        } else {
            zero.clone()
        };
        let d = p.mux(&dessus, &format!("d[{i}]"), "charger");
        p.relier(d, format!("{}.d", bs[i]));
    }
    p.relier(&q[0], "pixel");
    p.fin()
}

fn controleur_ecran() -> Composant {
    let mut p = Plan::nouveau(
        "contrôleur d'écran : le balayage parcourt l'écran pixel par pixel, un par front de clk, et pixel donne celui de la mémoire vidéo en (x, y) ; ecrire = 1 au front range d dans la ligne ligne_e ; raz = 1 recommence le balayage en (0, 0)",
    )
    .entree("ligne_e", 3)
    .entree("d", 8)
    .entree("ecrire", 1)
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("x", 3)
    .sortie("y", 3)
    .sortie("pixel", 1)
    .sortie("fin_image", 1);
    let b = p.poser(
        "BALAYAGE",
        &[
            ("raz", "raz"),
            ("clk", "clk"),
            ("x", "x"),
            ("y", "y"),
            ("fin_image", "fin_image"),
        ],
    );
    p.poser(
        "MEMOIRE_VIDEO",
        &[
            ("ligne_e", "ligne_e"),
            ("d", "d"),
            ("ecrire", "ecrire"),
            ("x", &format!("{b}.x")),
            ("y", &format!("{b}.y")),
            ("clk", "clk"),
            ("pixel", "pixel"),
        ],
    );
    p.fin()
}

fn gray() -> Composant {
    let mut p = Plan::nouveau(
        "code de Gray : g[i] = b[i] ⊕ b[i+1], et g[3] = b[3] ; deux nombres qui se suivent ne diffèrent que d'un bit",
    )
    .entree("b", 4)
    .sortie("g", 4);
    for i in 0..3 {
        let g = p.oux(&format!("b[{i}]"), &format!("b[{}]", i + 1));
        p.relier(g, format!("g[{i}]"));
    }
    p.relier("b[3]", "g[3]");
    p.fin()
}

fn motif_101() -> Composant {
    let mut p = Plan::nouveau(
        "reconnaisseur de motif : à chaque front montant de clk, e est lu ; vu = 1 quand les trois dernières valeurs lues sont 1, 0, 1 (dans cet ordre) ; raz = 1 oublie tout",
    )
    .entree("e", 1)
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("vu", 1);
    let nr = p.non("raz");
    let (bs, q) = bascules(&mut p, 3);
    // q[0] : la dernière valeur lue, q[2] : l'avant-avant-dernière.
    let entrees = ["e".to_owned(), q[0].clone(), q[1].clone()];
    for i in 0..3 {
        let d = p.et(&entrees[i], &nr);
        p.relier(d, format!("{}.d", bs[i]));
    }
    let n1 = p.non(&q[1]);
    let a = p.et(&q[2], &n1);
    let vu = p.et(&a, &q[0]);
    p.relier(vu, "vu");
    p.fin()
}

fn divisible_3() -> Composant {
    let mut p = Plan::nouveau(
        "divisible par 3 : à chaque front montant de clk, e est le bit suivant d'un nombre lu poids fort d'abord ; div = 1 quand le nombre lu jusque-là est un multiple de 3 ; raz = 1 recommence (le nombre vaut 0)",
    )
    .entree("e", 1)
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("div", 1);
    let nr = p.non("raz");
    let (bs, q) = bascules(&mut p, 2);
    // Le reste r = 2 r1 + r0 ; lire e donne le reste de 2 r + e.
    let (r0, r1) = (q[0].clone(), q[1].clone());
    let ne = p.non("e");
    let n0 = p.non(&r0);
    let n1 = p.non(&r1);
    let a = p.et(&n1, &n0);
    let a = p.et(&a, "e");
    let b = p.et(&r1, &ne);
    let s0 = p.ou(&a, &b);
    let c = p.et(&r0, &ne);
    let d = p.et(&r1, "e");
    let s1 = p.ou(&c, &d);
    for (i, s) in [s0, s1].iter().enumerate() {
        let x = p.et(s, &nr);
        p.relier(x, format!("{}.d", bs[i]));
    }
    let div = p.et(&n0, &n1);
    p.relier(div, "div");
    p.fin()
}

fn cellule_vie() -> Composant {
    let mut p = Plan::nouveau(
        "cellule du jeu de la vie : s = 1 quand exactement 3 des 8 voisins n sont vivants, ou quand la cellule est vivante (v = 1) et en a exactement 2",
    )
    .entree("v", 1)
    .entree("n", 8)
    .sortie("s", 1);
    let zero = p.zero("v");
    // Compter les voisins vivants : on ajoute chaque bit, un additionneur de 4 bits à la fois.
    let mut somme: Vec<String> = vec![zero.clone(); 4];
    for i in 0..8 {
        let a = p.poser("ADD4", &[("re", &format!("n[{i}]"))]);
        for (k, x) in somme.iter().enumerate() {
            p.relier(x, format!("{a}.a[{k}]"));
            p.relier(&zero, format!("{a}.b[{k}]"));
        }
        somme = (0..4).map(|k| format!("{a}.s[{k}]")).collect();
    }
    let n2 = p.non(&somme[2]);
    let n3 = p.non(&somme[3]);
    let deux_ou_trois = {
        let a = p.et(&n2, &n3);
        p.et(&a, &somme[1])
    };
    let trois = p.et(&deux_ou_trois, &somme[0]);
    let n0 = p.non(&somme[0]);
    let deux = p.et(&deux_ou_trois, &n0);
    let reste = p.et(&deux, "v");
    let s = p.ou(&trois, &reste);
    p.relier(s, "s");
    p.fin()
}

fn automate_30() -> Composant {
    let mut p = Plan::nouveau(
        "automate cellulaire, règle 30 : huit cellules en anneau ; au front montant de clk, charger = 1 met q à d, sinon chaque cellule devient gauche ⊕ (elle-même OU droite), avec gauche = q[i+1] et droite = q[i−1]",
    )
    .entree("d", 8)
    .entree("charger", 1)
    .entree("clk", 1)
    .sortie("q", 8);
    let (bs, q) = bascules(&mut p, 8);
    for i in 0..8 {
        let gauche = &q[(i + 1) % 8];
        let droite = &q[(i + 7) % 8];
        let o = p.ou(&q[i], droite);
        let n = p.oux(gauche, &o);
        let d = p.mux(&n, &format!("d[{i}]"), "charger");
        p.relier(d, format!("{}.d", bs[i]));
        p.relier(&q[i], format!("q[{i}]"));
    }
    p.fin()
}

fn pgcd() -> Composant {
    let mut p = Plan::nouveau(
        "PGCD par l'algorithme d'Euclide : au front montant de clk, charger = 1 met x à a et y à b ; sinon le plus grand des deux devient leur différence ; fini = 1 quand x = y, ou quand l'un vaut 0 : le PGCD est alors x (ou y si x vaut 0)",
    )
    .entree("a", 4)
    .entree("b", 4)
    .entree("charger", 1)
    .entree("clk", 1)
    .sortie("x", 4)
    .sortie("y", 4)
    .sortie("fini", 1);
    let (bx, qx) = bascules(&mut p, 4);
    let (by, qy) = bascules(&mut p, 4);
    let cmp = p.poser("COMPARATEUR4", &[]);
    let xy = p.poser("SOUSTRACTEUR4", &[]);
    let yx = p.poser("SOUSTRACTEUR4", &[]);
    let zx = {
        let o = p.arbre("OU", qx.clone());
        p.non(&o)
    };
    let zy = {
        let o = p.arbre("OU", qy.clone());
        p.non(&o)
    };
    for i in 0..4 {
        p.relier(&qx[i], format!("{cmp}.a[{i}]"));
        p.relier(&qy[i], format!("{cmp}.b[{i}]"));
        p.relier(&qx[i], format!("{xy}.a[{i}]"));
        p.relier(&qy[i], format!("{xy}.b[{i}]"));
        p.relier(&qy[i], format!("{yx}.a[{i}]"));
        p.relier(&qx[i], format!("{yx}.b[{i}]"));
        let nx = p.mux(&qx[i], &format!("{xy}.d[{i}]"), &format!("{cmp}.sup"));
        let nx = p.mux(&nx, &format!("a[{i}]"), "charger");
        p.relier(nx, format!("{}.d", bx[i]));
        let ny = p.mux(&qy[i], &format!("{yx}.d[{i}]"), &format!("{cmp}.inf"));
        let ny = p.mux(&ny, &format!("b[{i}]"), "charger");
        p.relier(ny, format!("{}.d", by[i]));
        p.relier(&qx[i], format!("x[{i}]"));
        p.relier(&qy[i], format!("y[{i}]"));
    }
    let f = p.ou(&format!("{cmp}.egal"), &zx);
    let f = p.ou(&f, &zy);
    p.relier(f, "fini");
    p.fin()
}

fn cesar() -> Composant {
    let mut p = Plan::nouveau(
        "chiffre de César : c = (d + k) modulo 16 ; chaque lettre (ici un nombre de 0 à 15) avance de k rangs, et revient au début après 15",
    )
    .entree("d", 4)
    .entree("k", 4)
    .sortie("c", 4);
    let zero = p.zero("d[0]");
    let add = p.poser("ADD4", &[("a", "d"), ("b", "k"), ("re", &zero)]);
    p.relier(format!("{add}.s"), "c");
    p.fin()
}

fn masque_xor() -> Composant {
    let mut p = Plan::nouveau(
        "masque jetable : c = d ⊕ k, bit à bit ; appliquer deux fois la même clé k redonne le message",
    )
    .entree("d", 8)
    .entree("k", 8)
    .sortie("c", 8);
    for i in 0..8 {
        let c = p.oux(&format!("d[{i}]"), &format!("k[{i}]"));
        p.relier(c, format!("c[{i}]"));
    }
    p.fin()
}

fn somme_controle() -> Composant {
    let mut p = Plan::nouveau(
        "somme de contrôle : au front montant de clk, raz = 1 remet somme à 0 ; sinon ajouter = 1 ajoute l'octet d à somme, modulo 256",
    )
    .entree("d", 8)
    .entree("ajouter", 1)
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("somme", 8);
    let nr = p.non("raz");
    let zero = p.zero("raz");
    let (bs, q) = bascules(&mut p, 8);
    let add = p.poser("ADD8", &[("b", "d"), ("re", &zero)]);
    for i in 0..8 {
        p.relier(&q[i], format!("{add}.a[{i}]"));
        let s = p.mux(&q[i], &format!("{add}.s[{i}]"), "ajouter");
        let s = p.et(&s, &nr);
        p.relier(s, format!("{}.d", bs[i]));
        p.relier(&q[i], format!("somme[{i}]"));
    }
    p.fin()
}

fn hamming_code() -> Composant {
    let mut p = Plan::nouveau(
        "code de Hamming (7, 4) : les 4 bits de d prennent les places 3, 5, 6 et 7 de h ; les places 1, 2 et 4 reçoivent des bits de parité, p1 = d0 ⊕ d1 ⊕ d3, p2 = d0 ⊕ d2 ⊕ d3, p4 = d1 ⊕ d2 ⊕ d3 (h[0] est la place 1)",
    )
    .entree("d", 4)
    .sortie("h", 7);
    let d = bits("d", 4);
    for (place, termes) in [(0, [0, 1, 3]), (1, [0, 2, 3]), (3, [1, 2, 3])] {
        let x = p.arbre("OU_X", termes.iter().map(|&i| d[i].clone()).collect());
        p.relier(x, format!("h[{place}]"));
    }
    for (i, place) in [2, 4, 5, 6].into_iter().enumerate() {
        p.relier(&d[i], format!("h[{place}]"));
    }
    p.fin()
}

fn hamming_correcteur() -> Composant {
    let mut p = Plan::nouveau(
        "correcteur de Hamming (7, 4) : trois contrôles de parité donnent la place (de 1 à 7) du bit faux, 0 s'il n'y en a pas ; ce bit est retourné, puis d est relu aux places 3, 5, 6 et 7 ; erreur = 1 quand un bit a été corrigé",
    )
    .entree("h", 7)
    .sortie("d", 4)
    .sortie("erreur", 1);
    // Le contrôle k couvre les places dont le numéro a le bit k : 1, 3, 5, 7 ; 2, 3, 6, 7 ;
    // 4, 5, 6, 7.
    let controle: Vec<String> = (0..3)
        .map(|k| {
            let termes = (1..=7)
                .filter(|place: &usize| place >> k & 1 == 1)
                .map(|place| format!("h[{}]", place - 1))
                .collect();
            p.arbre("OU_X", termes)
        })
        .collect();
    let dec = p.poser("DECODEUR3", &[]);
    for (k, c) in controle.iter().enumerate() {
        p.relier(c, format!("{dec}.a[{k}]"));
    }
    for (i, place) in [3, 5, 6, 7].into_iter().enumerate() {
        let x = p.oux(&format!("h[{}]", place - 1), &format!("{dec}.s[{place}]"));
        p.relier(x, format!("d[{i}]"));
    }
    let e = p.arbre("OU", controle);
    p.relier(e, "erreur");
    p.fin()
}

fn arbitre() -> Composant {
    let mut p = Plan::nouveau(
        "arbitre de pierre-feuille-ciseaux : a et b valent 0 (pierre), 1 (feuille) ou 2 (ciseaux) ; la feuille bat la pierre, les ciseaux battent la feuille, la pierre bat les ciseaux ; le code 3 n'est pas un coup : toutes les sorties restent à 0",
    )
    .entree("a", 2)
    .entree("b", 2)
    .sortie("gagne_a", 1)
    .sortie("gagne_b", 1)
    .sortie("egalite", 1);
    let da = p.poser("DECODEUR2", &[("a", "a")]);
    let db = p.poser("DECODEUR2", &[("a", "b")]);
    let coup = |x: &str, v: u32| format!("{x}.s[{v}]");
    // (gagnant, perdant) : feuille sur pierre, ciseaux sur feuille, pierre sur ciseaux.
    let bat = [(1, 0), (2, 1), (0, 2)];
    for (sortie, gagnant, perdant) in [("gagne_a", &da, &db), ("gagne_b", &db, &da)] {
        let termes = bat
            .iter()
            .map(|&(g, q)| p.et(&coup(gagnant, g), &coup(perdant, q)))
            .collect();
        let s = p.arbre("OU", termes);
        p.relier(s, sortie);
    }
    let termes = (0..3).map(|v| p.et(&coup(&da, v), &coup(&db, v))).collect();
    let s = p.arbre("OU", termes);
    p.relier(s, "egalite");
    p.fin()
}

/// Les huit alignements du morpion : trois lignes, trois colonnes, deux diagonales.
pub const ALIGNEMENTS: [[usize; 3]; 8] = [
    [0, 1, 2],
    [3, 4, 5],
    [6, 7, 8],
    [0, 3, 6],
    [1, 4, 7],
    [2, 5, 8],
    [0, 4, 8],
    [2, 4, 6],
];

fn morpion() -> Composant {
    let mut p = Plan::nouveau(
        "morpion : x[i] = 1 quand la case i porte une croix (cases numérotées de 0 à 8, ligne par ligne) ; gagne = 1 quand trois croix sont alignées, en ligne, en colonne ou en diagonale",
    )
    .entree("x", 9)
    .sortie("gagne", 1);
    let lignes = ALIGNEMENTS
        .iter()
        .map(|l| {
            let a = p.et(&format!("x[{}]", l[0]), &format!("x[{}]", l[1]));
            p.et(&a, &format!("x[{}]", l[2]))
        })
        .collect();
    let g = p.arbre("OU", lignes);
    p.relier(g, "gagne");
    p.fin()
}

fn de() -> Composant {
    let mut p = Plan::nouveau(
        "dé électronique : au front montant de clk, raz = 1 met face à 1 ; sinon, tant que lancer = 1, face avance de 1 à 6 puis revient à 1 ; on lâche lancer au hasard et le dé s'arrête sur une face",
    )
    .entree("lancer", 1)
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("face", 3);
    let nr = p.non("raz");
    let un = p.un("raz");
    let (bs, q) = bascules(&mut p, 3);
    let (s, _) = plus_un(&mut p, &q, &un);
    // 6 = 110 : les seules faces avec q1 et q2 à 1 sont 6 (et 7, jamais atteint).
    let six = p.et(&q[1], &q[2]);
    let pas_six = p.non(&six);
    let suivant = [
        p.mux(&s[0], &un, &six),
        p.et(&s[1], &pas_six),
        p.et(&s[2], &pas_six),
    ];
    for i in 0..3 {
        let n = p.mux(&q[i], &suivant[i], "lancer");
        let d = if i == 0 {
            p.ou(&n, "raz")
        } else {
            p.et(&n, &nr)
        };
        p.relier(d, format!("{}.d", bs[i]));
        p.relier(&q[i], format!("face[{i}]"));
    }
    p.fin()
}

/// `a > b` sur 8 bits, avec deux comparateurs de 4 bits.
fn superieur8(p: &mut Plan, a: &[String], b: &[String]) -> String {
    let moitie = |p: &mut Plan, m: usize| {
        let c = p.poser("COMPARATEUR4", &[]);
        for i in 0..4 {
            p.relier(&a[4 * m + i], format!("{c}.a[{i}]"));
            p.relier(&b[4 * m + i], format!("{c}.b[{i}]"));
        }
        c
    };
    let (bas, haut) = (moitie(p, 0), moitie(p, 1));
    let x = p.et(&format!("{haut}.egal"), &format!("{bas}.sup"));
    p.ou(&format!("{haut}.sup"), &x)
}

fn meilleur_score() -> Composant {
    let mut p = Plan::nouveau(
        "meilleur score : nouveau = valider ET (score > record) ; au front montant de clk, raz = 1 remet record à 0, sinon un nouveau record prend la place de l'ancien",
    )
    .entree("score", 8)
    .entree("valider", 1)
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("record", 8)
    .sortie("nouveau", 1);
    let nr = p.non("raz");
    let (bs, q) = bascules(&mut p, 8);
    let plus = superieur8(&mut p, &bits("score", 8), &q);
    let nouveau = p.et(&plus, "valider");
    for i in 0..8 {
        let d = p.mux(&q[i], &format!("score[{i}]"), &nouveau);
        let d = p.et(&d, &nr);
        p.relier(d, format!("{}.d", bs[i]));
        p.relier(&q[i], format!("record[{i}]"));
    }
    p.relier(nouveau, "nouveau");
    p.fin()
}

fn premier() -> Composant {
    let mut p = Plan::nouveau(
        "nombre premier : p = 1 quand n (de 0 à 15) est premier, c'est-à-dire 2, 3, 5, 7, 11 ou 13",
    )
    .entree("n", 4)
    .sortie("p", 1);
    let dec = p.poser("DECODEUR4", &[("e", "n")]);
    let termes = [2, 3, 5, 7, 11, 13]
        .iter()
        .map(|v| format!("{dec}.s[{v}]"))
        .collect();
    let s = p.arbre("OU", termes);
    p.relier(s, "p");
    p.fin()
}

fn carre() -> Composant {
    let mut p = Plan::nouveau("carré : c = n × n")
        .entree("n", 4)
        .sortie("c", 8);
    let m = p.poser("MULT4", &[("a", "n"), ("b", "n")]);
    p.relier(format!("{m}.p"), "c");
    p.fin()
}

/// Ajoute 3 à un chiffre qui vaut au moins 5 (le pas du « double dabble »). `None` : un bit
/// qui vaut sûrement 0.
fn ajoute_3(p: &mut Plan, x: &[Option<String>], zero: &str) -> Vec<Option<String>> {
    if x.iter().all(Option::is_none) {
        return x.to_vec();
    }
    let b: Vec<String> = x
        .iter()
        .map(|v| v.clone().unwrap_or_else(|| zero.to_owned()))
        .collect();
    // x ≥ 5 : x3, ou x2 avec x1 ou x0.
    let o = p.ou(&b[1], &b[0]);
    let a = p.et(&b[2], &o);
    let ge5 = p.ou(&b[3], &a);
    let add = p.poser("ADD4", &[("re", zero)]);
    for (i, v) in b.iter().enumerate() {
        p.relier(v, format!("{add}.a[{i}]"));
    }
    for (i, v) in [&ge5, &ge5, zero, zero].into_iter().enumerate() {
        p.relier(v, format!("{add}.b[{i}]"));
    }
    (0..4).map(|i| Some(format!("{add}.s[{i}]"))).collect()
}

fn binaire_dcb() -> Composant {
    let mut p = Plan::nouveau(
        "binaire vers décimal : n (de 0 à 255) = 100 c + 10 d + u, chaque chiffre sur 4 bits ; méthode « double dabble » : on décale n bit après bit dans les chiffres, en ajoutant 3 à tout chiffre qui vaut au moins 5 avant chaque décalage",
    )
    .entree("n", 8)
    .sortie("c", 4)
    .sortie("d", 4)
    .sortie("u", 4);
    let zero = p.zero("n[0]");
    // Les 12 bits des chiffres, u d'abord ; None : sûrement 0.
    let mut dcb: Vec<Option<String>> = vec![None; 12];
    for k in (0..8).rev() {
        let mut ajuste = Vec::new();
        for chiffre in dcb.chunks(4) {
            ajuste.extend(ajoute_3(&mut p, chiffre, &zero));
        }
        // Décalage d'un cran vers le haut, le bit k de n entre en bas.
        dcb = std::iter::once(Some(format!("n[{k}]")))
            .chain(ajuste.into_iter().take(11))
            .collect();
    }
    for (i, nom) in ["u", "d", "c"].iter().enumerate() {
        for b in 0..4 {
            let v = dcb[4 * i + b].clone().unwrap_or_else(|| zero.clone());
            p.relier(v, format!("{nom}[{b}]"));
        }
    }
    p.fin()
}

fn fibonacci() -> Composant {
    let mut p = Plan::nouveau(
        "suite de Fibonacci : au front montant de clk, raz = 1 met a à 0 et b à 1 ; sinon a prend b et b prend a + b (modulo 256) ; f montre a : 0, 1, 1, 2, 3, 5, 8, 13…",
    )
    .entree("raz", 1)
    .entree("clk", 1)
    .sortie("f", 8);
    let nr = p.non("raz");
    let zero = p.zero("raz");
    let (ba, qa) = bascules(&mut p, 8);
    let (bb, qb) = bascules(&mut p, 8);
    let add = p.poser("ADD8", &[("re", &zero)]);
    for i in 0..8 {
        p.relier(&qa[i], format!("{add}.a[{i}]"));
        p.relier(&qb[i], format!("{add}.b[{i}]"));
        let da = p.et(&qb[i], &nr);
        p.relier(da, format!("{}.d", ba[i]));
        let db = if i == 0 {
            p.ou(&format!("{add}.s[0]"), "raz")
        } else {
            p.et(&format!("{add}.s[{i}]"), &nr)
        };
        p.relier(db, format!("{}.d", bb[i]));
        p.relier(&qa[i], format!("f[{i}]"));
    }
    p.fin()
}

/// Ajoute les classiques à une bibliothèque qui contient déjà la bibliothèque de référence et
/// les composants de l'ordinateur.
pub fn ajouter(bib: &mut Bibliotheque) {
    for (nom, c) in [
        ("EQUIV", equiv()),
        ("MAJORITE", majorite()),
        ("DEMUX", demux()),
        ("MUX4", mux4()),
        ("DECODEUR2", decodeur2()),
        ("DECODEUR3", decodeur3()),
        ("ENCODEUR4", encodeur4()),
        ("EGAL4", egal4()),
        ("ADD4", add4()),
        ("ADD8_RAPIDE", add8_rapide()),
        ("COMPARATEUR4", comparateur4()),
        ("SOUSTRACTEUR4", soustracteur4()),
        ("NEGATIF8", negatif8()),
        ("PARITE8", parite8()),
        ("DECALEUR8", decaleur8()),
        ("MULT4", mult4()),
        ("DIVISEUR4", diviseur4()),
        ("SEPT_SEGMENTS", sept_segments()),
        ("BASCULE_T", bascule_t()),
        ("BASCULE_JK", bascule_jk()),
        ("DECALAGE8", decalage8()),
        ("COMPTEUR_BCD", compteur_bcd()),
        ("COMPTEUR_HB4", compteur_hb4()),
        ("LFSR8", lfsr8()),
        ("MINUTERIE", minuterie()),
        ("FEU", feu()),
        ("CHRONOMETRE", chronometre()),
        ("BANC_REGISTRES", banc_registres()),
        ("PILE", pile()),
        ("FILE", file()),
        ("EMETTEUR_SERIE", emetteur_serie()),
        ("INTERRUPTIONS", interruptions()),
        ("DETECTEUR_FRONT", detecteur_front()),
        ("CADENCE", cadence()),
        ("CONTROLE_PARITE", controle_parite()),
        ("RECEPTEUR_SERIE", recepteur_serie()),
        ("LIAISON_SERIE", liaison_serie()),
        ("CARTE_RESEAU", carte_reseau()),
        ("COMPTEUR_CASCADE", compteur_cascade()),
        ("COMPTEUR_60", compteur_60()),
        ("COMPTEUR_24", compteur_24()),
        ("REVEIL", reveil()),
        ("MONTRE", montre()),
        ("BALAYAGE", balayage()),
        ("POLICE", police()),
        ("MEMOIRE_VIDEO", memoire_video()),
        ("SERIALISEUR", serialiseur()),
        ("CONTROLEUR_ECRAN", controleur_ecran()),
        ("GRAY", gray()),
        ("MOTIF_101", motif_101()),
        ("DIVISIBLE_3", divisible_3()),
        ("CELLULE_VIE", cellule_vie()),
        ("AUTOMATE_30", automate_30()),
        ("PGCD", pgcd()),
        ("CESAR", cesar()),
        ("MASQUE_XOR", masque_xor()),
        ("SOMME_CONTROLE", somme_controle()),
        ("HAMMING_CODE", hamming_code()),
        ("HAMMING_CORRECTEUR", hamming_correcteur()),
        ("ARBITRE", arbitre()),
        ("MORPION", morpion()),
        ("DE", de()),
        ("MEILLEUR_SCORE", meilleur_score()),
        ("PREMIER", premier()),
        ("CARRE", carre()),
        ("BINAIRE_DCB", binaire_dcb()),
        ("FIBONACCI", fibonacci()),
    ] {
        bib.ajouter(nom, c);
    }
}

/// Tous les corrigés : la bibliothèque de référence, les composants de l'ordinateur (avec une
/// mémoire vive de 4 octets, `RAM4`, de 16, `RAM16`, et jusqu'à 256, `RAM256`) et les classiques.
pub fn bibliotheque() -> Bibliotheque {
    let reglages = Reglages {
        bits_ram: 4,
        ..Reglages::default()
    };
    let mut bib =
        ordinateur::bibliotheque(&[], reglages).expect("l'ordinateur sans programme se construit");
    ajouter(&mut bib);
    // Les grandes mémoires des grands projets : RAM32, RAM64, RAM128 et RAM256.
    ordinateur::materiel::ram(&mut bib, 8);
    bib.valider().expect("les classiques sont bien formés");
    bib
}
