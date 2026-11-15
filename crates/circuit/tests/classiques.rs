//! Chaque classique est comparé à sa définition mathématique : exhaustivement pour les
//! circuits combinatoires (toutes les combinaisons d'entrées), et pour les circuits à mémoire
//! contre un modèle écrit en Rust, sur de longues suites d'actions tirées au hasard, une
//! entrée changée à la fois.

use std::collections::BTreeMap;

use ce_circuit::analyse::{chemin_critique, table_de_verite};
use ce_circuit::classiques::{bibliotheque, ALIGNEMENTS, POLICE, SEGMENTS};
use ce_circuit::{Banc, Bibliotheque, Circuit};
use ce_moteur::alea::Alea;
use ce_moteur::Valeur;

fn bib() -> &'static Bibliotheque {
    static BIB: std::sync::OnceLock<Bibliotheque> = std::sync::OnceLock::new();
    BIB.get_or_init(bibliotheque)
}

/// Vérifie la table de vérité de `nom` contre `f(entrées) = sorties`, les deux codés en
/// entiers (première broche d'abord, bit 0 d'abord). Renvoie le nombre de lignes.
fn verifier_table(nom: &str, f: impl Fn(u64) -> u64) -> usize {
    let table = table_de_verite(bib(), nom).unwrap();
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
    table.lignes.len()
}

fn champ(e: u64, debut: u32, largeur: u32) -> u64 {
    (e >> debut) & ((1 << largeur) - 1)
}

#[test]
fn portes_et_aiguillages() {
    assert_eq!(verifier_table("EQUIV", |e| !(e ^ (e >> 1)) & 1), 4);
    assert_eq!(
        verifier_table("MAJORITE", |e| u64::from(e.count_ones() >= 2)),
        8
    );
    // DEMUX : e (bit 0), sel (bit 1) → s0, s1.
    assert_eq!(
        verifier_table("DEMUX", |e| if e & 2 == 0 { e & 1 } else { (e & 1) << 1 }),
        4
    );
    assert_eq!(
        verifier_table("MUX4", |e| champ(e, 0, 4) >> champ(e, 4, 2) & 1),
        64
    );
    assert_eq!(verifier_table("DECODEUR2", |e| 1 << e), 4);
    assert_eq!(verifier_table("DECODEUR3", |e| 1 << e), 8);
    assert_eq!(
        verifier_table("ENCODEUR4", |d| if d == 0 {
            0
        } else {
            u64::from(63 - d.leading_zeros()) | 1 << 2
        }),
        16
    );
}

#[test]
fn comparer_et_calculer_exhaustivement() {
    let (a, b) = (|e| champ(e, 0, 4), |e| champ(e, 4, 4));
    assert_eq!(verifier_table("EGAL4", |e| u64::from(a(e) == b(e))), 256);
    assert_eq!(
        verifier_table("COMPARATEUR4", |e| u64::from(a(e) < b(e))
            | u64::from(a(e) == b(e)) << 1
            | u64::from(a(e) > b(e)) << 2),
        256
    );
    assert_eq!(
        verifier_table("ADD4", |e| a(e) + b(e) + champ(e, 8, 1)),
        512
    );
    assert_eq!(
        verifier_table("SOUSTRACTEUR4", |e| (a(e).wrapping_sub(b(e)) & 15)
            | u64::from(a(e) < b(e)) << 4),
        256
    );
    assert_eq!(verifier_table("MULT4", |e| a(e) * b(e)), 256);
    assert_eq!(
        verifier_table("DIVISEUR4", |e| match b(e) {
            0 => 15 | a(e) << 4,
            d => (a(e) / d) | ((a(e) % d) << 4),
        }),
        256
    );
    assert_eq!(verifier_table("NEGATIF8", |e| (256 - e) & 255), 256);
    assert_eq!(
        verifier_table("PARITE8", |e| u64::from(e.count_ones() % 2)),
        256
    );
    assert_eq!(
        verifier_table("DECALEUR8", |e| {
            let (x, n, droite) = (champ(e, 0, 8), champ(e, 8, 3), champ(e, 11, 1));
            if droite == 1 {
                x >> n
            } else {
                (x << n) & 255
            }
        }),
        4096
    );
    assert_eq!(
        verifier_table("SEPT_SEGMENTS", |n| u64::from(SEGMENTS[n as usize])),
        16
    );
}

/// Les 131 072 additions de l'additionneur rapide, et il répond plus vite que la chaîne.
#[test]
fn la_retenue_selectionnee_est_juste_et_plus_rapide() {
    assert_eq!(
        verifier_table("ADD8_RAPIDE", |e| champ(e, 0, 8)
            + champ(e, 8, 8)
            + champ(e, 16, 1)),
        1 << 17
    );
    let critique = |nom: &str| chemin_critique(&Circuit::elaborer(bib(), nom).unwrap().netlist);
    let (chaine, rapide) = (critique("ADD8").unwrap(), critique("ADD8_RAPIDE").unwrap());
    assert!(rapide < chaine, "{rapide} contre {chaine}");
}

/// Tous les classiques sont bien formés, sans fil en l'air ni sortie inutilisée signalée.
#[test]
fn les_classiques_se_mettent_a_plat_sans_avertissement() {
    for nom in [
        "EQUIV",
        "MAJORITE",
        "DEMUX",
        "MUX4",
        "DECODEUR2",
        "DECODEUR3",
        "ENCODEUR4",
        "EGAL4",
        "ADD4",
        "ADD8_RAPIDE",
        "COMPARATEUR4",
        "SOUSTRACTEUR4",
        "NEGATIF8",
        "PARITE8",
        "DECALEUR8",
        "MULT4",
        "DIVISEUR4",
        "SEPT_SEGMENTS",
        "BASCULE_T",
        "BASCULE_JK",
        "DECALAGE8",
        "COMPTEUR_BCD",
        "COMPTEUR_HB4",
        "LFSR8",
        "MINUTERIE",
        "FEU",
        "CHRONOMETRE",
        "BANC_REGISTRES",
        "PILE",
        "FILE",
        "EMETTEUR_SERIE",
        "INTERRUPTIONS",
        "DETECTEUR_FRONT",
        "CADENCE",
        "CONTROLE_PARITE",
        "RECEPTEUR_SERIE",
        "LIAISON_SERIE",
        "CARTE_RESEAU",
        "COMPTEUR_CASCADE",
        "COMPTEUR_60",
        "COMPTEUR_24",
        "REVEIL",
        "MONTRE",
        "BALAYAGE",
        "POLICE",
        "MEMOIRE_VIDEO",
        "SERIALISEUR",
        "CONTROLEUR_ECRAN",
        "GRAY",
        "MOTIF_101",
        "DIVISIBLE_3",
        "CELLULE_VIE",
        "AUTOMATE_30",
        "PGCD",
        "CESAR",
        "MASQUE_XOR",
        "SOMME_CONTROLE",
        "HAMMING_CODE",
        "HAMMING_CORRECTEUR",
        "ARBITRE",
        "MORPION",
        "DE",
        "MEILLEUR_SCORE",
        "PREMIER",
        "CARRE",
        "BINAIRE_DCB",
        "FIBONACCI",
    ] {
        let c = Circuit::elaborer(bib(), nom).unwrap();
        assert!(c.nb_portes() > 0, "{nom}");
    }
}

