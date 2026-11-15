//! Programmer la mémoire morte de l'ordinateur depuis l'application.
//!
//! La mémoire morte de l'ordinateur miniature est un circuit de portes NAND fabriqué à partir
//! du programme : une ligne par instruction, une colonne par bit. La brique qui la représente
//! garde le texte assembleur dont elle est faite ([`Modele::programme`]). Changer ce texte
//! réassemble le programme et refabrique la brique en NAND, à l'identique de ce que fait
//! `ce_circuit::ordinateur` : la machine exécute alors le nouveau programme, porte par porte.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use ce_circuit::ordinateur::{self, programmes, Reglages, TAILLE_PROGRAMME};

use crate::brique::{Modele, Projet};
use crate::document::Genre;
use crate::importer::importer;

/// Nom du composant de la mémoire morte dans `ce_circuit::ordinateur`.
pub const MEMOIRE_MORTE: &str = "MEMOIRE_MORTE";

/// Les programmes livrés avec l'application : nom et texte.
pub const PROGRAMMES: [(&str, &str); 2] = [
    ("Calculatrice", programmes::CALCULATRICE),
    ("Majuscules", programmes::MAJUSCULES),
];

/// Les briques d'une mémoire morte programmée avec `texte` : la mémoire morte elle-même (en
/// dernier) et les briques qu'elle utilise.
pub fn briques_de_la_memoire(texte: &str) -> Result<(Vec<Modele>, usize), String> {
    let code = ordinateur::assembler(texte).map_err(|e| e.to_string())?;
    if code.is_empty() {
        return Err("le programme ne contient aucune instruction".to_owned());
    }
    let bib = ordinateur::bibliotheque(&code, Reglages::default()).map_err(|e| e.to_string())?;
    let mut modeles = importer(&bib, MEMOIRE_MORTE)?;
    if let Some(rom) = modeles.last_mut() {
        rom.programme = Some(texte.to_owned());
    }
    Ok((modeles, code.len()))
}

/// Le texte du programme d'une mémoire morte livrée, reconnue à son circuit : sert à retrouver
/// le programme des mémoires enregistrées avant que les briques ne le gardent (format 1).
pub fn programme_reconnu(plan: &serde_json::Value) -> Option<&'static str> {
    static CONNUS: OnceLock<Vec<(serde_json::Value, &'static str)>> = OnceLock::new();
    let connus = CONNUS.get_or_init(|| {
        PROGRAMMES
            .iter()
            .filter_map(|(_, texte)| {
                let (modeles, _) = briques_de_la_memoire(texte).ok()?;
                let plan = serde_json::to_value(&modeles.last()?.plan).ok()?;
                Some((plan, *texte))
            })
            .collect()
    });
    connus.iter().find(|(p, _)| p == plan).map(|(_, t)| *t)
}

/// Ce que donne un chargement réussi.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Charge {
    /// Nombre d'instructions du programme.
    pub instructions: usize,
    /// Nombre de portes NAND de la nouvelle mémoire morte.
    pub portes: u64,
}

/// Remplace le programme de la brique de mémoire morte `nom` du projet par `texte` : le
/// programme est assemblé, la mémoire refabriquée en NAND, les briques qui lui manquent
/// ajoutées. Une brique du projet qui porterait le même nom qu'une brique de la mémoire sans
/// en avoir le circuit est laissée intacte : celle de la mémoire prend un autre nom.
pub fn charger(projet: &mut Projet, nom: &str, texte: &str) -> Result<Charge, String> {
    let ancienne = projet
        .modele(nom)
        .ok_or_else(|| format!("la brique « {nom} » n'existe pas"))?;
    let (entrees, sorties) = ancienne.plan.interface();
    let (mut modeles, instructions) = briques_de_la_memoire(texte)?;
    let mut rom = modeles.pop().ok_or("mémoire morte vide")?;
    // Les briques utilisées : reprises si le projet a déjà les mêmes, ajoutées sinon.
    let mut renommees: BTreeMap<String, String> = BTreeMap::new();
    for mut m in modeles {
        renommer_dans(&mut m, &renommees);
        match projet.modele(&m.nom) {
            Some(deja) if deja.plan == m.plan => {}
            Some(_) => {
                let libre = projet.nom_libre(&m.nom);
                renommees.insert(m.nom.clone(), libre.clone());
                m.nom = libre;
                projet.ajouter(m);
            }
            None => {
                projet.ajouter(m);
            }
        }
    }
    renommer_dans(&mut rom, &renommees);
    if rom.plan.interface() != (entrees, sorties) {
        return Err(format!(
            "la brique « {nom} » n'a pas les broches d'une mémoire morte (a sur 8 bits, d sur 16 bits)"
        ));
    }
    let portes = projet.nb_portes(&rom.plan);
    projet.deposer(Some(nom), rom.plan);
    if let Some(i) = projet.index(nom) {
        projet.modeles[i].programme = Some(texte.to_owned());
        projet.modeles[i].description = format!(
            "Mémoire morte : {instructions} instruction{} de 16 bits, en NAND",
            if instructions > 1 { "s" } else { "" }
        );
    }
    projet.revision += 1;
    Ok(Charge {
        instructions,
        portes,
    })
}

