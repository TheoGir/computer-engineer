//! `ce` : outil en ligne de commande du projet Computer Engineer.

mod mesure;

use ce_circuit::analyse::{chemin_critique, pilotes_en_conflit, table_de_verite};
use ce_circuit::ordinateur::{self, programmes, Machine, Reglages};
use ce_circuit::{reference, Bibliotheque, Circuit, Sens};
use std::process::ExitCode;
use std::time::Instant;

const AIDE: &str = "\
ce — simulateur de circuits NAND (Computer Engineer)

Usage :
  ce reference                        écrit la bibliothèque de référence en JSON
  ce valider <fichier.json>           vérifie tous les composants d'une bibliothèque
  ce infos <fichier.json> <composant> portes, nets, chemin critique, avertissements
  ce table <fichier.json> <composant> table de vérité exhaustive (circuit combinatoire)
  ce mesure [options] [noms…]         mesure la vitesse du moteur sur cette machine
                                      (--duree <s>, --json <fichier>, --tout pour les
                                      très gros circuits, --sans-sommeil et
                                      --sans-rapide, --sans-compilation pour comparer ;
                                      sans nom : la série courante)

L'ordinateur miniature en NAND :
  ce ordinateur demo                  exécute les programmes d'exemple, porte par porte
  ce ordinateur executer <prog.asm> [touches]
                                      assemble et exécute un programme ; « touches » est
                                      le texte tapé au clavier, une touche après l'autre
  ce ordinateur bibliotheque <prog.asm>
                                      écrit en JSON tous les composants de la machine
  ce ordinateur exemple <nom>         écrit un programme d'exemple (fibonacci,
                                      calculatrice, majuscules)
";

fn lire(chemin: &str) -> Result<Bibliotheque, String> {
    let texte = std::fs::read_to_string(chemin).map_err(|e| format!("{chemin} : {e}"))?;
    Bibliotheque::depuis_json(&texte).map_err(|e| format!("{chemin} : {e}"))
}

fn valider(chemin: &str) -> Result<(), String> {
    let bib = lire(chemin)?;
    let mut erreurs = 0;
    for nom in bib.composants.keys() {
        match Circuit::elaborer(&bib, nom) {
            Ok(c) => {
                let n = c.nb_portes();
                println!(
                    "  ok      {nom} ({n} porte{})",
                    if n > 1 { "s" } else { "" }
                );
                for a in &c.avertissements {
                    println!("          attention, {a}");
                }
            }
            Err(e) => {
                erreurs += 1;
                println!("  ERREUR  {nom} : {e}");
            }
        }
    }
    if erreurs == 0 {
        println!("{} composants valides.", bib.composants.len());
        Ok(())
    } else {
        Err(format!("{erreurs} composant(s) en erreur"))
    }
}

fn infos(chemin: &str, nom: &str) -> Result<(), String> {
    let bib = lire(chemin)?;
    let c = Circuit::elaborer(&bib, nom).map_err(|e| e.to_string())?;
    println!("Composant « {nom} »");
    for b in &c.broches {
        let sens = match b.sens {
            Sens::Entree => "entrée",
            Sens::Sortie => "sortie",
        };
        println!("  {sens} {} ({} bit(s))", b.nom, b.nets.len());
    }
    println!("  portes primitives : {}", c.nb_portes());
    println!("  nets              : {}", c.netlist.nb_nets);
    match chemin_critique(&c.netlist) {
        Some(d) => println!("  chemin critique   : {d} unité(s) de temps (circuit sans boucle)"),
        None => println!("  chemin critique   : sans objet, le circuit contient une boucle (mémoire ou oscillateur)"),
    }
    let conflits = pilotes_en_conflit(&c.netlist);
    if !conflits.is_empty() {
        println!("  attention, {} net(s) pilotés par plusieurs sorties permanentes : court-circuit possible", conflits.len());
    }
    for a in &c.avertissements {
        println!("  attention, {a}");
    }
    Ok(())
}

