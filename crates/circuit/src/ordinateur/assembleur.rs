//! L'assembleur : du texte vers les mots de la mémoire morte.
//!
//! ```text
//! ; Compte de 0 à 9 et l'affiche.
//! i = 0                 ; un nom pour une case de mémoire (ou une valeur)
//!         LDI '0'
//! encore: STA i
//!         OUT
//!         SUBI '9'
//!         JZ fin
//!         LDA i
//!         ADDI 1
//!         JMP encore
//! fin:    HLT
//! ```
//!
//! Une ligne contient, dans l'ordre et tous facultatifs : une étiquette suivie de `:`, une
//! instruction avec son opérande, un commentaire après `;`. Une ligne `nom = valeur` définit
//! une constante. Un opérande est un nombre (`42`, `0x2A`, `0b101010`), un caractère entre
//! apostrophes (`'A'`), une étiquette ou une constante. Les noms d'instructions s'écrivent en
//! majuscules ou en minuscules.

use std::collections::BTreeMap;
use std::fmt;

use super::{Op, TAILLE_PROGRAMME};

/// Une erreur d'assemblage, avec le numéro de la ligne fautive (à partir de 1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErreurAssemblage {
    /// Numéro de ligne, à partir de 1.
    pub ligne: usize,
    /// Ce qui ne va pas.
    pub message: String,
}

impl fmt::Display for ErreurAssemblage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ligne {} : {}", self.ligne, self.message)
    }
}

impl std::error::Error for ErreurAssemblage {}

fn est_un_nom(s: &str) -> bool {
    let mut c = s.chars();
    c.next().is_some_and(|p| p.is_alphabetic() || p == '_')
        && c.all(|x| x.is_alphanumeric() || x == '_')
}

fn nombre(s: &str) -> Option<i64> {
    let (negatif, s) = match s.strip_prefix('-') {
        Some(reste) => (true, reste),
        None => (false, s),
    };
    let v = if let Some(h) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        i64::from_str_radix(h, 16).ok()?
    } else if let Some(b) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        i64::from_str_radix(b, 2).ok()?
    } else if s.chars().all(|c| c.is_ascii_digit()) && !s.is_empty() {
        s.parse().ok()?
    } else {
        return None;
    };
    Some(if negatif { -v } else { v })
}

/// Coupe le commentaire d'une ligne, sans se laisser tromper par `';'`.
fn sans_commentaire(ligne: &str) -> &str {
    let octets = ligne.as_bytes();
    let mut i = 0;
    while i < octets.len() {
        match octets[i] {
            // Un caractère entre apostrophes : on saute jusqu'à l'apostrophe fermante.
            b'\'' => match ligne[i + 1..].char_indices().nth(1) {
                Some((j, '\'')) => i += j + 2,
                _ => i += 1,
            },
            b';' => return &ligne[..i],
            _ => i += 1,
        }
    }
    ligne
}

/// Assemble un programme. Renvoie les mots de la mémoire morte, un par instruction.
pub fn assembler(texte: &str) -> Result<Vec<u16>, ErreurAssemblage> {
    let erreur = |ligne: usize, message: String| ErreurAssemblage {
        ligne: ligne + 1,
        message,
    };
    // Première passe : étiquettes, constantes, liste des instructions.
    let mut noms: BTreeMap<String, i64> = BTreeMap::new();
    let mut instructions: Vec<(usize, Op, Option<String>)> = Vec::new();
    let mut constantes: Vec<(usize, String, String)> = Vec::new();
    for (no, brute) in texte.lines().enumerate() {
        let mut ligne = sans_commentaire(brute).trim();
        if let Some((nom, valeur)) = ligne.split_once('=') {
            let (nom, valeur) = (nom.trim(), valeur.trim());
            if est_un_nom(nom) {
                constantes.push((no, nom.to_owned(), valeur.to_owned()));
                continue;
            }
        }
        // Des étiquettes : un nom suivi de deux-points (mais pas le caractère ':').
        while let Some((avant, apres)) = ligne.split_once(':') {
            let nom = avant.trim();
            if !est_un_nom(nom) {
                break;
            }
            if noms
                .insert(nom.to_owned(), instructions.len() as i64)
                .is_some()
            {
                return Err(erreur(no, format!("« {nom} » est défini deux fois")));
            }
            ligne = apres.trim();
        }
        if ligne.is_empty() {
            continue;
        }
        let (mot, reste) = match ligne.split_once(char::is_whitespace) {
            Some((m, r)) => (m, r.trim()),
            None => (ligne, ""),
        };
        let Some(op) = Op::TOUTES
            .into_iter()
            .find(|op| op.nom().eq_ignore_ascii_case(mot))
        else {
            return Err(erreur(no, format!("instruction inconnue « {mot} »")));
        };
        match (op.a_un_operande(), reste.is_empty()) {
            (true, true) => {
                return Err(erreur(no, format!("{} attend un opérande", op.nom())));
            }
            (false, false) => {
                return Err(erreur(no, format!("{} ne prend pas d'opérande", op.nom())));
            }
            _ => {}
        }
        instructions.push((no, op, (!reste.is_empty()).then(|| reste.to_owned())));
    }
    if instructions.len() > TAILLE_PROGRAMME {
        return Err(ErreurAssemblage {
            ligne: instructions[TAILLE_PROGRAMME].0 + 1,
            message: format!(
                "le programme a {} instructions, la mémoire morte en contient au plus {TAILLE_PROGRAMME}",
                instructions.len()
            ),
        });
    }

    let valeur = |noms: &BTreeMap<String, i64>,
                  no: usize,
                  s: &str|
     -> Result<i64, ErreurAssemblage> {
        let mut c = s.chars();
        if let (Some('\''), Some(x), Some('\''), None) = (c.next(), c.next(), c.next(), c.next()) {
            return u8::try_from(u32::from(x))
                .map(i64::from)
                .map_err(|_| erreur(no, format!("le caractère « {x} » ne tient pas sur 8 bits")));
        }
        if let Some(v) = nombre(s) {
            return Ok(v);
        }
        if est_un_nom(s) {
            return noms
                .get(s)
                .copied()
                .ok_or_else(|| erreur(no, format!("« {s} » n'est défini nulle part")));
        }
        Err(erreur(no, format!("opérande illisible « {s} »")))
    };
    // Les constantes peuvent citer des étiquettes et les constantes écrites plus haut.
    for (no, nom, texte) in constantes {
        let v = valeur(&noms, no, &texte)?;
        if noms.insert(nom.clone(), v).is_some() {
            return Err(erreur(no, format!("« {nom} » est défini deux fois")));
        }
    }

    // Seconde passe : les mots.
    let mut mots = Vec::with_capacity(instructions.len());
    for (no, op, operande) in instructions {
        let n = match operande {
            None => 0,
            Some(texte) => {
                let v = valeur(&noms, no, &texte)?;
                if !(-128..=255).contains(&v) {
                    return Err(erreur(no, format!("{v} ne tient pas sur 8 bits")));
                }
                v as u8
            }
        };
        mots.push(op.mot(n));
    }
    Ok(mots)
}

