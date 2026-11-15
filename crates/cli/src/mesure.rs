//! `ce mesure` : mesures de vitesse reproductibles du moteur.
//!
//! Chaque mesure construit un circuit, le fait travailler pendant une durée cible et
//! vérifie au passage que le circuit calcule juste. Les charges sont déterministes
//! (graines fixes) : seule la machine fait varier les chiffres.

use ce_circuit::grands::{self, MemoireEnMarche};
use ce_circuit::ordinateur::{self, Machine, Reglages};
use ce_circuit::{reference, Banc, Composant};
use ce_moteur::alea::Alea;
use ce_moteur::{Netlist, Options, Simulateur, StatistiquesSommeil, Valeur};
use std::time::Instant;

/// Le résultat d'une mesure.
pub struct Resultat {
    /// Nom court, celui qu'on donne à `ce mesure <nom>`.
    pub nom: &'static str,
    /// Ce que la mesure fait travailler.
    pub description: String,
    /// Nombre de portes du circuit.
    pub portes: usize,
    /// Temps de construction du circuit et du simulateur, en secondes.
    pub preparation: f64,
    /// Ce qui est compté.
    pub unite: &'static str,
    /// Nombre d'unités faites pendant la mesure.
    pub quantite: u64,
    /// Durée de la mesure, en secondes.
    pub secondes: f64,
    /// Évaluations de portes faites par le moteur pendant la mesure.
    pub evaluations: u64,
    /// Où en sont les régions du circuit à la fin de la mesure.
    pub sommeil: StatistiquesSommeil,
}

impl Resultat {
    /// Unités par seconde.
    pub fn debit(&self) -> f64 {
        self.quantite as f64 / self.secondes
    }

    fn ligne(&self) -> String {
        format!(
            "{:<18} {:>9} portes  {:>13} {}/s  {:>7.1} M éval/s  {:>9.0} éval/{}  {:>3.0} % endormies  prépa {:>5.0} ms",
            self.nom,
            self.portes,
            milliers(self.debit()),
            self.unite,
            self.evaluations as f64 / self.secondes / 1e6,
            self.evaluations as f64 / self.quantite.max(1) as f64,
            self.unite,
            100.0 * self.sommeil.portes_endormies as f64 / self.portes.max(1) as f64,
            self.preparation * 1e3,
        )
    }

    fn json(&self) -> String {
        format!(
            "{{\"nom\":\"{}\",\"description\":\"{}\",\"portes\":{},\"preparation_s\":{:.6},\"unite\":\"{}\",\"quantite\":{},\"secondes\":{:.6},\"debit_par_s\":{:.3},\"evaluations\":{},\"regions\":{},\"portes_endormies\":{},\"endormissements\":{},\"reveils\":{}}}",
            self.nom,
            self.description.replace('"', "'"),
            self.portes,
            self.preparation,
            self.unite,
            self.quantite,
            self.secondes,
            self.debit(),
            self.evaluations,
            self.sommeil.regions,
            self.sommeil.portes_endormies,
            self.sommeil.endormissements,
            self.sommeil.reveils
        )
    }
}

fn milliers(x: f64) -> String {
    let n = x.round() as u64;
    let chiffres = n.to_string();
    let mut texte = String::new();
    for (i, c) in chiffres.chars().enumerate() {
        if i > 0 && (chiffres.len() - i).is_multiple_of(3) {
            texte.push(' ');
        }
        texte.push(c);
    }
    texte
}

/// Réglages communs à toutes les mesures.
#[derive(Clone, Copy)]
pub struct Reglage {
    /// Durée cible de chaque mesure, en secondes.
    pub duree: f64,
    /// Le simulateur met en sommeil les régions qui répètent le même cycle.
    pub sommeil: bool,
    /// Le simulateur calcule les circuits synchrones période par période.
    pub rapide: bool,
    /// En mode rapide, le simulateur traduit le circuit en code machine.
    pub compilation: bool,
}

impl Reglage {
    fn options(self) -> Options {
        Options {
            sommeil: self.sommeil,
            rapide: self.rapide,
            compilation: self.compilation,
        }
    }

    /// Un banc d'essai pour le composant `nom`, avec les réglages de la mesure.
    fn banc(self, bib: &ce_circuit::Bibliotheque, nom: &str) -> Banc {
        let mut banc = Banc::nouveau(bib, nom).expect("circuit de mesure valide");
        if !self.sommeil || !self.rapide || !self.compilation {
            banc.regler(self.options()).expect("circuit déjà validé");
        }
        banc
    }
}

