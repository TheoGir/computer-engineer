//! Générateur pseudo-aléatoire déterministe (SplitMix64).
//!
//! Les tests aléatoires du projet fixent tous leur graine : un échec se rejoue à
//! l'identique. Ce générateur n'a aucune prétention cryptographique.

/// Générateur SplitMix64.
#[derive(Clone, Debug)]
pub struct Alea(u64);

impl Alea {
    /// Crée un générateur à partir d'une graine.
    pub fn nouveau(graine: u64) -> Self {
        Alea(graine)
    }

    /// Prochain entier de 64 bits.
    pub fn u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Entier uniforme dans `0..n` (`n` doit être non nul).
    pub fn sous(&mut self, n: u64) -> u64 {
        assert!(n > 0, "intervalle vide");
        // Le biais de modulo est négligeable pour les petits `n` utilisés dans les tests.
        self.u64() % n
    }

    /// Entier uniforme dans `min..=max`.
    pub fn entre(&mut self, min: u64, max: u64) -> u64 {
        min + self.sous(max - min + 1)
    }

    /// Vrai avec la probabilité `num / den`.
    pub fn chance(&mut self, num: u64, den: u64) -> bool {
        self.sous(den) < num
    }
}

use crate::netlist::Netlist;
use crate::valeur::Valeur;

/// Paramètres de [`netlist_aleatoire`].
#[derive(Clone, Copy, Debug)]
pub struct ParamsNetlist {
    /// Nombre d'entrées externes.
    pub nb_entrees: usize,
    /// Nombre de portes.
    pub nb_portes: usize,
    /// Délai maximal d'une porte (les délais sont tirés dans `1..=delai_max`).
    pub delai_max: u16,
    /// Proportion de tampons trois états, en pour cent.
    pub pourcent_tri: u64,
    /// Proportion de portes dont la sortie est branchée sur un net déjà piloté,
    /// en pour cent (crée des nets à plusieurs pilotes).
    pub pourcent_partage: u64,
    /// Nombre de constantes.
    pub nb_constantes: usize,
    /// Nombre d'horloges.
    pub nb_horloges: usize,
}

/// Fabrique un circuit quelconque, boucles comprises, pour les tests aléatoires.
pub fn netlist_aleatoire(alea: &mut Alea, p: ParamsNetlist) -> Netlist {
    let mut nl = Netlist::nouvelle();
    for _ in 0..p.nb_entrees {
        let n = nl.net();
        nl.entree(n);
    }
    for _ in 0..p.nb_constantes {
        let n = nl.net();
        nl.constante(n, Valeur::de_bool(alea.chance(1, 2)));
    }
    for _ in 0..p.nb_horloges {
        let n = nl.net();
        nl.horloge(n, alea.entre(1, 5));
    }
    // Un net flottant, pour exercer la lecture d'un Z.
    nl.net();
    let sorties = nl.nets(p.nb_portes);
    let nb_nets = nl.nb_nets as u64;
    for &propre in &sorties {
        let e0 = alea.sous(nb_nets) as u32;
        let e1 = alea.sous(nb_nets) as u32;
        let s = if alea.chance(p.pourcent_partage, 100) {
            alea.sous(nb_nets) as u32
        } else {
            propre
        };
        let delai = alea.entre(1, p.delai_max as u64) as u16;
        if alea.chance(p.pourcent_tri, 100) {
            nl.tri(e0, e1, s, delai);
        } else {
            nl.nand(e0, e1, s, delai);
        }
    }
    nl
}

/// Stimulus aléatoire : `resultat[t][i]` est la valeur de l'entrée `i` à l'instant `t`.
///
/// Chaque entrée change avec la probabilité `pourcent_changement` à chaque instant ;
/// `pourcent_xz` règle la part de `X` et de `Z` parmi les valeurs tirées.
pub fn stimulus_aleatoire(
    alea: &mut Alea,
    nb_entrees: usize,
    duree: usize,
    pourcent_changement: u64,
    pourcent_xz: u64,
) -> Vec<Vec<Valeur>> {
    let tirer = |alea: &mut Alea| {
        if alea.chance(pourcent_xz, 100) {
            if alea.chance(2, 3) {
                Valeur::X
            } else {
                Valeur::Z
            }
        } else {
            Valeur::de_bool(alea.chance(1, 2))
        }
    };
    let mut courant: Vec<Valeur> = (0..nb_entrees).map(|_| tirer(alea)).collect();
    let mut resultat = Vec::with_capacity(duree + 1);
    for _ in 0..=duree {
        for v in courant.iter_mut() {
            if alea.chance(pourcent_changement, 100) {
                *v = tirer(alea);
            }
        }
        resultat.push(courant.clone());
    }
    resultat
}