type Valeurs = BTreeMap<&'static str, u64>;

/// Le modèle mathématique d'un circuit à mémoire.
trait Modele {
    /// Les entrées, avec leur largeur. L'une d'elles s'appelle `clk`.
    fn entrees(&self) -> Vec<(&'static str, u32)>;
    /// Ce qui se passe au front montant de `clk`, avec les entrées du moment.
    fn front(&mut self, e: &Valeurs);
    /// Ce que valent les sorties ; `None` pour une valeur que le modèle ne connaît pas encore
    /// (une mémoire jamais écrite) : le circuit n'est alors pas jugé sur elle.
    fn sorties(&self, e: &Valeurs) -> Vec<(&'static str, Option<u64>)>;
}

/// Joue `n` actions tirées au hasard, une entrée à la fois, et compare le circuit au modèle
/// après chacune. Renvoie le nombre de comparaisons faites sur des valeurs connues.
fn jouer(nom: &str, modele: &mut dyn Modele, n: usize, graine: u64) -> usize {
    let mut banc = Banc::nouveau(bib(), nom).unwrap();
    let entrees = modele.entrees();
    let mut e: Valeurs = entrees.iter().map(|(n, _)| (*n, 0)).collect();
    for (nom, _) in &entrees {
        banc.ecrire(nom, 0).unwrap();
    }
    // Mise sous tension : tout à 0 le temps que le circuit se pose.
    banc.stabiliser(100_000);
    let mut alea = Alea::nouveau(graine);
    let mut jugees = 0;
    for pas in 0..n {
        // L'horloge bouge une fois sur deux : beaucoup de fronts.
        let (choisie, largeur) = if alea.chance(1, 2) {
            ("clk", 1)
        } else {
            entrees[alea.sous(entrees.len() as u64) as usize]
        };
        let ancienne = e[choisie];
        let nouvelle = if largeur == 1 {
            ancienne ^ 1
        } else {
            alea.sous(1 << largeur)
        };
        e.insert(choisie, nouvelle);
        banc.ecrire(choisie, nouvelle).unwrap();
        assert!(banc.stabiliser(100_000).est_stable(), "{nom} oscille");
        if choisie == "clk" && ancienne == 0 && nouvelle == 1 {
            modele.front(&e);
        }
        for (sortie, voulu) in modele.sorties(&e) {
            if let Some(v) = voulu {
                assert_eq!(
                    banc.lire(sortie).unwrap(),
                    Some(v),
                    "{nom}, action {pas} : {sortie} pour {e:?}"
                );
                jugees += 1;
            }
        }
    }
    jugees
}

fn jouer_bien<M: Modele>(nom: &str, neuf: impl Fn() -> M) {
    let mut jugees = 0;
    for graine in 1..=6 {
        jugees += jouer(nom, &mut neuf(), 1500, graine);
    }
    assert!(jugees > 3000, "{nom} : seulement {jugees} valeurs jugées");
}

#[derive(Default)]
struct BasculeT(Option<u64>);
impl Modele for BasculeT {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![("t", 1), ("raz", 1), ("clk", 1)]
    }
    fn front(&mut self, e: &Valeurs) {
        self.0 = if e["raz"] == 1 {
            Some(0)
        } else {
            self.0.map(|q| q ^ e["t"])
        };
    }
    fn sorties(&self, _: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        vec![("q", self.0)]
    }
}

#[derive(Default)]
struct BasculeJK(Option<u64>);
impl Modele for BasculeJK {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![("j", 1), ("k", 1), ("raz", 1), ("clk", 1)]
    }
    fn front(&mut self, e: &Valeurs) {
        self.0 = if e["raz"] == 1 {
            Some(0)
        } else {
            self.0.map(|q| match (e["j"], e["k"]) {
                (0, 0) => q,
                (1, 0) => 1,
                (0, 1) => 0,
                _ => q ^ 1,
            })
        };
    }
    fn sorties(&self, _: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        vec![("q", self.0)]
    }
}

/// Le registre à décalage : chaque bit est connu dès qu'il a été poussé.
#[derive(Default)]
struct Decalage(Vec<u64>);
impl Modele for Decalage {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![("e", 1), ("clk", 1)]
    }
    fn front(&mut self, e: &Valeurs) {
        self.0.insert(0, e["e"]);
        self.0.truncate(8);
    }
    fn sorties(&self, _: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        let connu = self.0.len() == 8;
        let v = self.0.iter().enumerate().map(|(i, b)| b << i).sum();
        vec![("q", connu.then_some(v))]
    }
}

