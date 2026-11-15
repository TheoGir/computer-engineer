//! Comparaison avec un simulateur établi : Icarus Verilog.
//!
//! Des circuits aléatoires (boucles comprises) sont exportés en Verilog structurel,
//! simulés par `iverilog`/`vvp`, et comparés au moteur, net par net et instant par
//! instant.
//!
//! Résultat attendu, démontré dans `docs/semantique.md` (« écarts avec Verilog ») :
//!
//! - à délai unité, les deux sémantiques coïncident : égalité exacte, y compris
//!   avec des entrées `x` et `z` ;
//! - avec des délais variés, le seul écart possible vient de la règle « mixte » du
//!   moteur (un `x` plus court que le délai d'une porte) : tout circuit simulé sans
//!   échéance mixte doit être exactement égal ; sinon le moteur peut dire `x` là où
//!   Verilog annonce une valeur, jamais l'inverse, et jamais deux valeurs
//!   binaires opposées ;
//! - avec des tampons trois états, même relation (Verilog distingue des forces de
//!   signal que le moteur ramène prudemment à `x`).
//!
//! Icarus Verilog 12 plante (erreur de segmentation de `vvp`) sur certaines
//! boucles formées uniquement de tampons trois états, par exemple un tampon dont
//! la sortie est rebouclée sur sa propre validation. Le moteur simule ces cas sans
//! difficulté (ils sont couverts par la comparaison à la spécification). Ici, un
//! circuit sur lequel `vvp` plante est écarté et compté ; le test exige qu'une
//! large majorité des circuits soit effectivement comparée.
//!
//! Sans `iverilog` dans le PATH, les tests sont ignorés avec un message, sauf si
//! la variable `CE_EXIGER_ICARUS=1` est posée (c'est le cas en CI).

use ce_circuit::verilog::banc_de_test;
use ce_moteur::alea::{netlist_aleatoire, stimulus_aleatoire, Alea, ParamsNetlist};
use ce_moteur::{GenrePorte, Netlist, Simulateur, Valeur};
use std::path::PathBuf;
use std::process::Command;

fn icarus_disponible() -> bool {
    let present = Command::new("iverilog").arg("-V").output().is_ok();
    if !present {
        assert!(
            std::env::var("CE_EXIGER_ICARUS").as_deref() != Ok("1"),
            "iverilog est exigé (CE_EXIGER_ICARUS=1) mais introuvable"
        );
        eprintln!("iverilog introuvable : comparaison ignorée");
    }
    present
}

/// Simule avec Icarus Verilog : `resultat[t][net]`, ou `None` si `vvp` a planté.
fn simuler_icarus(
    etiquette: &str,
    nl: &Netlist,
    stimulus: &[Vec<Valeur>],
) -> Option<Vec<Vec<Valeur>>> {
    let dossier: PathBuf =
        std::env::temp_dir().join(format!("ce-icarus-{}-{etiquette}", std::process::id()));
    std::fs::create_dir_all(&dossier).unwrap();
    let source = dossier.join("banc.v");
    let binaire = dossier.join("banc.vvp");
    std::fs::write(&source, banc_de_test(nl, stimulus)).unwrap();
    let compilation = Command::new("iverilog")
        .arg("-o")
        .arg(&binaire)
        .arg(&source)
        .output()
        .unwrap();
    assert!(
        compilation.status.success(),
        "iverilog a échoué : {}",
        String::from_utf8_lossy(&compilation.stderr)
    );
    let execution = Command::new("vvp").arg(&binaire).output().unwrap();
    if execution.status.code().is_none() {
        // Tué par un signal : plantage de vvp, pas un résultat de simulation.
        let _ = std::fs::remove_dir_all(&dossier);
        return None;
    }
    assert!(
        execution.status.success(),
        "vvp a échoué ({:?}) sur {} : {}",
        execution.status,
        source.display(),
        String::from_utf8_lossy(&execution.stderr),
    );
    let sortie = String::from_utf8_lossy(&execution.stdout).to_string();
    let _ = std::fs::remove_dir_all(&dossier);
    let lignes: Vec<Vec<Valeur>> = sortie
        .lines()
        .filter(|l| l.len() == nl.nb_nets as usize)
        .map(|l| {
            l.chars()
                .map(|c| Valeur::de_symbole(c).unwrap_or(Valeur::X))
                .collect()
        })
        .collect();
    assert!(
        lignes.len() >= stimulus.len(),
        "sortie de vvp trop courte :\n{sortie}"
    );
    Some(lignes)
}

/// Rend les valeurs `[t][net]` et le nombre d'échéances mixtes.
fn simuler_moteur(nl: &Netlist, stimulus: &[Vec<Valeur>]) -> (Vec<Vec<Valeur>>, u64) {
    let mut sim = Simulateur::nouveau(nl).unwrap();
    let mut nets = Vec::new();
    for ligne in stimulus {
        for (i, &v) in ligne.iter().enumerate() {
            sim.fixer_entree(i, v);
        }
        nets.push((0..nl.nb_nets).map(|n| sim.valeur(n)).collect());
        sim.avancer(1);
    }
    (nets, sim.statistiques().tirs_mixtes)
}

