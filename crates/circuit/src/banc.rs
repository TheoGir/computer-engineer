//! Banc de test : un composant mis à plat et son simulateur, pilotés par les noms
//! de broches.

use crate::elaboration::Circuit;
use crate::erreur::Erreur;
use crate::modele::{Bibliotheque, Sens};
use ce_moteur::{Options, Simulateur, Stabilite, Valeur};

/// Un composant en cours de simulation.
pub struct Banc {
    /// Le circuit mis à plat.
    pub circuit: Circuit,
    /// Le simulateur.
    pub sim: Simulateur,
}

impl Banc {
    /// Met à plat le composant `nom` et prépare sa simulation.
    pub fn nouveau(bib: &Bibliotheque, nom: &str) -> Result<Banc, Erreur> {
        Banc::de_circuit(Circuit::elaborer(bib, nom)?)
    }

    /// Prépare la simulation d'un circuit déjà mis à plat.
    pub fn de_circuit(circuit: Circuit) -> Result<Banc, Erreur> {
        let sim = Simulateur::nouveau(&circuit.netlist)?;
        Ok(Banc { circuit, sim })
    }

    /// Recommence la simulation depuis l'instant 0 avec d'autres réglages du
    /// simulateur (par exemple sans mise en sommeil, pour comparer).
    pub fn regler(&mut self, options: Options) -> Result<(), Erreur> {
        self.sim = Simulateur::avec_options(&self.circuit.netlist, options)?;
        Ok(())
    }

    /// Première entrée externe et largeur de la broche d'entrée `broche`.
    pub(crate) fn entree(&self, broche: &str) -> Result<(usize, usize), Erreur> {
        let erreur = |raison: &str| Erreur::Chemin {
            chemin: broche.to_string(),
            raison: raison.to_string(),
        };
        let b = self
            .circuit
            .broche(broche)
            .ok_or_else(|| erreur("broche inconnue"))?;
        match b.premiere_entree {
            Some(p) if b.sens == Sens::Entree => Ok((p, b.nets.len())),
            _ => Err(erreur("ce n'est pas une entrée")),
        }
    }

    /// Écrit un entier sur une broche d'entrée (bit 0 = poids faible).
    pub fn ecrire(&mut self, broche: &str, valeur: u64) -> Result<(), Erreur> {
        let (premiere, largeur) = self.entree(broche)?;
        for k in 0..largeur {
            let bit = k < 64 && (valeur >> k) & 1 == 1;
            self.sim.fixer_entree(premiere + k, Valeur::de_bool(bit));
        }
        Ok(())
    }

    /// Écrit des valeurs quelconques (`X` et `Z` compris) sur une broche d'entrée.
    pub fn ecrire_valeurs(&mut self, broche: &str, valeurs: &[Valeur]) -> Result<(), Erreur> {
        let (premiere, largeur) = self.entree(broche)?;
        if valeurs.len() != largeur {
            return Err(Erreur::Chemin {
                chemin: broche.to_string(),
                raison: format!("{} valeurs données pour {} bits", valeurs.len(), largeur),
            });
        }
        for (k, &v) in valeurs.iter().enumerate() {
            self.sim.fixer_entree(premiere + k, v);
        }
        Ok(())
    }

    /// Valeurs des bits désignés par un chemin (broche ou net interne), bit 0 d'abord.
    pub fn lire_valeurs(&self, chemin: &str) -> Result<Vec<Valeur>, Erreur> {
        Ok(self
            .circuit
            .nets(chemin)?
            .into_iter()
            .map(|n| self.sim.valeur(n))
            .collect())
    }

    /// Entier lu sur un chemin, ou `None` si un bit n'est ni `0` ni `1`.
    pub fn lire(&self, chemin: &str) -> Result<Option<u64>, Erreur> {
        let valeurs = self.lire_valeurs(chemin)?;
        if valeurs.len() > 64 {
            return Err(Erreur::Chemin {
                chemin: chemin.to_string(),
                raison: "plus de 64 bits".to_string(),
            });
        }
        let mut n = 0u64;
        for (k, v) in valeurs.iter().enumerate() {
            match v.en_bool() {
                Some(true) => n |= 1 << k,
                Some(false) => {}
                None => return Ok(None),
            }
        }
        Ok(Some(n))
    }

    /// Texte des bits d'un chemin, poids fort à gauche : `"01x1"`.
    pub fn lire_texte(&self, chemin: &str) -> Result<String, Erreur> {
        Ok(self
            .lire_valeurs(chemin)?
            .iter()
            .rev()
            .map(|v| v.symbole())
            .collect())
    }

    /// Avance le temps.
    pub fn avancer(&mut self, duree: u64) {
        self.sim.avancer(duree);
    }

    /// Attend le repos du circuit, pendant au plus `budget` unités de temps.
    pub fn stabiliser(&mut self, budget: u64) -> Stabilite {
        self.sim.stabiliser(budget)
    }
}