fn table(chemin: &str, nom: &str) -> Result<(), String> {
    let bib = lire(chemin)?;
    let t = table_de_verite(&bib, nom).map_err(|e| e.to_string())?;
    let titres = |liste: &[(String, u32)]| {
        liste
            .iter()
            .map(|(n, l)| {
                if *l == 1 {
                    n.clone()
                } else {
                    format!("{n}[{l}]")
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    println!("{} | {}", titres(&t.entrees), titres(&t.sorties));
    let nb_bits: u32 = t.entrees.iter().map(|(_, l)| l).sum();
    for (i, ligne) in t.lignes.iter().enumerate() {
        // Chaque broche est écrite poids fort à gauche.
        let mut texte = String::new();
        let mut bit = 0;
        for (_, largeur) in &t.entrees {
            for k in (0..*largeur).rev() {
                texte.push(if (i >> (bit + k)) & 1 == 1 { '1' } else { '0' });
            }
            texte.push(' ');
            bit += largeur;
        }
        texte.push_str("| ");
        let mut rang = 0usize;
        for (_, largeur) in &t.sorties {
            for k in (0..*largeur as usize).rev() {
                texte.push(ligne[rang + k].symbole());
            }
            texte.push(' ');
            rang += *largeur as usize;
        }
        println!("{}", texte.trim_end());
    }
    println!(
        "{} lignes ({} bit(s) d'entrée), chemin critique {} unité(s) de temps.",
        t.lignes.len(),
        nb_bits,
        t.chemin_critique
    );
    Ok(())
}

/// Nombre de cycles pendant lesquels une touche reste enfoncée, puis relâchée.
const DUREE_TOUCHE: usize = 12;

fn touche_au_cycle(touches: &[u8], cycle: usize) -> u8 {
    let (rang, phase) = (cycle / (2 * DUREE_TOUCHE), cycle % (2 * DUREE_TOUCHE));
    if phase < DUREE_TOUCHE {
        touches.get(rang).copied().unwrap_or(0)
    } else {
        0
    }
}

fn texte_ecran(ecran: &[u8]) -> String {
    ecran
        .iter()
        .map(|&c| {
            if c == b' ' || c.is_ascii_graphic() {
                (c as char).to_string()
            } else {
                format!("<{c}>")
            }
        })
        .collect()
}

/// Exécute un programme sur l'ordinateur en NAND. Renvoie l'écran et la machine.
fn faire_tourner(
    source: &str,
    touches: &str,
    max_cycles: usize,
) -> Result<(Vec<u8>, Machine, f64), String> {
    let programme = ordinateur::assembler(source).map_err(|e| e.to_string())?;
    let mut machine =
        Machine::nouvelle(&programme, Reglages::default()).map_err(|e| e.to_string())?;
    let touches = touches.as_bytes();
    let debut = Instant::now();
    let ecran = machine.executer(max_cycles, |i| touche_au_cycle(touches, i));
    Ok((ecran, machine, debut.elapsed().as_secs_f64()))
}

fn ordinateur_executer(source: &str, touches: &str) -> Result<(), String> {
    let (ecran, machine, ecoule) = faire_tourner(source, touches, 200_000)?;
    println!("Écran : {}", texte_ecran(&ecran));
    let fin = match machine.arret() {
        Some(true) => "arrêtée sur HLT",
        Some(false) => "encore en marche",
        None => "dans un état indéterminé (lecture d'une case de mémoire jamais écrite ?)",
    };
    let stats = machine.banc.sim.statistiques();
    println!(
        "{} cycles d'horloge, machine {fin}. {} portes NAND, {} évaluations de portes, {:.0} cycles par seconde.",
        machine.cycles,
        machine.nb_portes(),
        stats.evaluations,
        machine.cycles as f64 / ecoule.max(1e-9)
    );
    Ok(())
}

fn ordinateur_demo() -> Result<(), String> {
    for (nom, source, touches) in [
        ("bonjour", programmes::afficher("BONJOUR LUNE"), ""),
        ("fibonacci", programmes::FIBONACCI.to_string(), ""),
        ("calculatrice", programmes::CALCULATRICE.to_string(), "67"),
        ("majuscules", programmes::MAJUSCULES.to_string(), "nand"),
    ] {
        println!(
            "--- {nom}{}",
            if touches.is_empty() {
                String::new()
            } else {
                format!(" (touches : {touches})")
            }
        );
        let (ecran, machine, _) = faire_tourner(&source, touches, 400)?;
        if nom == "fibonacci" {
            let termes: Vec<String> = ecran.iter().map(u8::to_string).collect();
            println!("Écran (octets) : {}", termes.join(" "));
        } else {
            println!("Écran : {}", texte_ecran(&ecran));
        }
        println!(
            "{} cycles, {} portes NAND.",
            machine.cycles,
            machine.nb_portes()
        );
    }
    Ok(())
}

fn lire_source(chemin: &str) -> Result<String, String> {
    std::fs::read_to_string(chemin).map_err(|e| format!("{chemin} : {e}"))
}

fn executer(args: &[String]) -> Result<(), String> {
    match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["reference"] => {
            println!("{}", reference::bibliotheque().vers_json());
            Ok(())
        }
        ["valider", fichier] => valider(fichier),
        ["infos", fichier, nom] => infos(fichier, nom),
        ["table", fichier, nom] => table(fichier, nom),
        ["mesure", ref suite @ ..] => mesure::executer(suite),
        ["ordinateur", "demo"] => ordinateur_demo(),
        ["ordinateur", "executer", fichier] => ordinateur_executer(&lire_source(fichier)?, ""),
        ["ordinateur", "executer", fichier, touches] => {
            ordinateur_executer(&lire_source(fichier)?, touches)
        }
        ["ordinateur", "bibliotheque", fichier] => {
            let programme =
                ordinateur::assembler(&lire_source(fichier)?).map_err(|e| e.to_string())?;
            let bib = ordinateur::bibliotheque(&programme, Reglages::default())
                .map_err(|e| e.to_string())?;
            println!("{}", bib.vers_json());
            Ok(())
        }
        ["ordinateur", "exemple", nom] => {
            let source = match nom {
                "fibonacci" => programmes::FIBONACCI,
                "calculatrice" => programmes::CALCULATRICE,
                "majuscules" => programmes::MAJUSCULES,
                _ => {
                    return Err(format!(
                        "exemple inconnu « {nom} » (fibonacci, calculatrice, majuscules)"
                    ))
                }
            };
            print!("{source}");
            Ok(())
        }
        [] | ["aide"] | ["--help"] | ["-h"] => {
            print!("{AIDE}");
            Ok(())
        }
        _ => Err(format!("commande inconnue.\n\n{AIDE}")),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match executer(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("Erreur : {message}");
            ExitCode::FAILURE
        }
    }
}