#[derive(Default, Debug)]
struct Bilan {
    /// Circuits effectivement comparés.
    circuits: u64,
    /// Circuits écartés parce que vvp a planté.
    plantages_icarus: u64,
    comparaisons: u64,
    egaux: u64,
    /// Le moteur dit `x`, Icarus donne une valeur.
    moteur_plus_prudent: u64,
    /// Icarus dit `x`, le moteur donne une valeur.
    icarus_plus_prudent: u64,
    /// Deux valeurs définies différentes.
    contradictions: u64,
    /// Circuits simulés sans aucune échéance mixte.
    circuits_sans_mixte: u64,
    /// Parmi eux, ceux qui diffèrent d'Icarus (doit rester nul hors trois états).
    circuits_sans_mixte_differents: u64,
}

fn comparer(
    graine: u64,
    p: ParamsNetlist,
    duree: usize,
    changement: u64,
    xz: u64,
    bilan: &mut Bilan,
) {
    let mut alea = Alea::nouveau(graine);
    let mut nl = netlist_aleatoire(&mut alea, p);
    for porte in &mut nl.portes {
        // Le cas le plus fréquent de plantage de vvp (voir l'en-tête) est évité
        // d'office : un tampon trois états rebouclé directement sur lui-même.
        if porte.genre == GenrePorte::Tri {
            if porte.e0 == porte.s {
                porte.e0 = (porte.s + 1) % nl.nb_nets;
            }
            if porte.e1 == porte.s {
                porte.e1 = (porte.s + 1) % nl.nb_nets;
            }
        }
    }
    let stimulus = stimulus_aleatoire(&mut alea, p.nb_entrees, duree, changement, xz);
    let Some(icarus) = simuler_icarus(&graine.to_string(), &nl, &stimulus) else {
        bilan.plantages_icarus += 1;
        return;
    };
    bilan.circuits += 1;
    let (moteur, mixtes) = simuler_moteur(&nl, &stimulus);
    if mixtes == 0 {
        bilan.circuits_sans_mixte += 1;
        if moteur[..] != icarus[..=duree] {
            bilan.circuits_sans_mixte_differents += 1;
        }
    }
    for t in 0..=duree {
        for n in 0..nl.nb_nets as usize {
            let (m, i) = (moteur[t][n], icarus[t][n]);
            bilan.comparaisons += 1;
            if m == i {
                bilan.egaux += 1;
            } else if m == Valeur::X {
                bilan.moteur_plus_prudent += 1;
            } else if i == Valeur::X {
                bilan.icarus_plus_prudent += 1;
            } else {
                bilan.contradictions += 1;
            }
        }
    }
}

fn params(
    nb_portes: usize,
    delai_max: u16,
    tri: u64,
    partage: u64,
    horloges: usize,
) -> ParamsNetlist {
    ParamsNetlist {
        nb_entrees: 5,
        nb_portes,
        delai_max,
        pourcent_tri: tri,
        pourcent_partage: partage,
        nb_constantes: 2,
        nb_horloges: horloges,
    }
}

#[test]
fn nand_delai_unite_egalite_exacte() {
    if !icarus_disponible() {
        return;
    }
    let mut bilan = Bilan::default();
    for graine in 0..40 {
        comparer(graine, params(120, 1, 0, 0, 1), 250, 20, 15, &mut bilan);
    }
    println!("{bilan:?}");
    assert_eq!(bilan.egaux, bilan.comparaisons, "{bilan:?}");
    assert_eq!(bilan.plantages_icarus, 0);
    assert!(bilan.comparaisons > 1_000_000);
}

#[test]
fn nand_delais_varies_seule_la_regle_mixte_ecarte_de_verilog() {
    if !icarus_disponible() {
        return;
    }
    let mut bilan = Bilan::default();
    // Entrées binaires : peu d'échéances mixtes, beaucoup de circuits exactement égaux.
    for graine in 100..160 {
        comparer(graine, params(60, 6, 0, 0, 1), 250, 25, 0, &mut bilan);
    }
    // Entrées avec x et z : la règle mixte joue souvent.
    for graine in 200..240 {
        comparer(graine, params(120, 6, 0, 0, 1), 250, 25, 20, &mut bilan);
    }
    println!("{bilan:?}");
    assert_eq!(bilan.plantages_icarus, 0);
    assert_eq!(bilan.circuits_sans_mixte_differents, 0, "{bilan:?}");
    assert!(bilan.circuits_sans_mixte >= 10, "{bilan:?}");
    assert_eq!(bilan.icarus_plus_prudent, 0, "{bilan:?}");
    assert_eq!(bilan.contradictions, 0, "{bilan:?}");
    assert!(
        bilan.moteur_plus_prudent > 0,
        "le cas mixte doit se produire"
    );
    assert!(bilan.egaux * 100 >= bilan.comparaisons * 99, "{bilan:?}");
}

#[test]
fn trois_etats_et_pilotes_multiples() {
    if !icarus_disponible() {
        return;
    }
    let mut bilan = Bilan::default();
    for graine in 300..340 {
        comparer(graine, params(100, 1, 30, 20, 1), 250, 20, 15, &mut bilan);
    }
    for graine in 400..440 {
        comparer(graine, params(100, 5, 30, 20, 1), 250, 20, 15, &mut bilan);
    }
    println!("{bilan:?}");
    assert_eq!(bilan.icarus_plus_prudent, 0, "{bilan:?}");
    assert_eq!(bilan.contradictions, 0, "{bilan:?}");
    assert!(bilan.circuits >= 60, "trop de plantages de vvp : {bilan:?}");
    assert!(bilan.egaux * 100 >= bilan.comparaisons * 95, "{bilan:?}");
}