fn renommer_dans(m: &mut Modele, renommees: &BTreeMap<String, String>) {
    for e in &mut m.plan.elements {
        if e.genre == Genre::Brique {
            if let Some(nouveau) = renommees.get(&e.modele) {
                e.modele = nouveau.clone();
            }
        }
    }
}

/// Les briques programmables du projet : celles qui gardent leur programme.
pub fn programmables(projet: &Projet) -> Vec<&str> {
    projet
        .modeles
        .iter()
        .filter(|m| m.programme.is_some())
        .map(|m| m.nom.as_str())
        .collect()
}

/// Taille maximale d'un programme, en instructions.
pub const TAILLE_MAXIMALE: usize = TAILLE_PROGRAMME;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apparence::Apparence;
    use crate::document::Genre;
    use crate::etat::Atelier;
    use crate::exemples;

    #[test]
    fn les_exemples_gardent_leur_programme() {
        let p = exemples::ordinateur("o", programmes::CALCULATRICE).unwrap();
        assert_eq!(programmables(&p), [MEMOIRE_MORTE]);
        assert_eq!(
            p.modele(MEMOIRE_MORTE).unwrap().programme.as_deref(),
            Some(programmes::CALCULATRICE)
        );
    }

    #[test]
    fn la_memoire_livree_est_reconnue_a_son_circuit() {
        for (_, texte) in PROGRAMMES {
            let (m, _) = briques_de_la_memoire(texte).unwrap();
            let plan = serde_json::to_value(&m.last().unwrap().plan).unwrap();
            assert_eq!(programme_reconnu(&plan), Some(texte));
        }
        assert_eq!(programme_reconnu(&serde_json::json!({})), None);
    }

    #[test]
    fn une_erreur_d_assemblage_ne_touche_a_rien() {
        let mut p = exemples::ordinateur("o", programmes::CALCULATRICE).unwrap();
        let avant = p.clone();
        let e = charger(&mut p, MEMOIRE_MORTE, "LDI 1\nSAUTE ailleurs\n").unwrap_err();
        assert!(e.contains("ligne 2"), "{e}");
        assert_eq!(p, avant);
        assert!(charger(&mut p, MEMOIRE_MORTE, "; rien\n").is_err());
        assert_eq!(p, avant);
    }

    /// Le test d'acceptation : on écrit un nouveau programme dans l'application, la mémoire
    /// morte est refabriquée en NAND, et l'ordinateur l'exécute porte par porte.
    #[test]
    fn un_nouveau_programme_tourne_dans_l_ordinateur_en_nand() {
        let a = Apparence::default();
        let mut p = exemples::ordinateur("o", programmes::CALCULATRICE).unwrap();
        let texte = "; Écrit OK puis s'arrête.\n        LDI 'O'\n        OUT\n        LDI 'K'\n        OUT\n        HLT\n";
        let charge = charger(&mut p, MEMOIRE_MORTE, texte).unwrap();
        assert_eq!(charge.instructions, 5);
        assert!(charge.portes > 0);
        assert_eq!(
            p.modele(MEMOIRE_MORTE).unwrap().programme.as_deref(),
            Some(texte)
        );
        // Le projet reste lisible tel quel après enregistrement.
        let relu = Projet::de_json(&p.en_json()).unwrap();
        assert_eq!(relu.en_json(), p.en_json());
        let mut at = Atelier::avec(p, &a);
        at.session.avancer_vite(128 * 60);
        let ecran = at
            .doc
            .elements
            .iter()
            .find(|e| e.genre == Genre::Ecran)
            .map(|e| at.session.ecran(e.id).to_owned())
            .unwrap();
        assert_eq!(ecran, "OK");
    }

    #[test]
    fn une_brique_homonyme_du_projet_n_est_pas_ecrasee() {
        let mut p = exemples::ordinateur("o", programmes::CALCULATRICE).unwrap();
        // Une brique « DECODEUR4 » à soi, différente de celle de la mémoire.
        let i = p.index("DECODEUR4").unwrap();
        let mien = p.modeles[i].clone();
        let mut autre = mien.clone();
        autre.plan.elements.pop();
        p.modeles[i] = autre.clone();
        charger(&mut p, MEMOIRE_MORTE, programmes::MAJUSCULES).unwrap();
        assert_eq!(p.modele("DECODEUR4").unwrap().plan, autre.plan);
        let rom = p.modele(MEMOIRE_MORTE).unwrap();
        let utilisees: Vec<&str> = rom
            .plan
            .elements
            .iter()
            .filter(|e| e.genre == Genre::Brique)
            .map(|e| e.modele.as_str())
            .collect();
        assert!(!utilisees.contains(&"DECODEUR4"), "{utilisees:?}");
        // La mémoire utilise une copie renommée, au circuit d'origine.
        assert!(utilisees
            .iter()
            .any(|n| p.modele(n).is_some_and(|m| m.plan == mien.plan)));
    }
}