/// Un compteur : `suivant(q, entrées)`, avec `raz` qui le remet à `depart`.
/// Les sorties d'un compteur, tirées de son état.
type SortiesDuCompteur = fn(Option<u64>) -> Vec<(&'static str, Option<u64>)>;

struct Compteur {
    q: Option<u64>,
    entrees: Vec<(&'static str, u32)>,
    raz: &'static str,
    depart: u64,
    suivant: fn(u64, &Valeurs) -> u64,
    sorties: SortiesDuCompteur,
}
impl Modele for Compteur {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        self.entrees.clone()
    }
    fn front(&mut self, e: &Valeurs) {
        self.q = if e[self.raz] == 1 {
            Some(self.depart)
        } else {
            self.q.map(|q| (self.suivant)(q, e))
        };
    }
    fn sorties(&self, _: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        (self.sorties)(self.q)
    }
}

#[test]
fn bascules_t_et_jk() {
    jouer_bien("BASCULE_T", BasculeT::default);
    jouer_bien("BASCULE_JK", BasculeJK::default);
}

#[test]
fn registre_a_decalage() {
    jouer_bien("DECALAGE8", Decalage::default);
}

#[test]
fn compteurs_et_generateur() {
    jouer_bien("COMPTEUR_BCD", || Compteur {
        q: None,
        entrees: vec![("raz", 1), ("clk", 1)],
        raz: "raz",
        depart: 0,
        suivant: |q, _| if q == 9 { 0 } else { q + 1 },
        sorties: |q| vec![("q", q), ("fin", q.map(|q| u64::from(q == 9)))],
    });
    jouer_bien("COMPTEUR_HB4", || Compteur {
        q: None,
        entrees: vec![("raz", 1), ("haut", 1), ("clk", 1)],
        raz: "raz",
        depart: 0,
        suivant: |q, e| {
            if e["haut"] == 1 {
                (q + 1) % 16
            } else {
                (q + 15) % 16
            }
        },
        sorties: |q| vec![("q", q)],
    });
    jouer_bien("LFSR8", || Compteur {
        q: None,
        entrees: vec![("raz", 1), ("clk", 1)],
        raz: "raz",
        depart: 1,
        suivant: |q, _| {
            let retour = (q >> 7 ^ q >> 5 ^ q >> 4 ^ q >> 3) & 1;
            (q << 1 | retour) & 255
        },
        sorties: |q| vec![("q", q)],
    });
    jouer_bien("FEU", || Compteur {
        q: None,
        entrees: vec![("raz", 1), ("clk", 1)],
        raz: "raz",
        depart: 0,
        suivant: |q, _| (q + 1) % 8,
        sorties: |q| {
            vec![
                ("vert", q.map(|q| u64::from(q < 3))),
                ("orange", q.map(|q| u64::from(q == 3))),
                ("rouge", q.map(|q| u64::from(q >= 4))),
            ]
        },
    });
    // Le chronomètre : q = 10 × dizaines + unités, de 0 à 59.
    jouer_bien("CHRONOMETRE", || Compteur {
        q: None,
        entrees: vec![("raz", 1), ("clk", 1)],
        raz: "raz",
        depart: 0,
        suivant: |q, _| (q + 1) % 60,
        sorties: |q| vec![("u", q.map(|q| q % 10)), ("d", q.map(|q| q / 10))],
    });
}

/// Le générateur pseudo-aléatoire parcourt les 255 valeurs non nulles avant de revenir.
#[test]
fn le_generateur_a_une_periode_maximale() {
    let mut q = 1u64;
    let mut vus = std::collections::BTreeSet::new();
    while vus.insert(q) {
        let retour = (q >> 7 ^ q >> 5 ^ q >> 4 ^ q >> 3) & 1;
        q = (q << 1 | retour) & 255;
    }
    assert_eq!(vus.len(), 255);
    assert!(!vus.contains(&0));
}

#[derive(Default)]
struct Minuterie(Option<u64>);
impl Modele for Minuterie {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![("charger", 1), ("n", 8), ("clk", 1)]
    }
    fn front(&mut self, e: &Valeurs) {
        self.0 = if e["charger"] == 1 {
            Some(e["n"])
        } else {
            self.0.map(|q| q.saturating_sub(1))
        };
    }
    fn sorties(&self, _: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        vec![("q", self.0), ("fini", self.0.map(|q| u64::from(q == 0)))]
    }
}

#[test]
fn minuterie() {
    jouer_bien("MINUTERIE", Minuterie::default);
}

#[derive(Default)]
struct BancRegistres([Option<u64>; 4]);
impl Modele for BancRegistres {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![
            ("ecrire", 1),
            ("ad_e", 2),
            ("d", 8),
            ("ad_a", 2),
            ("ad_b", 2),
            ("clk", 1),
        ]
    }
    fn front(&mut self, e: &Valeurs) {
        if e["ecrire"] == 1 {
            self.0[e["ad_e"] as usize] = Some(e["d"]);
        }
    }
    fn sorties(&self, e: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        vec![
            ("a", self.0[e["ad_a"] as usize]),
            ("b", self.0[e["ad_b"] as usize]),
        ]
    }
}

#[test]
fn banc_de_registres() {
    jouer_bien("BANC_REGISTRES", BancRegistres::default);
}

/// La pile : `niveaux[0]` est le sommet. `nombre` est inconnu jusqu'à la première remise à zéro.
#[derive(Default)]
struct Pile {
    niveaux: [Option<u64>; 4],
    nombre: Option<usize>,
}
impl Modele for Pile {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![
            ("d", 8),
            ("empiler", 1),
            ("depiler", 1),
            ("raz", 1),
            ("clk", 1),
        ]
    }
    fn front(&mut self, e: &Valeurs) {
        let pousse = e["empiler"] == 1 && e["depiler"] == 0;
        let tire = e["depiler"] == 1 && e["empiler"] == 0;
        if pousse {
            self.niveaux.rotate_right(1);
            self.niveaux[0] = Some(e["d"]);
        } else if tire {
            self.niveaux.rotate_left(1);
            self.niveaux[3] = Some(0);
        }
        self.nombre = if e["raz"] == 1 {
            Some(0)
        } else {
            self.nombre.map(|n| {
                if pousse {
                    (n + 1).min(4)
                } else if tire {
                    n.saturating_sub(1)
                } else {
                    n
                }
            })
        };
    }
    fn sorties(&self, _: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        vec![
            ("sommet", self.niveaux[0]),
            ("vide", self.nombre.map(|n| u64::from(n == 0))),
            ("plein", self.nombre.map(|n| u64::from(n == 4))),
        ]
    }
}

#[test]
fn pile() {
    jouer_bien("PILE", Pile::default);
}

/// La file : `cases[0]` est la plus ancienne. Tant que le nombre est inconnu, on ne sait pas
/// si une lecture ou une écriture a lieu : tout devient inconnu.
#[derive(Default)]
struct File {
    cases: [Option<u64>; 4],
    nombre: Option<usize>,
}
impl Modele for File {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![("d", 8), ("ecrire", 1), ("lire", 1), ("raz", 1), ("clk", 1)]
    }
    fn front(&mut self, e: &Valeurs) {
        let veut_ecrire = e["ecrire"] == 1 && e["lire"] == 0;
        let veut_lire = e["lire"] == 1 && e["ecrire"] == 0;
        match self.nombre {
            Some(n) => {
                if veut_ecrire && n < 4 {
                    self.cases[n] = Some(e["d"]);
                    self.nombre = Some(n + 1);
                } else if veut_lire && n > 0 {
                    self.cases.rotate_left(1);
                    self.cases[3] = self.cases[2];
                    self.nombre = Some(n - 1);
                }
            }
            None if veut_ecrire || veut_lire => self.cases = [None; 4],
            None => {}
        }
        if e["raz"] == 1 {
            self.nombre = Some(0);
        }
    }
    fn sorties(&self, _: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        vec![
            ("sortie", self.cases[0]),
            ("vide", self.nombre.map(|n| u64::from(n == 0))),
            ("plein", self.nombre.map(|n| u64::from(n == 4))),
        ]
    }
}

#[test]
fn file() {
    jouer_bien("FILE", File::default);
}

/// L'émetteur série : la trame restant à envoyer, bit de départ en tête.
#[derive(Default)]
struct Emetteur {
    trame: Vec<u64>,
    connu: bool,
}
impl Modele for Emetteur {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![("d", 8), ("envoyer", 1), ("raz", 1), ("clk", 1)]
    }
    fn front(&mut self, e: &Valeurs) {
        if e["raz"] == 1 {
            self.trame.clear();
            self.connu = true;
        } else if self.connu {
            if self.trame.is_empty() {
                if e["envoyer"] == 1 {
                    self.trame = std::iter::once(0)
                        .chain((0..8).map(|k| e["d"] >> k & 1))
                        .chain(std::iter::once(1))
                        .collect();
                }
            } else {
                self.trame.remove(0);
            }
        }
    }
    fn sorties(&self, _: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        if !self.connu {
            return vec![("tx", None), ("occupe", None)];
        }
        vec![
            ("tx", Some(self.trame.first().copied().unwrap_or(1))),
            ("occupe", Some(u64::from(!self.trame.is_empty()))),
        ]
    }
}

