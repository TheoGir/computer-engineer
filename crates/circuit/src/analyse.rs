//! Analyses statiques et vérifications exhaustives.

use crate::banc::Banc;
use crate::elaboration::Circuit;
use crate::erreur::Erreur;
use crate::modele::{Bibliotheque, Sens};
use ce_moteur::{GenrePorte, NetId, Netlist, Valeur};

/// Chemin critique d'un circuit sans boucle : le plus long cumul de délais entre
/// une source (entrée, constante, horloge) et une sortie de porte.
///
/// Rend `None` si le circuit contient une boucle : il n'est alors pas
/// combinatoire, et son état peut dépendre du passé.
///
/// **Propriété** (démontrée dans `docs/semantique.md`) : dans un circuit sans
/// boucle dont les entrées ne bougent plus depuis l'instant `t0`, plus aucun net ne
/// change après `t0 + chemin critique`.
pub fn chemin_critique(netlist: &Netlist) -> Option<u64> {
    let nb_portes = netlist.portes.len();
    let nb_nets = netlist.nb_nets as usize;
    // Portes pilotant chaque net.
    let mut pilotes_debut = vec![0u32; nb_nets + 1];
    for p in &netlist.portes {
        pilotes_debut[p.s as usize + 1] += 1;
    }
    for n in 0..nb_nets {
        pilotes_debut[n + 1] += pilotes_debut[n];
    }
    let mut curseur = pilotes_debut.clone();
    let mut pilotes = vec![0u32; nb_portes];
    for (g, p) in netlist.portes.iter().enumerate() {
        pilotes[curseur[p.s as usize] as usize] = g as u32;
        curseur[p.s as usize] += 1;
    }
    let pilotes_de = |net: NetId| {
        &pilotes[pilotes_debut[net as usize] as usize..pilotes_debut[net as usize + 1] as usize]
    };
    // Lecteurs de chaque net.
    let mut lecteurs: Vec<Vec<u32>> = vec![Vec::new(); nb_nets];
    for (g, p) in netlist.portes.iter().enumerate() {
        lecteurs[p.e0 as usize].push(g as u32);
        if p.e1 != p.e0 {
            lecteurs[p.e1 as usize].push(g as u32);
        }
    }
    // Tri topologique de Kahn sur le graphe « la porte u pilote une entrée de v ».
    let mut attente: Vec<u32> = netlist
        .portes
        .iter()
        .map(|p| {
            let mut n = pilotes_de(p.e0).len();
            if p.e1 != p.e0 {
                n += pilotes_de(p.e1).len();
            }
            n as u32
        })
        .collect();
    let mut arrivee = vec![0u64; nb_portes];
    let mut file: Vec<u32> = (0..nb_portes as u32)
        .filter(|&g| attente[g as usize] == 0)
        .collect();
    let mut traitees = 0usize;
    let mut maximum = 0u64;
    while let Some(g) = file.pop() {
        traitees += 1;
        let p = &netlist.portes[g as usize];
        let sortie = arrivee[g as usize] + p.delai as u64;
        maximum = maximum.max(sortie);
        for &v in &lecteurs[p.s as usize] {
            let v = v as usize;
            arrivee[v] = arrivee[v].max(sortie);
            attente[v] -= 1;
            if attente[v] == 0 {
                file.push(v as u32);
            }
        }
    }
    (traitees == nb_portes).then_some(maximum)
}

/// Nets pilotés en permanence par au moins deux sources qui ne savent pas se taire
/// (portes NAND, constantes, entrées, horloges) : un court-circuit y est possible
/// quelles que soient les commandes. Les tampons trois états ne comptent pas.
pub fn pilotes_en_conflit(netlist: &Netlist) -> Vec<NetId> {
    let mut n = vec![0u32; netlist.nb_nets as usize];
    for p in &netlist.portes {
        if p.genre == GenrePorte::Nand {
            n[p.s as usize] += 1;
        }
    }
    for &(net, _) in &netlist.constantes {
        n[net as usize] += 1;
    }
    for &net in &netlist.entrees {
        n[net as usize] += 1;
    }
    for h in &netlist.horloges {
        n[h.net as usize] += 1;
    }
    (0..netlist.nb_nets)
        .filter(|&i| n[i as usize] > 1)
        .collect()
}

/// Table de vérité d'un composant combinatoire.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableDeVerite {
    /// Noms et largeurs des entrées, dans l'ordre des broches.
    pub entrees: Vec<(String, u32)>,
    /// Noms et largeurs des sorties, dans l'ordre des broches.
    pub sorties: Vec<(String, u32)>,
    /// `lignes[i]` : valeurs de tous les bits de sortie (première sortie d'abord,
    /// bit 0 d'abord) quand les bits d'entrée valent les bits de l'entier `i`
    /// (première entrée d'abord, bit 0 d'abord).
    pub lignes: Vec<Vec<Valeur>>,
    /// Chemin critique du circuit, en unités de temps.
    pub chemin_critique: u64,
}