/// Répète `lot` (qui rend le nombre d'unités faites) jusqu'à atteindre la durée cible.
/// Un premier lot, non compté, met le circuit en régime établi.
fn chronometrer(
    reglage: Reglage,
    evaluations: &mut dyn FnMut() -> u64,
    lot: &mut dyn FnMut() -> u64,
) -> (u64, f64, u64) {
    lot();
    let avant = evaluations();
    let debut = Instant::now();
    let mut quantite = 0;
    loop {
        quantite += lot();
        let ecoule = debut.elapsed().as_secs_f64();
        if ecoule >= reglage.duree {
            return (quantite, ecoule, evaluations() - avant);
        }
    }
}

/// Des anneaux d'inverseurs indépendants : toutes les portes sont actives à chaque
/// instant. Donne le débit brut du noyau.
fn anneaux(reglage: Reglage) -> Resultat {
    let nb_anneaux = 100_000;
    let mut nl = Netlist::nouvelle();
    let marche = nl.net();
    nl.entree(marche);
    for _ in 0..nb_anneaux {
        let (a, b, c) = (nl.net(), nl.net(), nl.net());
        nl.nand(marche, c, a, 1);
        nl.nand(a, a, b, 1);
        nl.nand(b, b, c, 1);
    }
    let debut = Instant::now();
    let sim = std::cell::RefCell::new(
        Simulateur::avec_options(&nl, reglage.options()).expect("netlist valide"),
    );
    let preparation = debut.elapsed().as_secs_f64();
    sim.borrow_mut().fixer_entree(0, Valeur::Zero);
    sim.borrow_mut().avancer(10);
    sim.borrow_mut().fixer_entree(0, Valeur::Un);
    let (quantite, secondes, evaluations) = chronometrer(
        reglage,
        &mut || sim.borrow().statistiques().evaluations,
        &mut || {
            sim.borrow_mut().avancer(30);
            30
        },
    );
    let sommeil = sim.borrow().sommeil();
    Resultat {
        nom: "anneaux",
        description: format!(
            "{nb_anneaux} anneaux de 3 inverseurs, toutes les portes actives à chaque instant"
        ),
        portes: nl.portes.len(),
        preparation,
        unite: "instant",
        quantite,
        secondes,
        evaluations,
        sommeil,
    }
}

/// Un grand additionneur hiérarchique : mise à plat, puis peu de portes actives à la fois.
fn additionneur(reglage: Reglage) -> Resultat {
    let nb_bits = 100_000usize;
    let mut bib = reference::bibliotheque();
    let mut add = Composant::nouveau("Additionneur large")
        .entree("a", nb_bits as u32)
        .entree("b", nb_bits as u32)
        .sortie("s", nb_bits as u32)
        .sortie("rs", 1)
        .inst("zero", "ZERO")
        .fil(&["zero.s", "add0.re"]);
    for i in 0..nb_bits {
        add = add
            .inst(&format!("add{i}"), "ADD_COMPLET")
            .fil(&[format!("a[{i}]"), format!("add{i}.a")])
            .fil(&[format!("b[{i}]"), format!("add{i}.b")])
            .fil(&[format!("add{i}.s"), format!("s[{i}]")]);
        if i > 0 {
            add = add.fil(&[format!("add{}.rs", i - 1), format!("add{i}.re")]);
        }
    }
    bib.ajouter(
        "ADD_LARGE",
        add.fil(&[format!("add{}.rs", nb_bits - 1), "rs".to_string()]),
    );
    let debut = Instant::now();
    let banc = std::cell::RefCell::new(reglage.banc(&bib, "ADD_LARGE"));
    let preparation = debut.elapsed().as_secs_f64();
    let portes = banc.borrow().circuit.nb_portes();
    let premiere = |broche: &str| {
        banc.borrow()
            .circuit
            .broche(broche)
            .and_then(|b| b.premiere_entree)
            .expect("broche d'entrée")
    };
    let (pa, pb) = (premiere("a"), premiere("b"));
    let mut alea = Alea::nouveau(1);
    let (quantite, secondes, evaluations) = chronometrer(
        reglage,
        &mut || banc.borrow().sim.statistiques().evaluations,
        &mut || {
            let mut banc = banc.borrow_mut();
            for _ in 0..500 {
                // Change un bit de a et un bit de b, puis laisse la retenue se propager.
                let (i, j) = (
                    alea.sous(nb_bits as u64) as usize,
                    alea.sous(nb_bits as u64) as usize,
                );
                banc.sim
                    .fixer_entree(pa + i, Valeur::de_bool(alea.chance(1, 2)));
                banc.sim
                    .fixer_entree(pb + j, Valeur::de_bool(alea.chance(1, 2)));
                banc.sim.stabiliser(u64::MAX);
            }
            500
        },
    );
    let sommeil = banc.borrow().sim.sommeil();
    Resultat {
        nom: "additionneur",
        description: format!("additionneur de {nb_bits} bits en {nb_bits} composants imbriqués, un bit de chaque opérande change"),
        portes,
        preparation,
        unite: "changement",
        quantite,
        secondes,
        evaluations,
        sommeil,
    }
}

