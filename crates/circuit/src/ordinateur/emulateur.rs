//! Le modèle de référence de la machine : ce que chaque instruction doit faire.
//!
//! Il ne connaît ni portes ni horloge. Les tests exécutent le même programme sur ce modèle et
//! sur la machine en NAND, et comparent les deux états après chaque cycle.

use super::{decoder, Op};

/// L'état de la machine vu par le programmeur.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Emulateur {
    /// L'accumulateur.
    pub a: u8,
    /// Le compteur de programme.
    pub pc: u8,
    /// Drapeau « le dernier résultat rangé dans `A` vaut zéro ».
    pub z: bool,
    /// Drapeau de retenue.
    pub c: bool,
    /// La mémoire vive ; `None` : case jamais écrite, contenu inconnu.
    pub ram: Vec<Option<u8>>,
    /// Vrai si le programme a lu une case de mémoire jamais écrite : la suite n'a plus de sens.
    pub indefini: bool,
}

impl Emulateur {
    /// La machine juste après la remise à zéro, avec `2^bits_ram` octets de mémoire vive.
    pub fn nouveau(bits_ram: u32) -> Self {
        Emulateur {
            a: 0,
            pc: 0,
            z: false,
            c: false,
            ram: vec![None; 1 << bits_ram],
            indefini: false,
        }
    }

    fn ranger(&mut self, valeur: u8) {
        self.a = valeur;
        self.z = valeur == 0;
    }

    /// Exécute une instruction. `clavier` est le code de la touche enfoncée.
    /// Renvoie le caractère envoyé à l'écran, s'il y en a un.
    pub fn pas(&mut self, programme: &[u16], clavier: u8) -> Option<u8> {
        let (op, n) = decoder(programme.get(usize::from(self.pc)).copied().unwrap_or(0));
        let adresse = usize::from(n) & (self.ram.len() - 1);
        let mut suivant = self.pc.wrapping_add(1);
        let mut ecran = None;
        // Second opérande de l'unité de calcul : une case de mémoire ou la valeur immédiate.
        let b = match op {
            Op::Lda | Op::Add | Op::Sub | Op::Nand => match self.ram[adresse] {
                Some(v) => v,
                None => {
                    self.indefini = true;
                    0
                }
            },
            _ => n,
        };
        match op {
            Op::Nop => {}
            Op::Sta => self.ram[adresse] = Some(self.a),
            Op::In => self.ranger(clavier),
            Op::Out => ecran = Some(self.a),
            Op::Lda | Op::Ldi => self.ranger(b),
            Op::Add | Op::Addi => {
                let (somme, retenue) = self.a.overflowing_add(b);
                self.c = retenue;
                self.ranger(somme);
            }
            Op::Sub | Op::Subi => {
                self.c = self.a >= b;
                self.ranger(self.a.wrapping_sub(b));
            }
            Op::Nand | Op::Nandi => self.ranger(!(self.a & b)),
            Op::Jmp => suivant = n,
            Op::Jz => {
                if self.z {
                    suivant = n;
                }
            }
            Op::Jc => {
                if self.c {
                    suivant = n;
                }
            }
            Op::Hlt => suivant = self.pc,
        }
        self.pc = suivant;
        ecran
    }

    /// Vrai si la machine est arrêtée : l'instruction désignée par `PC` est `HLT`.
    pub fn arret(&self, programme: &[u16]) -> bool {
        decoder(programme.get(usize::from(self.pc)).copied().unwrap_or(0)).0 == Op::Hlt
    }

    /// Exécute au plus `max` instructions, ou jusqu'à l'arrêt. `clavier(i)` donne la touche
    /// enfoncée pendant l'instruction numéro `i`. Renvoie tout ce qui a été écrit à l'écran.
    pub fn executer(
        &mut self,
        programme: &[u16],
        max: usize,
        mut clavier: impl FnMut(usize) -> u8,
    ) -> Vec<u8> {
        let mut ecran = Vec::new();
        for i in 0..max {
            if self.arret(programme) {
                break;
            }
            ecran.extend(self.pas(programme, clavier(i)));
        }
        ecran
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_operations_et_les_drapeaux() {
        let p = [
            Op::Ldi.mot(200),
            Op::Sta.mot(0),
            Op::Addi.mot(100), // 300 : retenue, reste 44
            Op::Sub.mot(0),    // 44 − 200 : emprunt
            Op::Nandi.mot(0xFF),
            Op::Subi.mot(99), // non(100) = 155 ; 155 − 99 = 56
            Op::Subi.mot(56),
            Op::Hlt.mot(0),
        ];
        let mut e = Emulateur::nouveau(4);
        e.pas(&p, 0);
        e.pas(&p, 0);
        assert_eq!(e.ram[0], Some(200));
        e.pas(&p, 0);
        assert_eq!((e.a, e.c, e.z), (44, true, false));
        e.pas(&p, 0);
        assert_eq!((e.a, e.c), (100, false));
        e.pas(&p, 0);
        assert_eq!(e.a, 155);
        e.pas(&p, 0);
        assert_eq!((e.a, e.c), (56, true));
        e.pas(&p, 0);
        assert_eq!((e.a, e.c, e.z), (0, true, true));
        assert!(e.arret(&p));
        e.pas(&p, 0);
        assert_eq!(e.pc, 7, "HLT ne fait plus avancer le compteur");
        assert!(!e.indefini);
    }

    #[test]
    fn lire_une_case_jamais_ecrite_est_signale() {
        let mut e = Emulateur::nouveau(4);
        e.pas(&[Op::Lda.mot(3)], 0);
        assert!(e.indefini);
    }

    #[test]
    fn au_dela_du_programme_la_memoire_morte_rend_des_nop() {
        let mut e = Emulateur::nouveau(4);
        for _ in 0..300 {
            e.pas(&[Op::Ldi.mot(1)], 0);
        }
        assert_eq!(e.pc, 300u16 as u8);
        assert_eq!(e.a, 1);
    }
}