/// Réécrit un programme assemblé en texte, une instruction par ligne (pour l'afficher).
pub fn desassembler(programme: &[u16]) -> String {
    let mut texte = String::new();
    for (i, mot) in programme.iter().enumerate() {
        let (op, n) = super::decoder(*mot);
        if op.a_un_operande() {
            texte.push_str(&format!("{i:3}  {} {n}\n", op.nom()));
        } else {
            texte.push_str(&format!("{i:3}  {}\n", op.nom()));
        }
    }
    texte
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_programme_avec_etiquettes_constantes_et_caracteres() {
        let p = assembler(
            "; compte\n i = 3\n dix = 0x0A\n  LDI '0'\nencore: STA i ; range\n out\n SUBI ';'\n JZ fin\n JMP encore\nfin: HLT\n",
        )
        .unwrap();
        assert_eq!(
            p,
            [
                Op::Ldi.mot(b'0'),
                Op::Sta.mot(3),
                Op::Out.mot(0),
                Op::Subi.mot(b';'),
                Op::Jz.mot(6),
                Op::Jmp.mot(1),
                Op::Hlt.mot(0),
            ]
        );
        assert_eq!(assembler("LDI -1").unwrap(), [Op::Ldi.mot(255)]);
        assert_eq!(assembler("LDI 0b101").unwrap(), [Op::Ldi.mot(5)]);
        assert_eq!(assembler("a: b: JMP b").unwrap(), [Op::Jmp.mot(0)]);
        assert_eq!(assembler("LDI ':'").unwrap(), [Op::Ldi.mot(b':')]);
        assert_eq!(assembler("LDI '=' ; égal").unwrap(), [Op::Ldi.mot(b'=')]);
        assert_eq!(assembler("LDI ';'").unwrap(), [Op::Ldi.mot(b';')]);
        assert_eq!(assembler("LDI '\''").unwrap(), [Op::Ldi.mot(b'\'')]);
    }

    #[test]
    fn les_erreurs_donnent_la_ligne() {
        let e = |t: &str| assembler(t).unwrap_err();
        assert_eq!(e("NOP\nFOO 1").ligne, 2);
        assert!(e("FOO 1").message.contains("inconnue"));
        assert!(e("LDI").message.contains("attend"));
        assert!(e("HLT 3").message.contains("ne prend pas"));
        assert!(e("LDI 256").message.contains("8 bits"));
        assert!(e("JMP nulle_part").message.contains("défini nulle part"));
        assert!(e("a: NOP\na: NOP").message.contains("deux fois"));
        assert!(e("LDI 1+1").message.contains("illisible"));
        let long = "NOP\n".repeat(257);
        assert_eq!(e(&long).ligne, 257);
    }

    #[test]
    fn desassembler_puis_assembler_rend_le_meme_programme() {
        let p: Vec<u16> = (0..16u16).map(|k| k << 8 | (k * 13 + 1)).collect();
        let texte: String = desassembler(&p)
            .lines()
            .map(|l| format!("{}\n", &l[5..]))
            .collect();
        let relu = assembler(&texte).unwrap();
        for (a, b) in p.iter().zip(&relu) {
            let (op, n) = super::super::decoder(*a);
            let attendu = if op.a_un_operande() {
                op.mot(n)
            } else {
                op.mot(0)
            };
            assert_eq!(*b, attendu);
        }
    }
}