#[test]
fn emetteur_serie() {
    jouer_bien("EMETTEUR_SERIE", Emetteur::default);
}

/// Une trame envoyée de bout en bout : 0, les huit bits de « A » (0x41), 1, puis le repos.
#[test]
fn l_emetteur_envoie_une_trame_complete() {
    let mut banc = Banc::nouveau(bib(), "EMETTEUR_SERIE").unwrap();
    for broche in ["d", "envoyer", "raz", "clk"] {
        banc.ecrire(broche, 0).unwrap();
    }
    let front = |banc: &mut Banc| {
        banc.stabiliser(10_000);
        banc.ecrire("clk", 1).unwrap();
        banc.stabiliser(10_000);
        banc.ecrire("clk", 0).unwrap();
        banc.stabiliser(10_000);
    };
    banc.ecrire("raz", 1).unwrap();
    front(&mut banc);
    banc.ecrire("raz", 0).unwrap();
    banc.ecrire("d", 0x41).unwrap();
    banc.ecrire("envoyer", 1).unwrap();
    front(&mut banc);
    banc.ecrire("envoyer", 0).unwrap();
    let mut vu = Vec::new();
    for _ in 0..12 {
        banc.stabiliser(10_000);
        vu.push(banc.lire("tx").unwrap().unwrap());
        front(&mut banc);
    }
    assert_eq!(vu, [0, 1, 0, 0, 0, 0, 0, 1, 0, 1, 1, 1]);
}

/// Les demandes en attente ; inconnues avant la première remise à zéro.
#[derive(Default)]
struct Interruptions(Option<[bool; 4]>);
impl Modele for Interruptions {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![("irq", 4), ("ack", 1), ("raz", 1), ("clk", 1)]
    }
    fn front(&mut self, e: &Valeurs) {
        if e["raz"] == 1 {
            self.0 = Some([false; 4]);
            return;
        }
        if let Some(att) = &mut self.0 {
            let num = (0..4).rev().find(|&i| att[i]);
            if e["ack"] == 1 {
                // Sans demande, num vaut 0 : acquitter 0 n'efface rien qui soit en attente.
                att[num.unwrap_or(0)] = false;
            }
            for (i, a) in att.iter_mut().enumerate() {
                *a |= e["irq"] >> i & 1 == 1;
            }
        }
    }
    fn sorties(&self, _: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        let Some(att) = self.0 else {
            return vec![("int", None), ("num", None)];
        };
        let num = (0..4).rev().find(|&i| att[i]);
        vec![
            ("int", Some(u64::from(num.is_some()))),
            ("num", Some(num.unwrap_or(0) as u64)),
        ]
    }
}

#[test]
fn controleur_d_interruptions() {
    jouer_bien("INTERRUPTIONS", Interruptions::default);
}

// ---------------------------------------------------------------------------------------
// La branche « Le réseau ».

/// L'ancienne valeur de e, gardée au dernier front.
#[derive(Default)]
struct Front(Option<u64>);
impl Modele for Front {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![("e", 1), ("clk", 1)]
    }
    fn front(&mut self, e: &Valeurs) {
        self.0 = Some(e["e"]);
    }
    fn sorties(&self, e: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        let f = match (e["e"], self.0) {
            (0, _) => Some(0),
            (_, Some(a)) => Some(1 - a),
            (_, None) => None,
        };
        vec![("front", f)]
    }
}

#[test]
fn detecteur_de_front() {
    jouer_bien("DETECTEUR_FRONT", Front::default);
}

#[test]
fn cadence() {
    jouer_bien("CADENCE", || Compteur {
        q: None,
        entrees: vec![("raz", 1), ("clk", 1)],
        raz: "raz",
        depart: 0,
        suivant: |q, _| (q + 1) % 8,
        sorties: |q| vec![("q", q), ("tic", q.map(|q| u64::from(q == 7)))],
    });
}

#[test]
fn controle_de_parite() {
    let n = verifier_table("CONTROLE_PARITE", |e| {
        u64::from((e & 0x1FF).count_ones() % 2 == 0)
    });
    assert_eq!(n, 512);
}

/// Le récepteur : `reste` bits encore attendus (0 au repos), l'octet en cours, le dernier
/// octet reçu, et les deux drapeaux d'un front.
#[derive(Default, Clone)]
struct Recepteur {
    connu: bool,
    reste: u32,
    bits: Vec<u64>,
    d: u64,
    pret: u64,
    erreur: u64,
}
impl Recepteur {
    fn pas(&mut self, rx: u64, raz: u64) {
        if raz == 1 {
            *self = Recepteur {
                connu: true,
                ..Recepteur::default()
            };
            return;
        }
        if !self.connu {
            return;
        }
        self.pret = 0;
        self.erreur = 0;
        match self.reste {
            0 => {
                if rx == 0 {
                    self.reste = 9;
                    self.bits.clear();
                }
            }
            1 => {
                if rx == 1 {
                    self.d = self.bits.iter().enumerate().map(|(i, b)| b << i).sum();
                    self.pret = 1;
                } else {
                    self.erreur = 1;
                }
                self.reste = 0;
            }
            _ => {
                self.bits.push(rx);
                self.reste -= 1;
            }
        }
    }
    fn sorties(&self) -> Vec<(&'static str, Option<u64>)> {
        let c = |v| self.connu.then_some(v);
        vec![
            ("d", c(self.d)),
            ("pret", c(self.pret)),
            ("erreur", c(self.erreur)),
        ]
    }
}
impl Modele for Recepteur {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![("rx", 1), ("raz", 1), ("clk", 1)]
    }
    fn front(&mut self, e: &Valeurs) {
        self.pas(e["rx"], e["raz"]);
    }
    fn sorties(&self, _: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        Recepteur::sorties(self)
    }
}

#[test]
fn recepteur_serie() {
    jouer_bien("RECEPTEUR_SERIE", Recepteur::default);
}

/// Le fil entre les deux : le récepteur lit ce que l'émetteur montrait avant le front.
#[derive(Default)]
struct Liaison {
    em: Emetteur,
    re: Recepteur,
}
impl Modele for Liaison {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![("d", 8), ("envoyer", 1), ("raz", 1), ("clk", 1)]
    }
    fn front(&mut self, e: &Valeurs) {
        let tx = self.em.sorties(e)[0].1;
        match tx {
            Some(tx) => self.re.pas(tx, e["raz"]),
            None if e["raz"] == 1 => self.re.pas(1, 1),
            None => {}
        }
        self.em.front(e);
    }
    fn sorties(&self, e: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        let r = self.re.sorties();
        vec![
            ("recu", r[0].1),
            ("pret", r[1].1),
            ("occupe", self.em.sorties(e)[1].1),
        ]
    }
}

