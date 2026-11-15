//! Logique à quatre valeurs.
//!
//! | Valeur | Sens |
//! |---|---|
//! | `0` | niveau bas, piloté |
//! | `1` | niveau haut, piloté |
//! | `X` | inconnu : la simulation ne peut pas garantir la valeur (état initial, conflit, entrée flottante lue par une porte) |
//! | `Z` | haute impédance : personne ne pilote le fil |
//!
//! L'**ordre d'information** est : `X ⊑ v` pour toute valeur `v`, et `0`, `1`, `Z`
//! ne sont comparables qu'à eux-mêmes. « `a ⊑ b` » se lit « `b` précise `a` ».
//! Toutes les fonctions de ce module sont *monotones* pour cet ordre : préciser une
//! entrée ne peut que préciser la sortie, jamais la contredire. C'est la base du
//! théorème de sûreté du `X` (voir `docs/semantique.md`).

use std::fmt;

/// Une valeur logique à quatre états.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[repr(u8)]
pub enum Valeur {
    /// Niveau bas.
    Zero = 0,
    /// Niveau haut.
    Un = 1,
    /// Inconnu.
    #[default]
    X = 2,
    /// Haute impédance.
    Z = 3,
}

pub(crate) const V0: u8 = 0;
pub(crate) const V1: u8 = 1;
pub(crate) const VX: u8 = 2;
pub(crate) const VZ: u8 = 3;

impl Valeur {
    /// Les quatre valeurs, dans l'ordre de leur code.
    pub const TOUTES: [Valeur; 4] = [Valeur::Zero, Valeur::Un, Valeur::X, Valeur::Z];

    /// Décode les deux bits de poids faible.
    #[inline]
    pub fn de_code(code: u8) -> Valeur {
        match code & 3 {
            0 => Valeur::Zero,
            1 => Valeur::Un,
            2 => Valeur::X,
            _ => Valeur::Z,
        }
    }

    /// Code sur deux bits.
    #[inline]
    pub fn code(self) -> u8 {
        self as u8
    }

    /// Convertit un booléen.
    #[inline]
    pub fn de_bool(b: bool) -> Valeur {
        if b {
            Valeur::Un
        } else {
            Valeur::Zero
        }
    }

    /// `Some(false)` pour `0`, `Some(true)` pour `1`, `None` pour `X` et `Z`.
    #[inline]
    pub fn en_bool(self) -> Option<bool> {
        match self {
            Valeur::Zero => Some(false),
            Valeur::Un => Some(true),
            _ => None,
        }
    }

    /// Vrai pour `0` et `1`.
    #[inline]
    pub fn est_binaire(self) -> bool {
        matches!(self, Valeur::Zero | Valeur::Un)
    }

    /// Ordre d'information : vrai si `autre` précise `self` (`self ⊑ autre`).
    #[inline]
    pub fn precisee_par(self, autre: Valeur) -> bool {
        self == Valeur::X || self == autre
    }

    /// Caractère d'affichage : `0`, `1`, `x` ou `z`.
    pub fn symbole(self) -> char {
        match self {
            Valeur::Zero => '0',
            Valeur::Un => '1',
            Valeur::X => 'x',
            Valeur::Z => 'z',
        }
    }

    /// Lit un caractère `0`, `1`, `x`/`X` ou `z`/`Z`.
    pub fn de_symbole(c: char) -> Option<Valeur> {
        match c {
            '0' => Some(Valeur::Zero),
            '1' => Some(Valeur::Un),
            'x' | 'X' => Some(Valeur::X),
            'z' | 'Z' => Some(Valeur::Z),
            _ => None,
        }
    }
}

impl fmt::Display for Valeur {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbole())
    }
}

/// Table de la porte NAND, indexée par `a << 2 | b`.
///
/// Une entrée à `0` force la sortie à `1`, même si l'autre entrée est inconnue.
/// Une entrée flottante (`Z`) est lue comme inconnue.
pub(crate) const TABLE_NAND: [u8; 16] = {
    let mut t = [VX; 16];
    let mut a = 0;
    while a < 4 {
        let mut b = 0;
        while b < 4 {
            t[a << 2 | b] = if a == V0 as usize || b == V0 as usize {
                V1
            } else if a == V1 as usize && b == V1 as usize {
                V0
            } else {
                VX
            };
            b += 1;
        }
        a += 1;
    }
    t
};

