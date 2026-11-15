//! Un ordinateur complet miniature, construit uniquement en portes NAND.
//!
//! C'est le test d'acceptation du projet : horloge, processeur, mémoire vive, mémoire morte,
//! sortie vers un écran et entrée depuis un clavier, le tout décrit par des composants
//! hiérarchiques de la bibliothèque et simulé porte par porte.
//!
//! ## La machine
//!
//! - Mots de 8 bits. Un accumulateur `A`, un compteur de programme `PC` de 8 bits, deux
//!   drapeaux `Z` (le dernier résultat rangé dans `A` vaut zéro) et `C` (retenue).
//! - Programme en mémoire morte : au plus 256 instructions de 12 bits (4 bits de code,
//!   8 bits d'opérande). Une instruction par cycle d'horloge.
//! - Mémoire vive de `2^n` octets (16 par défaut), adressée par les bits de poids faible de
//!   l'opérande. Son contenu est inconnu (`X`) tant qu'on n'y a pas écrit.
//! - Écran : quand `ecran_ecrit` monte, le caractère présent sur `ecran` est affiché.
//! - Clavier : `clavier` porte le code de la touche enfoncée, 0 s'il n'y en a pas.
//! - `raz` à 1 pendant un front d'horloge remet `PC`, `A`, `Z` et `C` à zéro.
//!
//! ## Les instructions
//!
//! | Code | Nom | Effet |
//! |---|---|---|
//! | 0 | `NOP` | rien |
//! | 1 | `STA m` | `M[m] ← A` |
//! | 2 | `IN` | `A ← clavier`, `Z` |
//! | 3 | `OUT` | affiche `A` à l'écran |
//! | 4 | `LDA m` | `A ← M[m]`, `Z` |
//! | 5 | `ADD m` | `A ← A + M[m]`, `Z`, `C` = retenue |
//! | 6 | `SUB m` | `A ← A − M[m]`, `Z`, `C` = 1 s'il n'y a pas d'emprunt |
//! | 7 | `NAND m` | `A ← non (A et M[m])`, `Z` |
//! | 8 | `LDI n` | `A ← n`, `Z` |
//! | 9 | `ADDI n` | `A ← A + n`, `Z`, `C` |
//! | 10 | `SUBI n` | `A ← A − n`, `Z`, `C` |
//! | 11 | `NANDI n` | `A ← non (A et n)`, `Z` |
//! | 12 | `JMP a` | `PC ← a` |
//! | 13 | `JZ a` | `PC ← a` si `Z` |
//! | 14 | `JC a` | `PC ← a` si `C` |
//! | 15 | `HLT` | arrêt : `PC` ne bouge plus, `arret` vaut 1 |
//!
//! ## Les pièces
//!
//! - [`materiel`] : la bibliothèque de composants (`ORDINATEUR` et tout ce qu'il contient).
//! - [`assembleur`] : du texte vers les mots de la mémoire morte.
//! - [`emulateur`] : le modèle de référence, quelques lignes de Rust qui disent ce que chaque
//!   instruction doit faire. Les tests comparent la machine en NAND à ce modèle, cycle par cycle.
//! - [`machine`] : la machine en marche, avec son écran et son clavier.
//! - [`programmes`] : des programmes d'exemple en assembleur.

pub mod assembleur;
pub mod emulateur;
pub mod machine;
pub mod materiel;
pub mod programmes;

pub use assembleur::{assembler, ErreurAssemblage};
pub use emulateur::Emulateur;
pub use machine::Machine;
pub use materiel::{bibliotheque, Reglages};

/// Les seize instructions, dans l'ordre de leur code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Op {
    /// Rien.
    Nop = 0,
    /// `M[m] ← A`.
    Sta = 1,
    /// `A ← clavier`.
    In = 2,
    /// Affiche `A`.
    Out = 3,
    /// `A ← M[m]`.
    Lda = 4,
    /// `A ← A + M[m]`.
    Add = 5,
    /// `A ← A − M[m]`.
    Sub = 6,
    /// `A ← non (A et M[m])`.
    Nand = 7,
    /// `A ← n`.
    Ldi = 8,
    /// `A ← A + n`.
    Addi = 9,
    /// `A ← A − n`.
    Subi = 10,
    /// `A ← non (A et n)`.
    Nandi = 11,
    /// `PC ← a`.
    Jmp = 12,
    /// `PC ← a` si `Z`.
    Jz = 13,
    /// `PC ← a` si `C`.
    Jc = 14,
    /// Arrêt.
    Hlt = 15,
}

impl Op {
    /// Toutes les instructions, dans l'ordre de leur code.
    pub const TOUTES: [Op; 16] = [
        Op::Nop,
        Op::Sta,
        Op::In,
        Op::Out,
        Op::Lda,
        Op::Add,
        Op::Sub,
        Op::Nand,
        Op::Ldi,
        Op::Addi,
        Op::Subi,
        Op::Nandi,
        Op::Jmp,
        Op::Jz,
        Op::Jc,
        Op::Hlt,
    ];

    /// L'instruction de code `code` (seuls les 4 bits de poids faible comptent).
    pub fn de_code(code: u8) -> Op {
        Op::TOUTES[usize::from(code & 15)]
    }

    /// Nom de l'instruction en assembleur.
    pub fn nom(self) -> &'static str {
        [
            "NOP", "STA", "IN", "OUT", "LDA", "ADD", "SUB", "NAND", "LDI", "ADDI", "SUBI", "NANDI",
            "JMP", "JZ", "JC", "HLT",
        ][self as usize]
    }

    /// Vrai si l'instruction prend un opérande.
    pub fn a_un_operande(self) -> bool {
        !matches!(self, Op::Nop | Op::In | Op::Out | Op::Hlt)
    }

    /// Le mot de 12 bits de cette instruction avec cet opérande.
    pub fn mot(self, operande: u8) -> u16 {
        (self as u16) << 8 | u16::from(operande)
    }
}

/// Découpe un mot de mémoire morte en instruction et opérande.
pub fn decoder(mot: u16) -> (Op, u8) {
    (Op::de_code((mot >> 8) as u8), (mot & 0xFF) as u8)
}

/// Nombre maximal d'instructions d'un programme.
pub const TAILLE_PROGRAMME: usize = 256;