/// Le compteur de référence cadencé par une horloge.
fn compteur(reglage: Reglage) -> Resultat {
    let mut bib = reference::bibliotheque();
    bib.ajouter(
        "MONTAGE",
        Composant::nouveau("Compteur et son horloge")
            .entree("raz", 1)
            .sortie("q", 4)
            .horloge("h", 40)
            .inst("c", "COMPTEUR4")
            .fil(&["h.s", "c.clk"])
            .fil(&["raz", "c.raz"])
            .fil(&["c.q", "q"]),
    );
    let debut = Instant::now();
    let banc = std::cell::RefCell::new(reglage.banc(&bib, "MONTAGE"));
    let preparation = debut.elapsed().as_secs_f64();
    let portes = banc.borrow().circuit.nb_portes();
    banc.borrow_mut().ecrire("raz", 1).expect("broche raz");
    banc.borrow_mut().avancer(100);
    banc.borrow_mut().ecrire("raz", 0).expect("broche raz");
    let mut total = 0u64;
    let (quantite, secondes, evaluations) = chronometrer(
        reglage,
        &mut || banc.borrow().sim.statistiques().evaluations,
        &mut || {
            banc.borrow_mut().avancer(80 * 50_000);
            total += 50_000;
            assert_eq!(
                banc.borrow().lire("q").ok().flatten(),
                Some(total % 16),
                "le compteur a perdu le compte"
            );
            50_000
        },
    );
    let sommeil = banc.borrow().sim.sommeil();
    Resultat {
        nom: "compteur",
        description: "compteur 4 bits de référence et son horloge".to_string(),
        portes,
        preparation,
        unite: "cycle",
        quantite,
        secondes,
        evaluations,
        sommeil,
    }
}

/// Un programme qui ne s'arrête pas : il compte, range en mémoire, relit et compare.
const PROGRAMME_DE_MESURE: &str =
    "LDI 0\nSTA 0\nboucle: LDA 0\nADDI 1\nSTA 0\nSUB 0\nJZ boucle\nHLT\n";

/// Ce que vaut l'accumulateur de l'ordinateur de mesure après `cycles` cycles : le modèle
/// de référence exécute le même programme.
fn accumulateur_attendu(programme: &[u16], bits_ram: u32, cycles: u64) -> u8 {
    let mut emulateur = ordinateur::Emulateur::nouveau(bits_ram);
    for _ in 0..cycles {
        emulateur.pas(programme, 0);
    }
    emulateur.a
}

fn ordinateur_mesure(
    reglage: Reglage,
    nom: &'static str,
    bits_ram: u32,
    lot_cycles: u64,
) -> Resultat {
    let programme =
        ordinateur::assembler(PROGRAMME_DE_MESURE).expect("programme de mesure correct");
    let reglages = Reglages {
        bits_ram,
        ..Reglages::default()
    };
    let debut = Instant::now();
    let machine = std::cell::RefCell::new(
        Machine::avec_options(&programme, reglages, reglage.options())
            .expect("machine de mesure correcte"),
    );
    // La compilation se ferait d'elle-même en arrière-plan ; elle compte ici dans la
    // préparation, pour que la mesure porte sur le régime établi.
    machine.borrow_mut().banc.sim.compiler();
    let preparation = debut.elapsed().as_secs_f64();
    let portes = machine.borrow().nb_portes();
    let (quantite, secondes, evaluations) = chronometrer(
        reglage,
        &mut || machine.borrow().banc.sim.statistiques().evaluations,
        &mut || {
            let mut m = machine.borrow_mut();
            for _ in 0..lot_cycles {
                m.cycle(0);
            }
            lot_cycles
        },
    );
    let m = machine.borrow();
    assert_eq!(
        m.a(),
        Some(accumulateur_attendu(&programme, bits_ram, m.cycles)),
        "l'ordinateur ne suit plus son modèle"
    );
    let sommeil = m.banc.sim.sommeil();
    Resultat {
        nom,
        description: format!(
            "ordinateur miniature, {} octets de mémoire vive, programme qui compte et range en mémoire",
            1u32 << bits_ram
        ),
        portes,
        preparation,
        unite: "cycle",
        quantite,
        secondes,
        evaluations,
        sommeil,
    }
}

