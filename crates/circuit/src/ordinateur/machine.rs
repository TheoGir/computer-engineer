//! La machine en marche : l'ordinateur en NAND dans le simulateur, son écran et son clavier.

use ce_moteur::{NetId, Valeur};

use crate::banc::Banc;
use crate::erreur::Erreur;

use super::materiel::{bibliotheque, Reglages};

/// L'ordinateur miniature en cours de simulation.
///
/// Chaque appel à [`Machine::cycle`] fait passer un front montant de l'horloge : une
/// instruction est exécutée, porte par porte.
pub struct Machine {
    /// Le banc : le circuit mis à plat et son simulateur.
    pub banc: Banc,
    /// Les réglages de fabrication.
    pub reglages: Reglages,
    /// Nombre de cycles exécutés depuis la remise à zéro.
    pub cycles: u64,
    /// Tout ce que la machine a écrit à l'écran.
    pub ecran: Vec<u8>,
    /// Les signaux lus et écrits à chaque cycle, cherchés une fois pour toutes.
    signaux: Signaux,
}

struct Signaux {
    ecran_ecrit: NetId,
    ecran: Vec<NetId>,
    /// Première entrée externe du clavier, et nombre de bits.
    clavier: (usize, usize),
}

impl Machine {
    /// Fabrique l'ordinateur qui exécute `programme`, le met sous tension et le remet à zéro.
    pub fn nouvelle(programme: &[u16], reglages: Reglages) -> Result<Machine, Erreur> {
        Machine::avec_options(programme, reglages, ce_moteur::Options::default())
    }

    /// Comme [`Machine::nouvelle`], en choisissant les réglages du simulateur.
    pub fn avec_options(
        programme: &[u16],
        reglages: Reglages,
        options: ce_moteur::Options,
    ) -> Result<Machine, Erreur> {
        let bib = bibliotheque(programme, reglages)?;
        let mut banc = Banc::nouveau(&bib, "ORDINATEUR")?;
        banc.regler(options)?;
        let signaux = Signaux {
            ecran_ecrit: banc.circuit.nets("ecran_ecrit")?[0],
            ecran: banc.circuit.nets("ecran")?,
            clavier: banc.entree("clavier")?,
        };
        let mut m = Machine {
            banc,
            reglages,
            cycles: 0,
            ecran: Vec::new(),
            signaux,
        };
        m.remettre_a_zero();
        Ok(m)
    }

    fn ecrire(&mut self, broche: &str, valeur: u64) {
        self.banc
            .ecrire(broche, valeur)
            .expect("l'ordinateur a toujours cette broche");
    }

    fn lire(&self, chemin: &str) -> Option<u64> {
        self.banc
            .lire(chemin)
            .expect("l'ordinateur a toujours ce signal")
    }

    /// Appuie sur le bouton de remise à zéro pendant un front d'horloge, puis le relâche.
    /// La machine s'arrête au milieu d'un niveau bas de l'horloge, prête à exécuter la
    /// première instruction. L'écran est effacé ; la mémoire vive garde son contenu.
    pub fn remettre_a_zero(&mut self) {
        let p = self.reglages.demi_periode;
        self.ecrire("raz", 1);
        self.ecrire("clavier", 0);
        // On se place juste après un front descendant, puis on laisse passer un front montant.
        let t = self.banc.sim.temps();
        let phase = t % (2 * p);
        self.banc.avancer(2 * p - phase);
        self.banc.avancer(2 * p + p / 2);
        self.ecrire("raz", 0);
        self.cycles = 0;
        self.ecran.clear();
    }

    /// Exécute une instruction : `clavier` est le code de la touche enfoncée pendant ce cycle.
    /// Renvoie le caractère envoyé à l'écran, s'il y en a un.
    pub fn cycle(&mut self, clavier: u8) -> Option<u8> {
        // Milieu du niveau bas : si l'instruction en cours est OUT, l'écran est en train
        // de recevoir le contenu de l'accumulateur.
        let sim = &mut self.banc.sim;
        let ecrit = sim.valeur(self.signaux.ecran_ecrit) == Valeur::Un;
        let sorti = if ecrit {
            // Le caractère, s'il est déterminé bit par bit.
            self.signaux
                .ecran
                .iter()
                .enumerate()
                .try_fold(0u8, |c, (k, &n)| {
                    sim.valeur(n).en_bool().map(|b| c | u8::from(b) << k)
                })
        } else {
            None
        };
        self.ecran.extend(sorti);
        let (premiere, largeur) = self.signaux.clavier;
        for k in 0..largeur {
            let bit = k < 8 && (clavier >> k) & 1 == 1;
            sim.fixer_entree(premiere + k, Valeur::de_bool(bit));
        }
        self.banc.avancer(2 * self.reglages.demi_periode);
        self.cycles += 1;
        sorti
    }

    /// Exécute au plus `max` instructions, ou jusqu'à l'arrêt. `clavier(i)` donne la touche
    /// enfoncée pendant l'instruction numéro `i`. Renvoie ce qui a été écrit à l'écran.
    pub fn executer(&mut self, max: usize, mut clavier: impl FnMut(usize) -> u8) -> Vec<u8> {
        let mut ecran = Vec::new();
        for i in 0..max {
            if self.arret() == Some(true) {
                break;
            }
            ecran.extend(self.cycle(clavier(i)));
        }
        ecran
    }

    /// Le compteur de programme, ou `None` s'il n'est pas déterminé.
    pub fn pc(&self) -> Option<u8> {
        self.lire("pc").map(|v| v as u8)
    }

    /// L'accumulateur.
    pub fn a(&self) -> Option<u8> {
        self.lire("cpu.acc.q").map(|v| v as u8)
    }

    /// Le drapeau `Z`.
    pub fn z(&self) -> Option<bool> {
        self.lire("cpu.rz.q").map(|v| v == 1)
    }

    /// Le drapeau `C`.
    pub fn c(&self) -> Option<bool> {
        self.lire("cpu.rc.q").map(|v| v == 1)
    }

    /// Vrai si la machine est arrêtée sur `HLT`.
    pub fn arret(&self) -> Option<bool> {
        self.lire("arret").map(|v| v == 1)
    }

    /// Le contenu d'une case de mémoire vive, ou `None` si elle n'a jamais été écrite.
    pub fn ram(&self, adresse: usize) -> Option<u8> {
        let mut chemin = String::from("ram");
        for bit in (0..self.reglages.bits_ram).rev() {
            chemin.push_str(if adresse >> bit & 1 == 1 {
                ".haut"
            } else {
                ".bas"
            });
        }
        chemin.push_str(".q");
        self.lire(&chemin).map(|v| v as u8)
    }

    /// Nombre de portes NAND de la machine.
    pub fn nb_portes(&self) -> usize {
        self.banc.circuit.nb_portes()
    }
}
