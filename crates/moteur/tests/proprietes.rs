//! Propriétés algébriques de la logique à quatre valeurs et théorème de sûreté du X.
//!
//! Les propriétés sur les valeurs sont vérifiées exhaustivement (le domaine est
//! fini). La monotonie de la simulation complète est démontrée dans
//! `docs/semantique.md` ; elle est ici recherchée en contre-exemple sur des
//! milliers de circuits aléatoires.

mod commun;

use ce_moteur::alea::{netlist_aleatoire, stimulus_aleatoire, Alea, ParamsNetlist};
use ce_moteur::specification::sortie_suivante;
use ce_moteur::valeur::{court_circuit, nand, resoudre, resoudre_tous, tri};
use ce_moteur::Valeur;
use commun::{chaine, derouler, derouler_spec};

const V: [Valeur; 4] = Valeur::TOUTES;

#[test]
fn ordre_d_information_est_un_ordre() {
    for a in V {
        assert!(a.precisee_par(a), "réflexivité");
        for b in V {
            if a.precisee_par(b) && b.precisee_par(a) {
                assert_eq!(a, b, "antisymétrie");
            }
            for c in V {
                if a.precisee_par(b) && b.precisee_par(c) {
                    assert!(a.precisee_par(c), "transitivité");
                }
            }
        }
    }
}

#[test]
fn nand_et_tri_sont_monotones() {
    for a in V {
        for b in V {
            for a2 in V {
                for b2 in V {
                    if a.precisee_par(a2) && b.precisee_par(b2) {
                        assert!(
                            nand(a, b).precisee_par(nand(a2, b2)),
                            "nand {a}{b} {a2}{b2}"
                        );
                        assert!(tri(a, b).precisee_par(tri(a2, b2)), "tri {a}{b} {a2}{b2}");
                    }
                }
            }
        }
    }
}

#[test]
fn nand_est_commutative() {
    for a in V {
        for b in V {
            assert_eq!(nand(a, b), nand(b, a));
        }
    }
}

#[test]
fn resolution_est_commutative_associative_idempotente_et_monotone() {
    for a in V {
        assert_eq!(resoudre(a, a), a, "idempotence");
        assert_eq!(resoudre(a, Valeur::Z), a, "Z est neutre");
        assert_eq!(resoudre(a, Valeur::X), Valeur::X, "X est absorbant");
        for b in V {
            assert_eq!(resoudre(a, b), resoudre(b, a), "commutativité");
            for c in V {
                assert_eq!(
                    resoudre(resoudre(a, b), c),
                    resoudre(a, resoudre(b, c)),
                    "associativité"
                );
            }
            for a2 in V {
                for b2 in V {
                    if a.precisee_par(a2) && b.precisee_par(b2) {
                        assert!(resoudre(a, b).precisee_par(resoudre(a2, b2)), "monotonie");
                    }
                }
            }
        }
    }
}

#[test]
fn resolution_par_comptage_equivaut_au_pliage() {
    // Tous les multi-ensembles de 0 à 4 pilotes.
    for n in 0..=4u32 {
        for code in 0..4u32.pow(n) {
            let pilotes: Vec<Valeur> = (0..n)
                .map(|k| Valeur::de_code(((code >> (2 * k)) & 3) as u8))
                .collect();
            let nx = pilotes.iter().filter(|&&v| v == Valeur::X).count();
            let n0 = pilotes.iter().filter(|&&v| v == Valeur::Zero).count();
            let n1 = pilotes.iter().filter(|&&v| v == Valeur::Un).count();
            let attendu = if nx > 0 || (n0 > 0 && n1 > 0) {
                Valeur::X
            } else if n0 > 0 {
                Valeur::Zero
            } else if n1 > 0 {
                Valeur::Un
            } else {
                Valeur::Z
            };
            assert_eq!(
                resoudre_tous(pilotes.iter().copied()),
                attendu,
                "{pilotes:?}"
            );
            assert_eq!(court_circuit(pilotes.iter().copied()), n0 > 0 && n1 > 0);
        }
    }
}