#[test]
fn liaison_serie() {
    jouer_bien("LIAISON_SERIE", Liaison::default);
}

/// La carte réseau : l'émetteur, le récepteur et la file, chacun avec son modèle.
#[derive(Default)]
struct Carte {
    em: Emetteur,
    re: Recepteur,
    file: File,
}
impl Modele for Carte {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![
            ("d", 8),
            ("envoyer", 1),
            ("rx", 1),
            ("lire", 1),
            ("raz", 1),
            ("clk", 1),
        ]
    }
    fn front(&mut self, e: &Valeurs) {
        // La file voit l'octet et le drapeau du récepteur d'avant le front.
        let mut ef: Valeurs = Valeurs::new();
        ef.insert("d", self.re.d);
        ef.insert("ecrire", self.re.pret);
        ef.insert("lire", e["lire"]);
        ef.insert("raz", e["raz"]);
        if self.re.connu || e["raz"] == 1 {
            self.file.front(&ef);
        } else {
            self.file = File::default();
        }
        self.re.pas(e["rx"], e["raz"]);
        self.em.front(e);
    }
    fn sorties(&self, e: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        let em = self.em.sorties(e);
        let f = self.file.sorties(e);
        let get = |n: &str| f.iter().find(|(k, _)| *k == n).and_then(|(_, v)| *v);
        vec![
            ("tx", em[0].1),
            ("occupe", em[1].1),
            ("recu", get("sortie")),
            ("dispo", get("vide").map(|v| 1 - v)),
            ("plein", get("plein")),
            ("erreur", self.re.sorties()[2].1),
        ]
    }
}

#[test]
fn carte_reseau() {
    jouer_bien("CARTE_RESEAU", Carte::default);
}

/// Un octet traverse la liaison de bout en bout : « N » (0x4E) arrive au onzième front.
#[test]
fn la_liaison_transporte_un_octet() {
    let mut banc = Banc::nouveau(bib(), "LIAISON_SERIE").unwrap();
    for broche in ["d", "envoyer", "raz", "clk"] {
        banc.ecrire(broche, 0).unwrap();
    }
    let front = |banc: &mut Banc| {
        banc.stabiliser(10_000);
        banc.ecrire("clk", 1).unwrap();
        banc.stabiliser(10_000);
        banc.ecrire("clk", 0).unwrap();
        banc.stabiliser(10_000);
    };
    banc.ecrire("raz", 1).unwrap();
    front(&mut banc);
    banc.ecrire("raz", 0).unwrap();
    banc.ecrire("d", 0x4E).unwrap();
    banc.ecrire("envoyer", 1).unwrap();
    front(&mut banc);
    banc.ecrire("envoyer", 0).unwrap();
    banc.ecrire("d", 0).unwrap();
    let mut pret_au = None;
    for n in 1..=12 {
        front(&mut banc);
        if banc.lire("pret").unwrap() == Some(1) {
            pret_au = Some(n);
            assert_eq!(banc.lire("recu").unwrap(), Some(0x4E));
        }
    }
    assert_eq!(pret_au, Some(10));
}

// ---------------------------------------------------------------------------------------
// La branche « La montre numérique ».

/// Un nombre de 0 à 99 écrit en décimal codé binaire : dizaines en haut, unités en bas.
fn dcb(n: u64) -> u64 {
    ((n / 10) << 4) | (n % 10)
}

/// Un compteur de 0 à `modulo − 1` qui avance quand `en` vaut 1, lu en décimal codé binaire.
struct Modulo {
    q: Option<u64>,
    modulo: u64,
    dcb: bool,
}
impl Modulo {
    fn neuf(modulo: u64, dcb: bool) -> Self {
        Modulo {
            q: None,
            modulo,
            dcb,
        }
    }
    fn retenue(&self, en: u64) -> Option<u64> {
        if en == 0 {
            return Some(0);
        }
        self.q.map(|q| u64::from(q == self.modulo - 1))
    }
    fn pas(&mut self, en: u64, raz: u64) {
        self.q = if raz == 1 {
            Some(0)
        } else if en == 1 {
            self.q.map(|q| (q + 1) % self.modulo)
        } else {
            self.q
        };
    }
    fn lu(&self) -> Option<u64> {
        self.q.map(|q| if self.dcb { dcb(q) } else { q })
    }
}
impl Modele for Modulo {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![("en", 1), ("raz", 1), ("clk", 1)]
    }
    fn front(&mut self, e: &Valeurs) {
        self.pas(e["en"], e["raz"]);
    }
    fn sorties(&self, e: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        vec![("q", self.lu()), ("retenue", self.retenue(e["en"]))]
    }
}

#[test]
fn compteurs_de_la_montre() {
    jouer_bien("COMPTEUR_CASCADE", || Modulo::neuf(10, false));
    jouer_bien("COMPTEUR_60", || Modulo::neuf(60, true));
    jouer_bien("COMPTEUR_24", || Modulo::neuf(24, true));
}

/// Les compteurs de la montre parcourent tout leur cycle : 60 et 24 pas ramènent à 00.
#[test]
fn le_compteur_des_heures_fait_le_tour_du_cadran() {
    let mut banc = Banc::nouveau(bib(), "COMPTEUR_24").unwrap();
    for broche in ["en", "raz", "clk"] {
        banc.ecrire(broche, 0).unwrap();
    }
    let front = |banc: &mut Banc| {
        banc.stabiliser(10_000);
        banc.ecrire("clk", 1).unwrap();
        banc.stabiliser(10_000);
        banc.ecrire("clk", 0).unwrap();
        banc.stabiliser(10_000);
    };
    banc.ecrire("raz", 1).unwrap();
    front(&mut banc);
    banc.ecrire("raz", 0).unwrap();
    banc.ecrire("en", 1).unwrap();
    let mut vu = Vec::new();
    for _ in 0..25 {
        vu.push(banc.lire("q").unwrap().unwrap());
        front(&mut banc);
    }
    let attendu: Vec<u64> = (0..25).map(|h| dcb(h % 24)).collect();
    assert_eq!(vu, attendu);
}

#[derive(Default)]
struct Reveil {
    h: Option<u64>,
    m: Option<u64>,
    arme: Option<u64>,
    sonne: Option<u64>,
}
impl Modele for Reveil {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![
            ("h", 8),
            ("m", 8),
            ("regler", 1),
            ("arreter", 1),
            ("raz", 1),
            ("clk", 1),
        ]
    }
    fn front(&mut self, e: &Valeurs) {
        let egal = match (self.h, self.m) {
            (Some(h), Some(m)) => Some(h == e["h"] && m == e["m"]),
            _ => None,
        };
        let atteint = match (self.arme, egal) {
            (Some(0), _) => Some(false),
            (Some(_), Some(g)) => Some(g),
            _ => None,
        };
        self.sonne = if e["raz"] == 1 || e["arreter"] == 1 {
            Some(0)
        } else {
            match (self.sonne, atteint) {
                (Some(1), _) => Some(1),
                (_, Some(true)) => Some(1),
                (Some(0), Some(false)) => Some(0),
                _ => None,
            }
        };
        self.arme = if e["raz"] == 1 {
            Some(0)
        } else if e["regler"] == 1 {
            Some(1)
        } else {
            self.arme
        };
        if e["regler"] == 1 {
            self.h = Some(e["h"]);
            self.m = Some(e["m"]);
        }
    }
    fn sorties(&self, _: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        vec![("sonne", self.sonne)]
    }
}