/// Table du tampon trois états, indexée par `validation << 2 | donnee`.
///
/// Validation à `1` : la sortie recopie la donnée (une donnée flottante donne `X`).
/// Validation à `0` : la sortie est en haute impédance.
/// Validation inconnue : la sortie est inconnue.
pub(crate) const TABLE_TRI: [u8; 16] = {
    let mut t = [VX; 16];
    let mut e = 0;
    while e < 4 {
        let mut d = 0;
        while d < 4 {
            t[e << 2 | d] = if e == V0 as usize {
                VZ
            } else if e == V1 as usize && (d == V0 as usize || d == V1 as usize) {
                d as u8
            } else {
                VX
            };
            d += 1;
        }
        e += 1;
    }
    t
};

/// Porte NAND à quatre valeurs.
#[inline]
pub fn nand(a: Valeur, b: Valeur) -> Valeur {
    Valeur::de_code(TABLE_NAND[(a as usize) << 2 | b as usize])
}

/// Tampon trois états : `validation` commande le passage de `donnee`.
#[inline]
pub fn tri(validation: Valeur, donnee: Valeur) -> Valeur {
    Valeur::de_code(TABLE_TRI[(validation as usize) << 2 | donnee as usize])
}

/// Résolution de deux pilotes branchés sur le même fil.
///
/// `Z` est neutre, deux pilotes d'accord donnent leur valeur commune, et tout
/// désaccord (dont le court-circuit `0` contre `1`) donne `X`.
#[inline]
pub fn resoudre(a: Valeur, b: Valeur) -> Valeur {
    match (a, b) {
        (Valeur::Z, v) | (v, Valeur::Z) => v,
        (u, v) if u == v => u,
        _ => Valeur::X,
    }
}

/// Résolution d'un nombre quelconque de pilotes. Sans pilote, le fil est à `Z`.
pub fn resoudre_tous<I: IntoIterator<Item = Valeur>>(pilotes: I) -> Valeur {
    pilotes.into_iter().fold(Valeur::Z, resoudre)
}

/// Vrai si des pilotes forment un court-circuit franc : au moins un `0` et un `1`.
pub fn court_circuit<I: IntoIterator<Item = Valeur>>(pilotes: I) -> bool {
    let (mut zero, mut un) = (false, false);
    for v in pilotes {
        zero |= v == Valeur::Zero;
        un |= v == Valeur::Un;
    }
    zero && un
}

#[cfg(test)]
mod tests {
    use super::*;
    use Valeur::*;

    #[test]
    fn nand_binaire_est_la_table_classique() {
        assert_eq!(nand(Zero, Zero), Un);
        assert_eq!(nand(Zero, Un), Un);
        assert_eq!(nand(Un, Zero), Un);
        assert_eq!(nand(Un, Un), Zero);
    }

    #[test]
    fn nand_un_zero_force_la_sortie() {
        for v in Valeur::TOUTES {
            assert_eq!(nand(Zero, v), Un);
            assert_eq!(nand(v, Zero), Un);
        }
        assert_eq!(nand(Un, X), X);
        assert_eq!(nand(X, X), X);
        assert_eq!(nand(Z, Un), X);
        assert_eq!(nand(Z, Z), X);
    }

    #[test]
    fn tri_etats() {
        assert_eq!(tri(Un, Zero), Zero);
        assert_eq!(tri(Un, Un), Un);
        assert_eq!(tri(Un, X), X);
        assert_eq!(tri(Un, Z), X);
        for v in Valeur::TOUTES {
            assert_eq!(tri(Zero, v), Z);
            assert_eq!(tri(X, v), X);
            assert_eq!(tri(Z, v), X);
        }
    }

    #[test]
    fn resolution_table() {
        assert_eq!(resoudre(Z, Z), Z);
        assert_eq!(resoudre(Z, Un), Un);
        assert_eq!(resoudre(Zero, Z), Zero);
        assert_eq!(resoudre(Zero, Zero), Zero);
        assert_eq!(resoudre(Zero, Un), X);
        assert_eq!(resoudre(X, Un), X);
        assert_eq!(resoudre(X, Z), X);
        assert_eq!(resoudre_tous([]), Z);
        assert_eq!(resoudre_tous([Z, Un, Z, Un]), Un);
        assert!(court_circuit([Z, Un, Zero]));
        assert!(!court_circuit([X, Un, Z]));
    }
}