/// Lemme 3 de `docs/semantique.md` : la règle du délai inertiel sûr est monotone.
/// Vérification exhaustive pour toutes les fenêtres de longueur 1 à 4.
#[test]
fn delai_inertiel_sur_est_monotone() {
    for d in 1..=4usize {
        let nb = 4usize.pow(d as u32);
        for code in 0..nb {
            let f: Vec<Valeur> = (0..d)
                .map(|k| Valeur::de_code(((code >> (2 * k)) & 3) as u8))
                .collect();
            for code2 in 0..nb {
                let f2: Vec<Valeur> = (0..d)
                    .map(|k| Valeur::de_code(((code2 >> (2 * k)) & 3) as u8))
                    .collect();
                if !f.iter().zip(&f2).all(|(a, b)| a.precisee_par(*b)) {
                    continue;
                }
                for p in V {
                    for p2 in V {
                        if p.precisee_par(p2) {
                            let (s, s2) = (sortie_suivante(&f, p), sortie_suivante(&f2, p2));
                            assert!(
                                s.precisee_par(s2),
                                "fenêtres {} ⊑ {}, précédentes {p} ⊑ {p2} : {s} n'est pas précisée par {s2}",
                                chaine(&f),
                                chaine(&f2)
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Sur des signaux binaires, la règle est le délai inertiel classique : la sortie
/// change si et seulement si la cible est restée constante pendant tout le délai.
#[test]
fn delai_inertiel_binaire_est_le_modele_classique() {
    for d in 1..=6usize {
        for code in 0..(1usize << d) {
            let f: Vec<Valeur> = (0..d)
                .map(|k| Valeur::de_bool((code >> k) & 1 == 1))
                .collect();
            for p in [Valeur::Zero, Valeur::Un] {
                let constante = f.iter().all(|&v| v == f[0]);
                let attendu = if constante { f[0] } else { p };
                assert_eq!(sortie_suivante(&f, p), attendu);
            }
        }
    }
}

/// Remplace chaque X et chaque Z-ou-X d'un stimulus par une valeur plus précise.
fn preciser(alea: &mut Alea, stimulus: &[Vec<Valeur>]) -> Vec<Vec<Valeur>> {
    stimulus
        .iter()
        .map(|ligne| {
            ligne
                .iter()
                .map(|&v| {
                    if v == Valeur::X {
                        V[alea.sous(4) as usize]
                    } else {
                        v
                    }
                })
                .collect()
        })
        .collect()
}

/// Théorème de sûreté du X, recherché en contre-exemple : si l'on précise les
/// entrées inconnues, toute valeur que la simulation annonçait reste vraie.
#[test]
fn surete_du_x_sur_circuits_aleatoires() {
    let mut nb_definis = 0u64;
    let mut nb_x_precises = 0u64;
    for graine in 0..1500u64 {
        let mut alea = Alea::nouveau(0xC0FFEE + graine);
        let p = ParamsNetlist {
            nb_entrees: 4,
            nb_portes: 14,
            delai_max: 1 + (graine % 6) as u16,
            pourcent_tri: if graine % 3 == 0 { 0 } else { 25 },
            pourcent_partage: if graine % 3 == 0 { 0 } else { 15 },
            nb_constantes: 1,
            nb_horloges: (graine % 2) as usize,
        };
        let nl = netlist_aleatoire(&mut alea, p);
        let duree = 100;
        let flou = stimulus_aleatoire(&mut alea, p.nb_entrees, duree, 30, 45);
        let (nets_flous, sorties_floues) = derouler(&nl, &flou);
        for _ in 0..4 {
            let precis = preciser(&mut alea, &flou);
            let (nets_precis, sorties_precises) = derouler(&nl, &precis);
            let spec = derouler_spec(&nl, &precis);
            assert_eq!(nets_precis, spec.nets, "graine {graine}");
            for t in 0..=duree {
                for n in 0..nl.nb_nets as usize {
                    let (a, b) = (nets_flous[t][n], nets_precis[t][n]);
                    assert!(
                        a.precisee_par(b),
                        "graine {graine}, instant {t}, net {n} : {a} annoncé, {b} après précision\n{nl:#?}"
                    );
                    if a != Valeur::X {
                        nb_definis += 1;
                    } else if b != Valeur::X {
                        nb_x_precises += 1;
                    }
                }
                for g in 0..nl.portes.len() {
                    assert!(sorties_floues[t][g].precisee_par(sorties_precises[t][g]));
                }
            }
        }
    }
    // Le test n'a de valeur que si les deux situations se produisent souvent.
    assert!(nb_definis > 100_000, "{nb_definis}");
    assert!(nb_x_precises > 100_000, "{nb_x_precises}");
}