#[test]
fn reveil() {
    jouer_bien("REVEIL", Reveil::default);
}

/// La montre : secondes, minutes et heures.
struct Montre {
    s: Modulo,
    m: Modulo,
    h: Modulo,
}
impl Default for Montre {
    fn default() -> Self {
        Montre {
            s: Modulo::neuf(60, true),
            m: Modulo::neuf(60, true),
            h: Modulo::neuf(24, true),
        }
    }
}
impl Modele for Montre {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![
            ("en", 1),
            ("plus_m", 1),
            ("plus_h", 1),
            ("raz", 1),
            ("clk", 1),
        ]
    }
    fn front(&mut self, e: &Valeurs) {
        let ou = |a: Option<u64>, b: u64| if b == 1 { Some(1) } else { a };
        let em = ou(self.s.retenue(e["en"]), e["plus_m"]);
        let eh = em.and_then(|em| ou(self.m.retenue(em), e["plus_h"]));
        let eh = if e["plus_h"] == 1 { Some(1) } else { eh };
        let raz = e["raz"];
        self.s.pas(e["en"], raz);
        match em {
            Some(em) => self.m.pas(em, raz),
            None if raz == 1 => self.m.pas(0, 1),
            None => self.m.q = None,
        }
        match eh {
            Some(eh) => self.h.pas(eh, raz),
            None if raz == 1 => self.h.pas(0, 1),
            None => self.h.q = None,
        }
    }
    fn sorties(&self, _: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        vec![("s", self.s.lu()), ("m", self.m.lu()), ("h", self.h.lu())]
    }
}

#[test]
fn montre() {
    jouer_bien("MONTRE", Montre::default);
}

// ---------------------------------------------------------------------------------------
// La branche « Le contrôleur d'écran ».

/// Le balayage : la position (x, y).
#[derive(Default)]
struct Balayage(Option<(u64, u64)>);
impl Balayage {
    fn pas(&mut self, raz: u64) {
        self.0 = if raz == 1 {
            Some((0, 0))
        } else {
            self.0
                .map(|(x, y)| if x == 7 { (0, (y + 1) % 8) } else { (x + 1, y) })
        };
    }
}
impl Modele for Balayage {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![("raz", 1), ("clk", 1)]
    }
    fn front(&mut self, e: &Valeurs) {
        self.pas(e["raz"]);
    }
    fn sorties(&self, _: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        let p = self.0;
        vec![
            ("x", p.map(|p| p.0)),
            ("y", p.map(|p| p.1)),
            ("fin_ligne", p.map(|p| u64::from(p.0 == 7))),
            ("fin_image", p.map(|p| u64::from(p == (7, 7)))),
        ]
    }
}

#[test]
fn balayage() {
    jouer_bien("BALAYAGE", Balayage::default);
}

#[test]
fn police_de_caracteres() {
    let n = verifier_table("POLICE", |e| {
        let (chiffre, ligne) = (champ(e, 0, 4) as usize, champ(e, 4, 3) as usize);
        if ligne < 5 {
            u64::from(POLICE[chiffre][ligne])
        } else {
            0
        }
    });
    assert_eq!(n, 128);
    // Le 8 est le seul chiffre plein sur ses lignes 0, 2 et 4 avec des côtés aux lignes 1 et 3.
    assert_eq!(POLICE[8], [7, 5, 7, 5, 7]);
}

/// La mémoire vidéo : huit lignes, inconnues tant qu'elles n'ont pas été écrites.
#[derive(Default)]
struct Video([Option<u64>; 8]);
impl Video {
    fn ecrire(&mut self, e: &Valeurs) {
        if e["ecrire"] == 1 {
            self.0[e["ligne_e"] as usize] = Some(e["d"]);
        }
    }
    fn pixel(&self, x: u64, y: u64) -> Option<u64> {
        self.0[y as usize].map(|l| l >> x & 1)
    }
}
impl Modele for Video {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![
            ("ligne_e", 3),
            ("d", 8),
            ("ecrire", 1),
            ("x", 3),
            ("y", 3),
            ("clk", 1),
        ]
    }
    fn front(&mut self, e: &Valeurs) {
        self.ecrire(e);
    }
    fn sorties(&self, e: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        vec![
            ("pixel", self.pixel(e["x"], e["y"])),
            ("ligne", self.0[e["y"] as usize]),
        ]
    }
}

#[test]
fn memoire_video() {
    jouer_bien("MEMOIRE_VIDEO", Video::default);
}

/// Le sérialiseur : les bits connus qui restent, bit 0 en tête.
#[derive(Default)]
struct Serialiseur([Option<u64>; 8]);
impl Modele for Serialiseur {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![("d", 8), ("charger", 1), ("clk", 1)]
    }
    fn front(&mut self, e: &Valeurs) {
        if e["charger"] == 1 {
            for (i, bit) in self.0.iter_mut().enumerate() {
                *bit = Some(e["d"] >> i & 1);
            }
        } else {
            self.0.rotate_left(1);
            self.0[7] = Some(0);
        }
    }
    fn sorties(&self, _: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        vec![("pixel", self.0[0])]
    }
}

#[test]
fn serialiseur() {
    jouer_bien("SERIALISEUR", Serialiseur::default);
}

#[derive(Default)]
struct Controleur {
    balayage: Balayage,
    video: Video,
}
impl Modele for Controleur {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![
            ("ligne_e", 3),
            ("d", 8),
            ("ecrire", 1),
            ("raz", 1),
            ("clk", 1),
        ]
    }
    fn front(&mut self, e: &Valeurs) {
        self.video.ecrire(e);
        self.balayage.pas(e["raz"]);
    }
    fn sorties(&self, e: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        let b = self.balayage.sorties(e);
        let pixel = self.balayage.0.and_then(|(x, y)| self.video.pixel(x, y));
        vec![
            ("x", b[0].1),
            ("y", b[1].1),
            ("pixel", pixel),
            ("fin_image", b[3].1),
        ]
    }
}

#[test]
fn controleur_d_ecran() {
    jouer_bien("CONTROLEUR_ECRAN", Controleur::default);
}

// ---------------------------------------------------------------------------------------
// La branche « Maths et automates ».

