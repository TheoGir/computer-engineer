//! Circuit « à plat » : des portes primitives reliées par des nets.
//!
//! Un *net* est un fil électrique : tous les points qui y sont reliés ont la même
//! valeur. Un net peut avoir plusieurs *pilotes* (sorties de portes, entrées
//! externes, constantes, horloges) ; sa valeur est leur résolution
//! ([`crate::valeur::resoudre`]).

use crate::valeur::Valeur;
use std::fmt;

/// Identifiant d'un net, de `0` à `nb_nets - 1`.
pub type NetId = u32;

/// Les deux portes primitives du moteur.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum GenrePorte {
    /// NAND à deux entrées : `e0` et `e1` sont les deux entrées.
    Nand = 0,
    /// Tampon trois états : `e0` est la validation, `e1` la donnée.
    Tri = 1,
}

/// Une porte primitive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Porte {
    /// Nature de la porte.
    pub genre: GenrePorte,
    /// Première entrée (validation pour un tampon trois états).
    pub e0: NetId,
    /// Seconde entrée (donnée pour un tampon trois états).
    pub e1: NetId,
    /// Net piloté par la sortie.
    pub s: NetId,
    /// Délai de propagation, en unités de temps (au moins 1).
    pub delai: u16,
}

/// Une horloge : pilote son net à `0` jusqu'à `demi_periode`, puis bascule toutes
/// les `demi_periode` unités de temps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Horloge {
    /// Net piloté.
    pub net: NetId,
    /// Durée de chaque niveau (au moins 1).
    pub demi_periode: u64,
}

impl Horloge {
    /// Niveau de l'horloge à l'instant `t`.
    pub fn niveau(&self, t: u64) -> Valeur {
        Valeur::de_bool((t / self.demi_periode) % 2 == 1)
    }
}

/// Un circuit à plat.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Netlist {
    /// Nombre de nets.
    pub nb_nets: u32,
    /// Portes primitives.
    pub portes: Vec<Porte>,
    /// Pilotes constants.
    pub constantes: Vec<(NetId, Valeur)>,
    /// Entrées externes : l'entrée numéro `i` pilote le net `entrees[i]`.
    pub entrees: Vec<NetId>,
    /// Horloges.
    pub horloges: Vec<Horloge>,
    /// Indications facultatives pour le simulateur : des plages de portes consécutives
    /// qui forment chacune une unité, typiquement un registre. Elles doivent être
    /// croissantes et disjointes. Elles ne changent **aucune** valeur calculée : le
    /// simulateur s'en sert seulement pour mettre en sommeil les unités qui ne font que
    /// répéter le même cycle au rythme de l'horloge.
    pub regions: Vec<std::ops::Range<u32>>,
}

/// Erreur de construction d'une netlist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErreurNetlist {
    /// Un net référencé n'existe pas.
    NetInconnu { net: NetId, nb_nets: u32 },
    /// Une porte a un délai nul : le modèle exige au moins une unité de temps.
    DelaiNul { porte: usize },
    /// Une horloge a une demi-période nulle.
    DemiPeriodeNulle { horloge: usize },
    /// Trop de portes pour des index sur 32 bits.
    TropDePortes,
    /// Une région est vide, sort de la liste des portes ou chevauche la précédente.
    RegionInvalide { region: usize },
}

impl fmt::Display for ErreurNetlist {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErreurNetlist::NetInconnu { net, nb_nets } => {
                write!(f, "net {net} inconnu (le circuit en compte {nb_nets})")
            }
            ErreurNetlist::DelaiNul { porte } => {
                write!(f, "la porte {porte} a un délai nul, il faut au moins 1")
            }
            ErreurNetlist::DemiPeriodeNulle { horloge } => {
                write!(f, "l'horloge {horloge} a une demi-période nulle")
            }
            ErreurNetlist::TropDePortes => write!(f, "trop de portes (plus de 2^31)"),
            ErreurNetlist::RegionInvalide { region } => write!(
                f,
                "la région {region} est vide, hors des portes ou chevauche la précédente"
            ),
        }
    }
}

impl std::error::Error for ErreurNetlist {}