/// Nombre maximal de bits d'entrée pour une table de vérité exhaustive.
pub const MAX_BITS_TABLE: u32 = 24;

/// Calcule la table de vérité exhaustive du composant `nom`.
///
/// Le composant doit être combinatoire (sans boucle ni horloge) et avoir au plus
/// [`MAX_BITS_TABLE`] bits d'entrée. Chaque ligne est obtenue en simulant le
/// circuit jusqu'à son chemin critique : c'est une preuve par exhaustion, pas un
/// échantillonnage.
pub fn table_de_verite(bib: &Bibliotheque, nom: &str) -> Result<TableDeVerite, Erreur> {
    table_du_circuit(Circuit::elaborer(bib, nom)?)
}

fn table_du_circuit(circuit: Circuit) -> Result<TableDeVerite, Erreur> {
    let nom = circuit.nom.clone();
    let Some(critique) = chemin_critique(&circuit.netlist) else {
        return Err(Erreur::Analyse(format!(
            "« {nom} » contient une boucle : ce n'est pas un circuit combinatoire, il n'a pas de table de vérité"
        )));
    };
    if !circuit.netlist.horloges.is_empty() {
        return Err(Erreur::Analyse(format!(
            "« {nom} » contient une horloge : il n'a pas de table de vérité"
        )));
    }
    let mut entrees = Vec::new();
    let mut sorties = Vec::new();
    let mut nets_sorties = Vec::new();
    for b in &circuit.broches {
        match b.sens {
            Sens::Entree => entrees.push((b.nom.clone(), b.nets.len() as u32)),
            Sens::Sortie => {
                sorties.push((b.nom.clone(), b.nets.len() as u32));
                nets_sorties.extend(&b.nets);
            }
        }
    }
    let nb_bits: u32 = entrees.iter().map(|(_, l)| l).sum();
    if nb_bits > MAX_BITS_TABLE {
        return Err(Erreur::Analyse(format!(
            "« {nom} » a {nb_bits} bits d'entrée : au-delà de {MAX_BITS_TABLE}, la table exhaustive est trop grande"
        )));
    }
    let mut banc = Banc::de_circuit(circuit)?;
    let mut lignes = Vec::with_capacity(1 << nb_bits);
    for combinaison in 0..(1u64 << nb_bits) {
        for bit in 0..nb_bits as usize {
            banc.sim
                .fixer_entree(bit, Valeur::de_bool((combinaison >> bit) & 1 == 1));
        }
        // Sans boucle, tout est figé au plus tard après le chemin critique.
        banc.sim.avancer(critique + 1);
        debug_assert!(banc.sim.est_stable());
        lignes.push(nets_sorties.iter().map(|&n| banc.sim.valeur(n)).collect());
    }
    Ok(TableDeVerite {
        entrees,
        sorties,
        lignes,
        chemin_critique: critique,
    })
}

/// Résultat de la comparaison de deux composants combinatoires.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Equivalence {
    /// Mêmes sorties pour toutes les combinaisons d'entrées : équivalence prouvée
    /// par exhaustion. `combinaisons` est le nombre de cas vérifiés.
    Prouvee { combinaisons: u64 },
    /// Les sorties diffèrent pour cette combinaison d'entrées.
    Differents {
        /// Combinaison d'entrées (même codage que [`TableDeVerite::lignes`]).
        entrees: u64,
        /// Sorties du premier composant.
        sorties_a: Vec<Valeur>,
        /// Sorties du second composant.
        sorties_b: Vec<Valeur>,
    },
}

/// Compare exhaustivement deux composants combinatoires de mêmes broches.
pub fn equivalence(
    bib_a: &Bibliotheque,
    nom_a: &str,
    bib_b: &Bibliotheque,
    nom_b: &str,
) -> Result<Equivalence, Erreur> {
    let ta = table_de_verite(bib_a, nom_a)?;
    let tb = table_de_verite(bib_b, nom_b)?;
    if ta.entrees != tb.entrees || ta.sorties != tb.sorties {
        return Err(Erreur::Analyse(format!(
            "« {nom_a} » et « {nom_b} » n'ont pas les mêmes broches : ils ne sont pas comparables"
        )));
    }
    for (i, (la, lb)) in ta.lignes.iter().zip(&tb.lignes).enumerate() {
        if la != lb {
            return Ok(Equivalence::Differents {
                entrees: i as u64,
                sorties_a: la.clone(),
                sorties_b: lb.clone(),
            });
        }
    }
    Ok(Equivalence::Prouvee {
        combinaisons: ta.lignes.len() as u64,
    })
}