#[test]
fn code_de_gray() {
    verifier_table("GRAY", |b| b ^ (b >> 1));
    // Deux nombres qui se suivent ne diffèrent que d'un bit, 15 et 0 compris.
    for b in 0..16u64 {
        let (g, h) = (b ^ (b >> 1), ((b + 1) % 16) ^ (((b + 1) % 16) >> 1));
        assert_eq!((g ^ h).count_ones(), 1);
    }
}

/// Les trois dernières valeurs lues, la plus récente d'abord ; inconnues avant raz.
#[derive(Default)]
struct Motif(Option<Vec<u64>>);
impl Modele for Motif {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![("e", 1), ("raz", 1), ("clk", 1)]
    }
    fn front(&mut self, e: &Valeurs) {
        if e["raz"] == 1 {
            self.0 = Some(vec![0, 0, 0]);
        } else if let Some(h) = &mut self.0 {
            h.insert(0, e["e"]);
            h.truncate(3);
        }
    }
    fn sorties(&self, _: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        vec![("vu", self.0.as_ref().map(|h| u64::from(h[..] == [1, 0, 1])))]
    }
}

#[test]
fn reconnaisseur_de_motif() {
    jouer_bien("MOTIF_101", Motif::default);
}

/// Le nombre lu jusque-là, modulo 3.
#[derive(Default)]
struct Divisible(Option<u64>);
impl Modele for Divisible {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![("e", 1), ("raz", 1), ("clk", 1)]
    }
    fn front(&mut self, e: &Valeurs) {
        self.0 = if e["raz"] == 1 {
            Some(0)
        } else {
            self.0.map(|r| (2 * r + e["e"]) % 3)
        };
    }
    fn sorties(&self, _: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        vec![("div", self.0.map(|r| u64::from(r == 0)))]
    }
}

#[test]
fn divisible_par_3() {
    jouer_bien("DIVISIBLE_3", Divisible::default);
}

#[test]
fn cellule_du_jeu_de_la_vie() {
    let n = verifier_table("CELLULE_VIE", |e| {
        let (v, voisins) = (e & 1, (e >> 1).count_ones());
        u64::from(voisins == 3 || (v == 1 && voisins == 2))
    });
    assert_eq!(n, 512);
}

#[derive(Default)]
struct Regle30(Option<u64>);
impl Modele for Regle30 {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![("d", 8), ("charger", 1), ("clk", 1)]
    }
    fn front(&mut self, e: &Valeurs) {
        self.0 = if e["charger"] == 1 {
            Some(e["d"])
        } else {
            self.0.map(|q| {
                // La voisine de gauche de la cellule i est i + 1, celle de droite i − 1.
                let g = (q >> 1 | (q & 1) << 7) & 0xFF;
                let d = (q << 1 | q >> 7) & 0xFF;
                g ^ (q | d)
            })
        };
    }
    fn sorties(&self, _: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        vec![("q", self.0)]
    }
}

#[test]
fn automate_regle_30() {
    jouer_bien("AUTOMATE_30", Regle30::default);
}

#[derive(Default)]
struct Pgcd(Option<(u64, u64)>);
impl Modele for Pgcd {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![("a", 4), ("b", 4), ("charger", 1), ("clk", 1)]
    }
    fn front(&mut self, e: &Valeurs) {
        self.0 = if e["charger"] == 1 {
            Some((e["a"], e["b"]))
        } else {
            self.0.map(|(x, y)| {
                if x > y {
                    (x - y, y)
                } else if y > x {
                    (x, y - x)
                } else {
                    (x, y)
                }
            })
        };
    }
    fn sorties(&self, _: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        let p = self.0;
        vec![
            ("x", p.map(|p| p.0)),
            ("y", p.map(|p| p.1)),
            (
                "fini",
                p.map(|(x, y)| u64::from(x == y || x == 0 || y == 0)),
            ),
        ]
    }
}

#[test]
fn pgcd_d_euclide() {
    jouer_bien("PGCD", Pgcd::default);
}

/// Euclide trouve bien le PGCD de tous les couples de 1 à 15.
#[test]
fn euclide_trouve_le_pgcd_de_tous_les_couples() {
    fn pgcd(a: u64, b: u64) -> u64 {
        if b == 0 {
            a
        } else {
            pgcd(b, a % b)
        }
    }
    let mut banc = Banc::nouveau(bib(), "PGCD").unwrap();
    for broche in ["a", "b", "charger", "clk"] {
        banc.ecrire(broche, 0).unwrap();
    }
    let front = |banc: &mut Banc| {
        banc.stabiliser(10_000);
        banc.ecrire("clk", 1).unwrap();
        banc.stabiliser(10_000);
        banc.ecrire("clk", 0).unwrap();
        banc.stabiliser(10_000);
    };
    for a in 1..16 {
        for b in 1..16 {
            banc.ecrire("a", a).unwrap();
            banc.ecrire("b", b).unwrap();
            banc.ecrire("charger", 1).unwrap();
            front(&mut banc);
            banc.ecrire("charger", 0).unwrap();
            let mut pas = 0;
            while banc.lire("fini").unwrap() != Some(1) {
                front(&mut banc);
                pas += 1;
                assert!(pas <= 15, "{a}, {b} : trop long");
            }
            assert_eq!(banc.lire("x").unwrap(), Some(pgcd(a, b)), "{a}, {b}");
        }
    }
}

/// Le code de Hamming, bit par bit : d0 à d3 aux places 3, 5, 6, 7, parités aux places 1, 2, 4.
fn hamming(d: u64) -> u64 {
    let b = |i: u32| d >> i & 1;
    let p1 = b(0) ^ b(1) ^ b(3);
    let p2 = b(0) ^ b(2) ^ b(3);
    let p4 = b(1) ^ b(2) ^ b(3);
    p1 | p2 << 1 | b(0) << 2 | p4 << 3 | b(1) << 4 | b(2) << 5 | b(3) << 6
}

#[test]
fn codes_secrets_exhaustivement() {
    let (d, k) = (|e| champ(e, 0, 4), |e| champ(e, 4, 4));
    assert_eq!(verifier_table("CESAR", |e| (d(e) + k(e)) % 16), 256);
    // Déchiffrer, c'est avancer de 16 − k : on retombe sur le message.
    for m in 0..16 {
        for cle in 0..16 {
            assert_eq!((((m + cle) % 16) + (16 - cle)) % 16, m);
        }
    }
    assert_eq!(
        verifier_table("MASQUE_XOR", |e| champ(e, 0, 8) ^ champ(e, 8, 8)),
        1 << 16
    );
    assert_eq!(verifier_table("HAMMING_CODE", hamming), 16);
    // Le correcteur, sur les 128 mots : il relit la place que désignent les trois contrôles.
    assert_eq!(
        verifier_table("HAMMING_CORRECTEUR", |h| {
            let syndrome = (1..=7u64)
                .filter(|place| h >> (place - 1) & 1 == 1)
                .fold(0, |s, place| s ^ place);
            let juste = if syndrome == 0 {
                h
            } else {
                h ^ 1 << (syndrome - 1)
            };
            let b = |place: u64| juste >> (place - 1) & 1;
            b(3) | b(5) << 1 | b(6) << 2 | b(7) << 3 | u64::from(syndrome != 0) << 4
        }),
        128
    );
}