impl Netlist {
    /// Netlist vide.
    pub fn nouvelle() -> Self {
        Self::default()
    }

    /// Crée un net et renvoie son identifiant.
    pub fn net(&mut self) -> NetId {
        let id = self.nb_nets;
        self.nb_nets += 1;
        id
    }

    /// Crée `n` nets.
    pub fn nets(&mut self, n: usize) -> Vec<NetId> {
        (0..n).map(|_| self.net()).collect()
    }

    /// Ajoute une porte NAND et renvoie son index.
    pub fn nand(&mut self, a: NetId, b: NetId, s: NetId, delai: u16) -> usize {
        self.portes.push(Porte {
            genre: GenrePorte::Nand,
            e0: a,
            e1: b,
            s,
            delai,
        });
        self.portes.len() - 1
    }

    /// Ajoute un tampon trois états et renvoie son index.
    pub fn tri(&mut self, validation: NetId, donnee: NetId, s: NetId, delai: u16) -> usize {
        self.portes.push(Porte {
            genre: GenrePorte::Tri,
            e0: validation,
            e1: donnee,
            s,
            delai,
        });
        self.portes.len() - 1
    }

    /// Déclare une entrée externe sur `net` et renvoie son numéro.
    pub fn entree(&mut self, net: NetId) -> usize {
        self.entrees.push(net);
        self.entrees.len() - 1
    }

    /// Ajoute un pilote constant.
    pub fn constante(&mut self, net: NetId, valeur: Valeur) {
        self.constantes.push((net, valeur));
    }

    /// Ajoute une horloge et renvoie son index.
    pub fn horloge(&mut self, net: NetId, demi_periode: u64) -> usize {
        self.horloges.push(Horloge { net, demi_periode });
        self.horloges.len() - 1
    }

    /// Déclare que les portes `plage` forment une unité (voir [`Netlist::regions`]).
    pub fn region(&mut self, plage: std::ops::Range<u32>) {
        self.regions.push(plage);
    }

    /// Vérifie la cohérence de la netlist.
    pub fn valider(&self) -> Result<(), ErreurNetlist> {
        let verifier = |net: NetId| {
            if net < self.nb_nets {
                Ok(())
            } else {
                Err(ErreurNetlist::NetInconnu {
                    net,
                    nb_nets: self.nb_nets,
                })
            }
        };
        if self.portes.len() >= (1 << 31) {
            return Err(ErreurNetlist::TropDePortes);
        }
        for (i, p) in self.portes.iter().enumerate() {
            verifier(p.e0)?;
            verifier(p.e1)?;
            verifier(p.s)?;
            if p.delai == 0 {
                return Err(ErreurNetlist::DelaiNul { porte: i });
            }
        }
        for &(net, _) in &self.constantes {
            verifier(net)?;
        }
        for &net in &self.entrees {
            verifier(net)?;
        }
        for (i, h) in self.horloges.iter().enumerate() {
            verifier(h.net)?;
            if h.demi_periode == 0 {
                return Err(ErreurNetlist::DemiPeriodeNulle { horloge: i });
            }
        }
        let mut fin_precedente = 0u32;
        for (i, r) in self.regions.iter().enumerate() {
            if r.start >= r.end || r.start < fin_precedente || r.end as usize > self.portes.len() {
                return Err(ErreurNetlist::RegionInvalide { region: i });
            }
            fin_precedente = r.end;
        }
        Ok(())
    }

    /// Nombre de pilotes de chaque net.
    pub fn nb_pilotes(&self) -> Vec<u32> {
        let mut n = vec![0u32; self.nb_nets as usize];
        for p in &self.portes {
            n[p.s as usize] += 1;
        }
        for &(net, _) in &self.constantes {
            n[net as usize] += 1;
        }
        for &net in &self.entrees {
            n[net as usize] += 1;
        }
        for h in &self.horloges {
            n[h.net as usize] += 1;
        }
        n
    }

    /// Le plus grand délai de porte (0 pour une netlist sans porte).
    pub fn delai_max(&self) -> u16 {
        self.portes.iter().map(|p| p.delai).max().unwrap_or(0)
    }
}