/// Un multiplicateur 32 × 32 bits : chaque multiplication fait onduler des milliers de portes.
fn multiplicateur(reglage: Reglage) -> Resultat {
    let mut bib = grands::bibliotheque();
    let nom = grands::multiplicateur(&mut bib, 32);
    let debut = Instant::now();
    let banc = std::cell::RefCell::new(reglage.banc(&bib, &nom));
    let preparation = debut.elapsed().as_secs_f64();
    let portes = banc.borrow().circuit.nb_portes();
    let mut alea = Alea::nouveau(2);
    let (quantite, secondes, evaluations) = chronometrer(
        reglage,
        &mut || banc.borrow().sim.statistiques().evaluations,
        &mut || {
            let mut banc = banc.borrow_mut();
            for _ in 0..10 {
                let (a, b) = (alea.u64() >> 32, alea.u64() >> 32);
                banc.ecrire("a", a).expect("broche a");
                banc.ecrire("b", b).expect("broche b");
                banc.stabiliser(u64::MAX);
                assert_eq!(banc.lire("p").ok().flatten(), Some(a * b), "{a} × {b}");
            }
            10
        },
    );
    let sommeil = banc.borrow().sim.sommeil();
    Resultat {
        nom: "multiplicateur-32",
        description: "multiplicateur combinatoire 32 × 32 bits, opérandes au hasard".to_string(),
        portes,
        preparation,
        unite: "produit",
        quantite,
        secondes,
        evaluations,
        sommeil,
    }
}

/// Une mémoire vive à bus trois états : une lecture ou une écriture par cycle, à une
/// adresse au hasard, comparée à un tableau.
fn memoire(reglage: Reglage, nom: &'static str, bits: u32, lot_cycles: u64) -> Resultat {
    let debut = Instant::now();
    let mem = std::cell::RefCell::new(
        MemoireEnMarche::avec_options(bits, 64, reglage.options()).expect("mémoire valide"),
    );
    let preparation = debut.elapsed().as_secs_f64();
    let portes = mem.borrow().banc.circuit.nb_portes();
    let mut alea = Alea::nouveau(3);
    let taille = 1u64 << bits;
    let (quantite, secondes, evaluations) = chronometrer(
        reglage,
        &mut || mem.borrow().banc.sim.statistiques().evaluations,
        &mut || {
            let mut m = mem.borrow_mut();
            for _ in 0..lot_cycles {
                let adresse = alea.sous(taille) as usize;
                let lu = m.cycle(adresse, alea.u64() as u8, alea.chance(1, 4));
                assert_eq!(lu, m.modele[adresse], "la mémoire ne suit plus son modèle");
            }
            lot_cycles
        },
    );
    let sommeil = mem.borrow().banc.sim.sommeil();
    Resultat {
        nom,
        description: format!(
            "mémoire vive de {taille} octets à bus trois états, une lecture ou une écriture (1 sur 4) par cycle à une adresse au hasard"
        ),
        portes,
        preparation,
        unite: "cycle",
        quantite,
        secondes,
        evaluations,
        sommeil,
    }
}