/// La promesse du code de Hamming : un bit retourné, n'importe lequel, est corrigé.
#[test]
fn hamming_corrige_toute_erreur_d_un_bit() {
    let mut code = Banc::nouveau(bib(), "HAMMING_CODE").unwrap();
    let mut correcteur = Banc::nouveau(bib(), "HAMMING_CORRECTEUR").unwrap();
    for d in 0..16 {
        code.ecrire("d", d).unwrap();
        code.stabiliser(10_000);
        let h = code.lire("h").unwrap().unwrap();
        assert_eq!(h, hamming(d));
        for faute in std::iter::once(None).chain((0..7).map(Some)) {
            let recu = faute.map_or(h, |i| h ^ 1 << i);
            correcteur.ecrire("h", recu).unwrap();
            correcteur.stabiliser(10_000);
            assert_eq!(
                correcteur.lire("d").unwrap().unwrap(),
                d,
                "d = {d}, faute {faute:?}"
            );
            assert_eq!(
                correcteur.lire("erreur").unwrap().unwrap(),
                u64::from(faute.is_some())
            );
        }
    }
}

#[derive(Default)]
struct SommeControle(Option<u64>);
impl Modele for SommeControle {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![("d", 8), ("ajouter", 1), ("raz", 1), ("clk", 1)]
    }
    fn front(&mut self, e: &Valeurs) {
        self.0 = if e["raz"] == 1 {
            Some(0)
        } else if e["ajouter"] == 1 {
            self.0.map(|s| (s + e["d"]) % 256)
        } else {
            self.0
        };
    }
    fn sorties(&self, _: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        vec![("somme", self.0)]
    }
}

#[test]
fn somme_de_controle() {
    jouer_bien("SOMME_CONTROLE", SommeControle::default);
}

#[test]
fn jeux_exhaustivement() {
    // 0 pierre, 1 feuille, 2 ciseaux ; bat(x, y) : x l'emporte sur y.
    let bat = |x: u64, y: u64| (x, y) == (1, 0) || (x, y) == (2, 1) || (x, y) == (0, 2);
    assert_eq!(
        verifier_table("ARBITRE", |e| {
            let (a, b) = (champ(e, 0, 2), champ(e, 2, 2));
            if a == 3 || b == 3 {
                0
            } else {
                u64::from(bat(a, b)) | u64::from(bat(b, a)) << 1 | u64::from(a == b) << 2
            }
        }),
        16
    );
    assert_eq!(
        verifier_table("MORPION", |x| u64::from(
            ALIGNEMENTS
                .iter()
                .any(|l| l.iter().all(|&i| x >> i & 1 == 1))
        )),
        512
    );
    // Une case de chaque alignement et rien d'autre : jamais gagnant ; trois en ligne : gagnant.
    assert_eq!(ALIGNEMENTS.len(), 8);
}

#[derive(Default)]
struct De(Option<u64>);
impl Modele for De {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![("lancer", 1), ("raz", 1), ("clk", 1)]
    }
    fn front(&mut self, e: &Valeurs) {
        self.0 = if e["raz"] == 1 {
            Some(1)
        } else if e["lancer"] == 1 {
            self.0.map(|f| f % 6 + 1)
        } else {
            self.0
        };
    }
    fn sorties(&self, _: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        vec![("face", self.0)]
    }
}

#[test]
fn de_electronique() {
    jouer_bien("DE", De::default);
}

#[derive(Default)]
struct Record(Option<u64>);
impl Modele for Record {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![("score", 8), ("valider", 1), ("raz", 1), ("clk", 1)]
    }
    fn front(&mut self, e: &Valeurs) {
        self.0 = if e["raz"] == 1 {
            Some(0)
        } else {
            self.0.map(|r| {
                if e["valider"] == 1 {
                    r.max(e["score"])
                } else {
                    r
                }
            })
        };
    }
    fn sorties(&self, e: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        vec![
            ("record", self.0),
            (
                "nouveau",
                self.0
                    .map(|r| u64::from(e["valider"] == 1 && e["score"] > r)),
            ),
        ]
    }
}

#[test]
fn meilleur_score() {
    jouer_bien("MEILLEUR_SCORE", Record::default);
}

#[test]
fn nombres_remarquables_exhaustivement() {
    let premier = |n: u64| n >= 2 && (2..n).all(|d| !n.is_multiple_of(d));
    assert_eq!(verifier_table("PREMIER", |n| u64::from(premier(n))), 16);
    assert_eq!(verifier_table("CARRE", |n| n * n), 16);
    assert_eq!(
        verifier_table("BINAIRE_DCB", |n| (n / 100)
            | (n / 10 % 10) << 4
            | (n % 10) << 8),
        256
    );
}

#[derive(Default)]
struct Fibonacci(Option<(u64, u64)>);
impl Modele for Fibonacci {
    fn entrees(&self) -> Vec<(&'static str, u32)> {
        vec![("raz", 1), ("clk", 1)]
    }
    fn front(&mut self, e: &Valeurs) {
        self.0 = if e["raz"] == 1 {
            Some((0, 1))
        } else {
            self.0.map(|(a, b)| (b, (a + b) % 256))
        };
    }
    fn sorties(&self, _: &Valeurs) -> Vec<(&'static str, Option<u64>)> {
        vec![("f", self.0.map(|p| p.0))]
    }
}

#[test]
fn suite_de_fibonacci() {
    jouer_bien("FIBONACCI", Fibonacci::default);
    // Les premiers termes, front après front.
    let mut banc = Banc::nouveau(bib(), "FIBONACCI").unwrap();
    let front = |banc: &mut Banc| {
        banc.stabiliser(10_000);
        banc.ecrire("clk", 1).unwrap();
        banc.stabiliser(10_000);
        banc.ecrire("clk", 0).unwrap();
        banc.stabiliser(10_000);
    };
    banc.ecrire("clk", 0).unwrap();
    banc.ecrire("raz", 1).unwrap();
    front(&mut banc);
    banc.ecrire("raz", 0).unwrap();
    let mut vus = Vec::new();
    for _ in 0..12 {
        vus.push(banc.lire("f").unwrap().unwrap());
        front(&mut banc);
    }
    assert_eq!(vus, [0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89]);
}

/// Ce que disent les consignes des grandes mémoires : chaque étage multiplie par quatre, et
/// la mémoire de 256 octets dépasse 20 000 portes NAND.
#[test]
fn les_grandes_memoires() {
    let portes = |nom: &str| Circuit::elaborer(bib(), nom).unwrap().nb_portes();
    let (m16, m64, m256) = (portes("RAM16"), portes("RAM64"), portes("RAM256"));
    assert!(m64 > 4 * m16 && m256 > 4 * m64, "{m16} {m64} {m256}");
    assert!(m256 > 20_000, "{m256}");
}
