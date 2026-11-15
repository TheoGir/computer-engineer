//! Les constructions minimales : chaque construction rangée calcule bien la même chose que son
//! corrigé, et le tableau des fonctions de 3 entrées est recoupé par une énumération complète.

use std::collections::BTreeMap;

use ce_circuit::analyse::table_de_verite;
use ce_circuit::minimum::{
    self, etablis, minimum_d_une_fonction, minimums_a_trois_entrees, nb_nand, Ecrit, Statut,
};
use ce_circuit::Circuit;
use ce_moteur::Valeur;

#[test]
fn chaque_construction_rangee_calcule_son_corrige() {
    let bib = ce_circuit::classiques::bibliotheque();
    assert!(!etablis().is_empty());
    for e in etablis().values() {
        let corrige = Circuit::elaborer(&bib, &e.composant)
            .unwrap_or_else(|err| panic!("{} : {err}", e.composant));
        let n_corrige = nb_nand(&corrige);
        assert!(e.borne_basse <= e.nand, "{}", e.composant);
        assert!(e.nand <= n_corrige, "{} : pire que le corrigé", e.composant);
        let Some(texte) = &e.circuit else {
            assert_eq!(
                e.nand, n_corrige,
                "{} : sans circuit rangé, c'est le corrigé qui fait foi",
                e.composant
            );
            continue;
        };
        assert!(
            e.nand < n_corrige,
            "{} : circuit rangé inutile",
            e.composant
        );
        let ecrit = Ecrit::lire(texte).unwrap_or_else(|| panic!("{} : texte", e.composant));
        assert_eq!(ecrit.portes.len() as u64, e.nand, "{}", e.composant);
        let table = table_de_verite(&bib, &e.composant).unwrap();
        let bits: u32 = table.entrees.iter().map(|(_, l)| l).sum();
        assert!(ecrit.entrees as u32 <= bits, "{}", e.composant);
        for (r, ligne) in table.lignes.iter().enumerate() {
            let vu = ecrit.calculer(r as u64);
            assert_eq!(vu.len(), ligne.len(), "{} : nombre de sorties", e.composant);
            for (o, v) in ligne.iter().enumerate() {
                match v {
                    Valeur::Un => assert!(vu[o], "{} ligne {r} sortie {o}", e.composant),
                    Valeur::Zero => assert!(!vu[o], "{} ligne {r} sortie {o}", e.composant),
                    _ => {}
                }
            }
        }
    }
}

#[test]
fn les_minimums_connus_de_la_litterature_sont_retrouves() {
    let bib = ce_circuit::classiques::bibliotheque();
    // Valeurs classiques : NON 1, ET 2, OU 3, OU exclusif 4, équivalence 5, demi-additionneur 5,
    // additionneur complet 9 (portes NAND à deux entrées).
    for (nom, n) in [
        ("NON", 1),
        ("ET", 2),
        ("OU", 3),
        ("OU_X", 4),
        ("EQUIV", 5),
        ("DEMI_ADD", 5),
        ("ADD_COMPLET", 9),
    ] {
        let m = minimum::minimum(&bib, nom).unwrap();
        assert_eq!((m.nand, m.statut), (n, Statut::Prouve), "{nom}");
    }
    // Sans résultat établi : le corrigé, au statut de référence.
    let m = minimum::minimum(&bib, "REGISTRE8").unwrap();
    assert_eq!(m.statut, Statut::Reference);
    assert_eq!(m.nand, 104);
}

#[test]
fn chaque_fonction_de_trois_entrees_a_sa_construction() {
    let t = minimums_a_trois_entrees();
    for (f, (k, ecrit)) in t.iter().enumerate() {
        assert_eq!(ecrit.portes.len() as u64, *k, "fonction {f}");
        assert_eq!(ecrit.sorties.len(), 1);
        for r in 0..8u64 {
            assert_eq!(
                ecrit.calculer(r)[0],
                (f >> r) & 1 == 1,
                "fonction {f}, ligne {r}"
            );
        }
    }
}

/// Le plus petit nombre de NAND pour chaque fonction de 3 entrées, en énumérant TOUS les
/// circuits d'au plus `max` portes : méthode indépendante du solveur SAT.
fn enumerer(max: u32) -> BTreeMap<u8, u32> {
    fn descendre(signaux: &mut Vec<u8>, k: u32, max: u32, meilleur: &mut BTreeMap<u8, u32>) {
        if k == max {
            return;
        }
        let m = signaux.len();
        for i in 0..m {
            for j in i..m {
                let g = !(signaux[i] & signaux[j]);
                let e = meilleur.entry(g).or_insert(u32::MAX);
                *e = (*e).min(k + 1);
                signaux.push(g);
                descendre(signaux, k + 1, max, meilleur);
                signaux.pop();
            }
        }
    }
    let entrees = [0b1010_1010u8, 0b1100_1100, 0b1111_0000];
    let mut meilleur: BTreeMap<u8, u32> = entrees.iter().map(|&f| (f, 0)).collect();
    descendre(&mut entrees.to_vec(), 0, max, &mut meilleur);
    meilleur
}

#[test]
fn l_enumeration_complete_confirme_le_tableau_jusqu_a_six_portes() {
    const MAX: u32 = 6;
    let enumere = enumerer(MAX);
    let t = minimums_a_trois_entrees();
    for f in 0..=255u8 {
        let k = t[f as usize].0;
        match enumere.get(&f) {
            Some(&e) => assert_eq!(k, e as u64, "fonction {f:08b}"),
            None => assert!(k > MAX as u64, "fonction {f:08b} : {k} portes annoncées"),
        }
    }
    // Les fonctions atteintes en 0 à 6 portes : 3 + 6 + 13 + 26 + 43 + 48 + 53.
    assert_eq!(enumere.len(), 192);
}

#[test]
fn une_fonction_de_moins_de_trois_entrees_garde_son_minimum() {
    assert_eq!(minimum_d_une_fonction(1, 0b01), Some(1)); // NON
    assert_eq!(minimum_d_une_fonction(1, 0b10), Some(0)); // a0 lui-même
    assert_eq!(minimum_d_une_fonction(2, 0b0111), Some(1)); // NAND
    assert_eq!(minimum_d_une_fonction(2, 0b1000), Some(2)); // ET
    assert_eq!(minimum_d_une_fonction(2, 0b1110), Some(3)); // OU
    assert_eq!(minimum_d_une_fonction(2, 0b0110), Some(4)); // OU exclusif
    assert_eq!(minimum_d_une_fonction(3, 0b1110_1000), Some(6)); // majorité
    assert_eq!(minimum_d_une_fonction(3, 0b1001_0110), Some(8)); // parité de 3 bits
    assert_eq!(minimum_d_une_fonction(4, 0), None);
}