/// Soixante-quatre ordinateurs miniatures côte à côte : beaucoup de logique irrégulière
/// active en même temps.
fn ferme(reglage: Reglage) -> Resultat {
    let nombre = 64u32;
    let programme =
        ordinateur::assembler(PROGRAMME_DE_MESURE).expect("programme de mesure correct");
    let reglages = Reglages::default();
    let (bib, nom) = grands::ferme(&programme, reglages, nombre).expect("ferme valide");
    let debut = Instant::now();
    let banc = std::cell::RefCell::new(reglage.banc(&bib, &nom));
    let preparation = debut.elapsed().as_secs_f64();
    let portes = banc.borrow().circuit.nb_portes();
    let p = reglages.demi_periode;
    {
        let mut banc = banc.borrow_mut();
        banc.ecrire("raz", 1).expect("broche raz");
        banc.ecrire("clavier", 0).expect("broche clavier");
        banc.avancer(4 * p + p / 2);
        banc.ecrire("raz", 0).expect("broche raz");
    }
    let mut cycles = 0u64;
    let (quantite, secondes, evaluations) = chronometrer(
        reglage,
        &mut || banc.borrow().sim.statistiques().evaluations,
        &mut || {
            banc.borrow_mut().avancer(2 * p * 10);
            cycles += 10;
            10
        },
    );
    // Tous les ordinateurs en sont au même point que le modèle.
    let mut emulateur = ordinateur::Emulateur::nouveau(reglages.bits_ram);
    for _ in 0..cycles {
        emulateur.pas(&programme, 0);
    }
    let pc = u64::from(emulateur.pc);
    assert_eq!(
        banc.borrow().lire("o63.pc").ok().flatten(),
        Some(pc),
        "le dernier ordinateur de la ferme ne suit plus son modèle"
    );
    assert_eq!(banc.borrow().lire("o0.pc").ok().flatten(), Some(pc));
    let sommeil = banc.borrow().sim.sommeil();
    Resultat {
        nom: "ferme-64",
        description: format!("{nombre} ordinateurs miniatures côte à côte, chacun avec son horloge ; un cycle fait avancer les {nombre}"),
        portes,
        preparation,
        unite: "cycle",
        quantite,
        secondes,
        evaluations,
        sommeil,
    }
}

/// Une mesure disponible : son nom, si elle fait partie de la série courante, et sa fonction.
type Entree = (&'static str, bool, fn(Reglage) -> Resultat);

const MESURES: [Entree; 10] = [
    ("anneaux", true, anneaux),
    ("additionneur", true, additionneur),
    ("compteur", true, compteur),
    ("ordinateur", true, |r| {
        ordinateur_mesure(r, "ordinateur", 4, 1_000)
    }),
    ("ordinateur-256", true, |r| {
        ordinateur_mesure(r, "ordinateur-256", 8, 100)
    }),
    ("multiplicateur-32", true, multiplicateur),
    ("memoire-256", true, |r| memoire(r, "memoire-256", 8, 100)),
    ("memoire-4ko", true, |r| memoire(r, "memoire-4ko", 12, 20)),
    ("ferme-64", true, ferme),
    ("memoire-64ko", false, |r| memoire(r, "memoire-64ko", 16, 5)),
];

/// Exécute `ce mesure [--duree <s>] [--json <fichier>] [--tout] [--sans-sommeil] [--sans-rapide] [--sans-compilation] [noms…]`.
pub fn executer(args: &[&str]) -> Result<(), String> {
    let mut reglage = Reglage {
        duree: 1.0,
        sommeil: true,
        rapide: true,
        compilation: true,
    };
    let mut json = None;
    let mut tout = false;
    let mut noms = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i] {
            "--duree" => {
                i += 1;
                reglage.duree = args
                    .get(i)
                    .and_then(|d| d.parse().ok())
                    .filter(|d: &f64| *d > 0.0)
                    .ok_or("--duree attend un nombre de secondes")?;
            }
            "--json" => {
                i += 1;
                json = Some(*args.get(i).ok_or("--json attend un nom de fichier")?);
            }
            "--tout" => tout = true,
            "--sans-sommeil" => reglage.sommeil = false,
            "--sans-rapide" => reglage.rapide = false,
            "--sans-compilation" => reglage.compilation = false,
            nom if MESURES.iter().any(|(n, _, _)| *n == nom) => noms.push(nom),
            autre => {
                let connus: Vec<&str> = MESURES.iter().map(|(n, _, _)| *n).collect();
                return Err(format!(
                    "mesure inconnue « {autre} ». Mesures : {}",
                    connus.join(", ")
                ));
            }
        }
        i += 1;
    }
    let mut resultats = Vec::new();
    for (nom, courante, mesure) in MESURES {
        let voulue = if noms.is_empty() {
            courante || tout
        } else {
            noms.contains(&nom)
        };
        if voulue {
            let r = mesure(reglage);
            println!("{}", r.ligne());
            resultats.push(r);
        }
    }
    if let Some(chemin) = json {
        let lignes: Vec<String> = resultats
            .iter()
            .map(|r| format!("  {}", r.json()))
            .collect();
        let texte = format!("[\n{}\n]\n", lignes.join(",\n"));
        std::fs::write(chemin, texte).map_err(|e| format!("{chemin} : {e}"))?;
    }
    Ok(())
}
