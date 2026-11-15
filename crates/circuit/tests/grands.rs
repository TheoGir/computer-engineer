//! Les grands circuits de mesure calculent bien ce qu'ils annoncent.

use ce_circuit::analyse::table_de_verite;
use ce_circuit::grands::{self, MemoireEnMarche};
use ce_circuit::ordinateur::{self, Machine, Reglages};
use ce_circuit::Banc;
use ce_moteur::alea::Alea;
use ce_moteur::Valeur;

#[test]
fn les_decodeurs_sont_prouves_par_exhaustion() {
    for bits in 1..=7u32 {
        let mut bib = grands::bibliotheque();
        let nom = grands::decodeur(&mut bib, bits);
        let table = table_de_verite(&bib, &nom).unwrap();
        for (v, ligne) in table.lignes.iter().enumerate() {
            for (k, &s) in ligne.iter().enumerate() {
                assert_eq!(
                    s,
                    Valeur::de_bool(k == v),
                    "décodeur {bits} bits, e = {v}, s[{k}]"
                );
            }
        }
    }
}

#[test]
fn le_multiplicateur_4_bits_est_prouve_par_exhaustion() {
    for bits in [2u32, 3, 4, 5] {
        let mut bib = grands::bibliotheque();
        let nom = grands::multiplicateur(&mut bib, bits);
        let table = table_de_verite(&bib, &nom).unwrap();
        for (i, ligne) in table.lignes.iter().enumerate() {
            let (a, b) = (i & ((1 << bits) - 1), i >> bits);
            let p: usize = ligne
                .iter()
                .enumerate()
                .map(|(k, v)| (v.en_bool().expect("sortie binaire") as usize) << k)
                .sum();
            assert_eq!(p, a * b, "{a} × {b} sur {bits} bits");
        }
    }
}

#[test]
fn le_multiplicateur_32_bits_multiplie() {
    let mut bib = grands::bibliotheque();
    let nom = grands::multiplicateur(&mut bib, 32);
    let mut banc = Banc::nouveau(&bib, &nom).unwrap();
    let mut alea = Alea::nouveau(7);
    for essai in 0..40 {
        let (a, b) = match essai {
            0 => (0, 0),
            1 => (u32::MAX as u64, u32::MAX as u64),
            2 => (1, u32::MAX as u64),
            _ => (alea.u64() >> 32, alea.u64() >> 32),
        };
        banc.ecrire("a", a).unwrap();
        banc.ecrire("b", b).unwrap();
        assert!(banc.stabiliser(100_000).est_stable());
        assert_eq!(banc.lire("p").unwrap(), Some(a * b), "{a} × {b}");
    }
}

#[test]
fn la_memoire_se_comporte_comme_un_tableau() {
    for bits in [2u32, 5, 6] {
        let mut m = MemoireEnMarche::nouvelle(bits, 64).unwrap();
        let mut alea = Alea::nouveau(100 + bits as u64);
        let taille = 1usize << bits;
        for cycle in 0..600 {
            let adresse = alea.sous(taille as u64) as usize;
            let d = alea.u64() as u8;
            let ecrire = alea.chance(1, 3);
            let lu = m.cycle(adresse, d, ecrire);
            assert_eq!(
                lu, m.modele[adresse],
                "mémoire {taille} octets, cycle {cycle}, adresse {adresse}"
            );
        }
        // Relecture complète.
        for adresse in 0..taille {
            assert_eq!(m.cycle(adresse, 0, false), m.modele[adresse]);
        }
    }
}

#[test]
fn les_ordinateurs_d_une_ferme_avancent_ensemble() {
    let programme = ordinateur::assembler(ordinateur::programmes::FIBONACCI).unwrap();
    let reglages = Reglages::default();
    let (bib, nom) = grands::ferme(&programme, reglages, 3).unwrap();
    let mut ferme = Banc::nouveau(&bib, &nom).unwrap();
    let mut seule = Machine::nouvelle(&programme, reglages).unwrap();
    // Même remise à zéro que `Machine::remettre_a_zero`.
    let p = reglages.demi_periode;
    ferme.ecrire("raz", 1).unwrap();
    ferme.ecrire("clavier", 0).unwrap();
    ferme.avancer(2 * p);
    ferme.avancer(2 * p + p / 2);
    ferme.ecrire("raz", 0).unwrap();
    for cycle in 0..150 {
        let pc = seule.pc().expect("pc déterminé") as u64;
        assert_eq!(
            ferme.lire("pc").unwrap(),
            Some(pc | pc << 8 | pc << 16),
            "cycle {cycle}"
        );
        seule.cycle(0);
        ferme.avancer(2 * p);
    }
}
