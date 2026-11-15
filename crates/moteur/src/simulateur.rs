//! Simulateur événementiel optimisé.
//!
//! Il calcule exactement la sémantique de [`crate::specification`], mais en ne
//! touchant qu'aux portes dont une entrée vient de changer.
//!
//! ## Déroulement d'un instant
//!
//! Chaque instant `t` se traite en deux phases strictement séparées :
//!
//! 1. **Entrée dans l'instant** : les sorties de portes programmées pour `t`
//!    changent, les horloges basculent, les nets sont résolus. Les portes dont un
//!    net d'entrée a changé sont marquées « à réévaluer ».
//! 2. **Sortie de l'instant** : chaque porte marquée est évaluée *une seule fois*,
//!    avec les valeurs définitives des nets à `t`, et programme si besoin un
//!    changement de sortie à `t + délai`.
//!
//! Comme tout délai vaut au moins 1, la phase 2 ne modifie aucun net de l'instant
//! courant : le résultat ne dépend pas de l'ordre de parcours des portes.
//!
//! Entre deux appels, l'instant courant reste *ouvert* (phase 2 non faite) : les
//! entrées externes modifiées par [`Simulateur::fixer_entree`] prennent effet à
//! l'instant courant, et les portes les verront avec toutes les autres valeurs de
//! cet instant.
//!
//! ## Délai inertiel sûr, version événementielle
//!
//! Pour chaque porte on garde la sortie, la cible, un changement programmé éventuel
//! et un drapeau « mixte ». Invariant : *un changement est programmé si et
//! seulement si la cible diffère de la sortie*. Quand la cible passe à `b` à
//! l'instant `t` :
//!
//! - si `b` est la sortie actuelle, le changement programmé est annulé (l'impulsion
//!   était plus courte que le délai : elle est filtrée) ;
//! - sinon, si rien n'est programmé, un changement est programmé à `t + d` ;
//! - sinon la cible a pris deux valeurs différentes de la sortie dans la même
//!   fenêtre : le changement reste programmé à sa date, mais il est marqué *mixte*.
//!
//! À l'échéance, un changement non mixte donne à la sortie la valeur de la cible.
//! Un changement mixte donne `X`, puis reprogramme le passage à la cible `d` unités
//! après son dernier changement. Le cas mixte n'existe que si un `X` ou un `Z`
//! circule : sur des signaux purement binaires, c'est le délai inertiel classique.
//!
//! ## Portes de délai 1
//!
//! Une porte de délai 1 est un cas particulier de la règle : sa fenêtre ne contient
//! qu'une cible, donc sa sortie à `t + 1` est sa cible à `t`. Un changement
//! programmé à la sortie de l'instant `t` arrive toujours à l'entrée de l'instant
//! `t + 1`, avant toute autre évaluation : il n'est jamais annulé ni mixte. Le
//! simulateur range ces portes dans une simple liste « à l'instant suivant », sans
//! date ni roue temporelle, et les traite sans branchement.
//!
//! ## Régions en sommeil
//!
//! Quand la netlist désigne des régions ([`Netlist::regions`]) et contient une
//! horloge, le simulateur met en sommeil les régions qui répètent exactement le même
//! cycle à chaque période (un registre que personne n'écrit, par exemple) : il ne
//! les calcule plus, et reconstitue leur état à la demande. Les valeurs rendues sont
//! les mêmes, à chaque instant et sur chaque net : c'est le théorème 5 de
//! `docs/semantique.md`, et le sous-module `sommeil` en porte le détail.

mod rapide;
mod sommeil;

use crate::netlist::{ErreurNetlist, NetId, Netlist};
use crate::valeur::{Valeur, TABLE_NAND, TABLE_TRI, V0, V1, VX, VZ};
use rapide::Rapide;
use sommeil::Sommeil;
pub use sommeil::StatistiquesSommeil;

// Bits de l'état d'une porte.
const SORTIE: u16 = 0x003;
const CIBLE: u16 = 0x00C;
const ATTENTE: u16 = 0x010;
const MIXTE: u16 = 0x020;
const SALE: u16 = 0x040;
/// La porte appartient à une région en sommeil : le noyau ne la touche pas.
const ENDORMIE: u16 = 0x080;
/// La région de la porte est en observation : ses changements d'état sont notés.
const OBSERVEE: u16 = 0x100;
/// La sortie de la porte est visible hors de sa région (ou sondée).
const VISIBLE: u16 = 0x200;
/// Porte de délai 1.
const UNITE: u16 = 0x400;
/// Tampon trois états (sinon NAND).
const TRI: u16 = 0x800;
/// Les bits qui changent pendant la simulation.
const DYNAMIQUE: u16 = SORTIE | CIBLE | ATTENTE | MIXTE;

const AUCUN: u32 = u32::MAX;

/// Table des deux portes : `TRI >> 7 | e0 << 2 | e1`.
const TABLE: [u8; 32] = {
    let mut t = [0u8; 32];
    let mut i = 0;
    while i < 16 {
        t[i] = TABLE_NAND[i];
        t[16 + i] = TABLE_TRI[i];
        i += 1;
    }
    t
};

/// Une porte, sur 16 octets : tout ce que les deux phases lisent.
#[derive(Clone, Copy)]
#[repr(C)]
struct P {
    e0: u32,
    e1: u32,
    s: u32,
    etat: u16,
    delai: u16,
}

/// Un net : ses lecteurs actifs sont `fan[fan_debut..fan_debut + fan_actifs]`.
#[derive(Clone, Copy)]
struct N {
    fan_debut: u32,
    fan_actifs: u32,
    /// Index dans `comptes` si le net a plusieurs pilotes, `AUCUN` sinon.
    multi: u32,
    /// Drapeaux du sous-module `sommeil` ; zéro pour un net ordinaire.
    drap: u32,
}

#[derive(Clone, Copy)]
struct H {
    net: u32,
    demi_periode: u64,
    prochain_front: u64,
    niveau: u8,
}

/// Un changement de valeur enregistré par une sonde.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Changement {
    /// Instant du changement.
    pub t: u64,
    /// Numéro de la sonde (rendu par [`Simulateur::sonder`]).
    pub sonde: u32,
    /// Nouvelle valeur.
    pub valeur: Valeur,
}

/// Résultat de [`Simulateur::stabiliser`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stabilite {
    /// Le circuit est au repos depuis l'instant `t` : plus rien ne changera sans
    /// nouvelle action extérieure (hors horloges).
    Stable { t: u64 },
    /// Le budget de temps est épuisé et le circuit bouge encore à l'instant `t` :
    /// il oscille, ou il lui faut plus de temps.
    Instable { t: u64 },
}

impl Stabilite {
    /// Vrai si le circuit s'est stabilisé.
    pub fn est_stable(self) -> bool {
        matches!(self, Stabilite::Stable { .. })
    }
}

/// Compteurs du travail fait par le simulateur.
///
/// Ils comptent le travail réellement fait : une région en sommeil
/// ([`Netlist::regions`]) n'est pas calculée, donc pas comptée.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Statistiques {
    /// Nombre d'évaluations de portes.
    pub evaluations: u64,
    /// Nombre de changements de sortie arrivés à échéance.
    pub tirs: u64,
    /// Nombre d'instants où quelque chose s'est passé.
    pub instants_actifs: u64,
    /// Nombre d'échéances « mixtes » : la sortie est passée par `X` parce que la
    /// cible a pris deux valeurs différentes de la sortie dans une même fenêtre.
    pub tirs_mixtes: u64,
    /// Nombre d'impulsions filtrées : changements annulés avant leur échéance.
    pub impulsions_filtrees: u64,
    /// Nombre de périodes d'horloge calculées en mode rapide (voir [`Options::rapide`]).
    pub cycles_rapides: u64,
    /// Parmi elles, périodes calculées par le code compilé du circuit (voir
    /// [`Options::compilation`]).
    pub cycles_compiles: u64,
}

/// Réglages du simulateur. Aucun ne change les valeurs calculées.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    /// Met en sommeil les régions de la netlist qui répètent le même cycle à chaque
    /// période d'horloge (voir [`Netlist::regions`]). Actif par défaut.
    pub sommeil: bool,
    /// Calcule les circuits synchrones période par période : quand le circuit n'est
    /// fait que de bascules D cadencées par une seule horloge et de logique sans boucle
    /// qui a le temps de s'établir, et que l'on avance d'un nombre entier de périodes,
    /// le simulateur saute directement d'une période à la suivante. L'état obtenu est
    /// exactement celui du calcul porte par porte (théorème 6 de la sémantique) ; le
    /// simulateur revient de lui-même au calcul porte par porte dès qu'on regarde
    /// l'intérieur d'une période. Actif par défaut.
    pub rapide: bool,
    /// En mode rapide, traduit le circuit en code machine (en arrière-plan, quand il
    /// a passé assez de périodes en mode rapide pour que cela vaille la peine) : les
    /// valeurs calculées sont les mêmes, plusieurs fois plus vite. Actif par défaut.
    pub compilation: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            sommeil: true,
            rapide: true,
            compilation: true,
        }
    }
}

/// Le simulateur événementiel.
///
/// Il se clone : une copie est un état complet et indépendant de la simulation
/// (utile pour explorer plusieurs suites possibles depuis un même instant).
#[derive(Clone)]
pub struct Simulateur {
    t: u64,
    portes: Vec<P>,
    /// Date du changement programmé d'une porte de délai supérieur à 1 (32 bits de poids faible).
    t_tir: Vec<u32>,
    /// Date du dernier changement de cible en mode mixte (32 bits de poids faible).
    t_der: Vec<u32>,
    /// Valeur courante de chaque net (code sur 2 bits).
    val: Vec<u8>,
    nets: Vec<N>,
    /// Pour chaque net multi-pilote : nombre de pilotes à 0, à 1 et à X.
    comptes: Vec<[u32; 3]>,
    comptes_net: Vec<u32>,
    multi_touches: Vec<u32>,
    /// Portes lectrices de chaque net (numéro de porte × 2 + numéro de l'entrée), au format CSR.
    fan: Vec<u32>,
    /// Portes de délai 1 qui changent à l'instant suivant : les `nb_suiv` premières cases.
    unites_suiv: Vec<u32>,
    nb_suiv: usize,
    /// Tampon échangé avec `unites_suiv` à l'entrée de chaque instant.
    unites: Vec<u32>,
    /// Roue temporelle des autres portes : `roue[t & masque]` contient celles à échéance à `t`.
    roue: Vec<Vec<u32>>,
    masque: u64,
    nb_dans_roue: usize,
    /// Nombre de portes de délai supérieur à 1 ayant un changement programmé.
    nb_attente: usize,
    /// Portes à réévaluer à la sortie de l'instant courant : les `nb_sales` premières cases.
    sales: Vec<u32>,
    nb_sales: usize,
    entrees: Vec<u32>,
    entree_val: Vec<u8>,
    horloges: Vec<H>,
    /// Le plus proche front d'horloge.
    prochain_front: u64,
    sondes: Vec<(u32, u8)>,
    journal: Vec<Changement>,
    activite: bool,
    premier_conflit: Option<(u64, NetId)>,
    stats: Statistiques,
    /// Régions, observations et sommeils ; `None` sans région ou sans horloge.
    sommeil: Option<Box<Sommeil>>,
    /// Des nets surveillés par le sommeil ont changé pendant l'instant courant.
    a_traiter: bool,
    /// Prochain instant où le sommeil attend un changement d'un net surveillé.
    prochaine_verif: u64,
    /// Mode rapide ; `None` si le circuit ne s'y prête pas.
    rapide: Option<Box<Rapide>>,
    /// Le mode rapide compilé n'a tenu à jour que les valeurs des nets : chaque porte
    /// est établie, et sa sortie comme sa cible sont la valeur de son net de sortie
    /// (voir `etablir_etats`).
    etats_perimes: bool,
}

#[inline]
fn valeur_des_comptes(c: &[u32; 3]) -> u8 {
    if c[VX as usize] > 0 || (c[V0 as usize] > 0 && c[V1 as usize] > 0) {
        VX
    } else if c[V0 as usize] > 0 {
        V0
    } else if c[V1 as usize] > 0 {
        V1
    } else {
        VZ
    }
}

impl Simulateur {
    /// Prépare la simulation de `netlist`, à l'instant 0, avec les réglages par défaut.
    ///
    /// À la mise sous tension toutes les sorties de portes sont à `X`, les entrées
    /// externes aussi, et les horloges sont à `0`.
    pub fn nouveau(netlist: &Netlist) -> Result<Self, ErreurNetlist> {
        Self::avec_options(netlist, Options::default())
    }

    /// Comme [`Simulateur::nouveau`], avec des réglages choisis.
    pub fn avec_options(netlist: &Netlist, options: Options) -> Result<Self, ErreurNetlist> {
        netlist.valider()?;
        let nb_nets = netlist.nb_nets as usize;
        let nb_portes = netlist.portes.len();

        // Nets à plusieurs pilotes : résolution par comptage.
        let nb_pilotes = netlist.nb_pilotes();
        let mut multi = vec![AUCUN; nb_nets];
        let mut comptes = Vec::new();
        let mut comptes_net = Vec::new();
        for (n, &k) in nb_pilotes.iter().enumerate() {
            if k > 1 {
                multi[n] = comptes.len() as u32;
                comptes.push([0u32; 3]);
                comptes_net.push(n as u32);
            }
        }

        // Valeurs initiales : sans pilote Z, sinon résolution des pilotes initiaux.
        let mut val = vec![VZ; nb_nets];
        let mut poser = |net: u32, v: u8| {
            let m = multi[net as usize];
            if m == AUCUN {
                val[net as usize] = v;
            } else if v != VZ {
                comptes[m as usize][v as usize] += 1;
            }
        };
        for p in &netlist.portes {
            poser(p.s, VX);
        }
        for &(net, v) in &netlist.constantes {
            poser(net, v.code());
        }
        for &net in &netlist.entrees {
            poser(net, VX);
        }
        for h in &netlist.horloges {
            poser(h.net, V0);
        }
        for (m, c) in comptes.iter().enumerate() {
            val[comptes_net[m] as usize] = valeur_des_comptes(c);
        }

        // Lecteurs de chaque net (CSR). Une porte dont les deux entrées sont sur le
        // même net n'y figure qu'une fois.
        let mut fan_debut = vec![0u32; nb_nets + 1];
        for p in &netlist.portes {
            fan_debut[p.e0 as usize + 1] += 1;
            if p.e1 != p.e0 {
                fan_debut[p.e1 as usize + 1] += 1;
            }
        }
        for n in 0..nb_nets {
            fan_debut[n + 1] += fan_debut[n];
        }
        let mut curseur = fan_debut.clone();
        let mut fan = vec![0u32; fan_debut[nb_nets] as usize];
        for (g, p) in netlist.portes.iter().enumerate() {
            fan[curseur[p.e0 as usize] as usize] = (g as u32) << 1;
            curseur[p.e0 as usize] += 1;
            if p.e1 != p.e0 {
                fan[curseur[p.e1 as usize] as usize] = (g as u32) << 1 | 1;
                curseur[p.e1 as usize] += 1;
            }
        }
        let nets: Vec<N> = (0..nb_nets)
            .map(|n| N {
                fan_debut: fan_debut[n],
                fan_actifs: fan_debut[n + 1] - fan_debut[n],
                multi: multi[n],
                drap: 0,
            })
            .collect();

        // Toutes les portes sont à réévaluer à l'instant 0.
        let mut portes: Vec<P> = netlist
            .portes
            .iter()
            .map(|p| P {
                e0: p.e0,
                e1: p.e1,
                s: p.s,
                etat: VX as u16
                    | (VX as u16) << 2
                    | SALE
                    | if p.delai == 1 { UNITE } else { 0 }
                    | if p.genre as u8 == 1 { TRI } else { 0 },
                delai: p.delai,
            })
            .collect();
        let mut sales: Vec<u32> = (0..nb_portes as u32).collect();
        // Une case de plus : l'écriture sans branchement vise toujours une case valide.
        sales.push(0);

        let taille_roue = (netlist.delai_max() as usize + 1)
            .next_power_of_two()
            .max(2);
        let horloges: Vec<H> = netlist
            .horloges
            .iter()
            .map(|h| H {
                net: h.net,
                demi_periode: h.demi_periode,
                prochain_front: h.demi_periode,
                niveau: V0,
            })
            .collect();
        let prochain_front = horloges
            .iter()
            .map(|h| h.prochain_front)
            .min()
            .unwrap_or(u64::MAX);

        let sommeil = if options.sommeil {
            Sommeil::nouveau(netlist, &mut portes, &nets, &fan)
        } else {
            None
        };

        Ok(Simulateur {
            t: 0,
            portes,
            t_tir: vec![0; nb_portes],
            t_der: vec![0; nb_portes],
            val,
            nets,
            comptes,
            comptes_net,
            multi_touches: Vec::new(),
            fan,
            unites_suiv: vec![0; nb_portes + 1],
            nb_suiv: 0,
            unites: vec![0; nb_portes + 1],
            roue: vec![Vec::new(); taille_roue],
            masque: taille_roue as u64 - 1,
            nb_dans_roue: 0,
            nb_attente: 0,
            sales,
            nb_sales: nb_portes,
            entrees: netlist.entrees.clone(),
            entree_val: vec![VX; netlist.entrees.len()],
            horloges,
            prochain_front,
            sondes: Vec::new(),
            journal: Vec::new(),
            activite: true,
            premier_conflit: None,
            stats: Statistiques::default(),
            sommeil,
            a_traiter: false,
            prochaine_verif: u64::MAX,
            rapide: if options.rapide {
                Rapide::nouveau(netlist, options.compilation)
            } else {
                None
            },
            etats_perimes: false,
        })
    }

    /// Instant courant.
    #[inline]
    pub fn temps(&self) -> u64 {
        self.t
    }

    /// Valeur d'un net à l'instant courant.
    #[inline]
    pub fn valeur(&self, net: NetId) -> Valeur {
        Valeur::de_code(self.val[net as usize])
    }

    /// Codes (2 bits utiles par octet) de tous les nets, sans copie.
    ///
    /// Destiné à l'affichage : `codes()[net]` vaut `Valeur::code()`.
    #[inline]
    pub fn codes(&self) -> &[u8] {
        &self.val
    }

    /// Valeur pilotée par la sortie d'une porte à l'instant courant.
    #[inline]
    pub fn sortie_porte(&self, porte: usize) -> Valeur {
        let p = &self.portes[porte];
        if self.etats_perimes {
            return Valeur::de_code(self.val[p.s as usize]);
        }
        Valeur::de_code((p.etat & SORTIE) as u8)
    }

    /// État interne d'une porte, exprimé relativement à l'instant courant :
    /// `[sortie, cible, attente, mixte, temps restant avant l'échéance, âge du
    /// dernier changement de cible en mode mixte]`. Sert aux preuves par
    /// exploration exhaustive (`tests/exhaustif.rs`), pas à l'usage courant.
    #[doc(hidden)]
    pub fn etat_relatif_porte(&self, porte: usize) -> [u32; 6] {
        let p = &self.portes[porte];
        if self.etats_perimes {
            let v = self.val[p.s as usize] as u32;
            return [v, v, 0, 0, 0, 0];
        }
        let t32 = self.t as u32;
        let attente = p.etat & ATTENTE != 0;
        let mixte = p.etat & MIXTE != 0;
        [
            (p.etat & SORTIE) as u32,
            ((p.etat & CIBLE) >> 2) as u32,
            attente as u32,
            mixte as u32,
            if !attente {
                0
            } else if p.etat & UNITE != 0 {
                1
            } else {
                self.t_tir[porte].wrapping_sub(t32)
            },
            if mixte {
                t32.wrapping_sub(self.t_der[porte])
            } else {
                0
            },
        ]
    }

    /// Nombre de portes.
    pub fn nb_portes(&self) -> usize {
        self.portes.len()
    }

    /// Nombre d'entrées externes.
    pub fn nb_entrees(&self) -> usize {
        self.entrees.len()
    }

    /// Valeur actuellement pilotée par une entrée externe.
    pub fn entree(&self, entree: usize) -> Valeur {
        Valeur::de_code(self.entree_val[entree])
    }

    /// Change la valeur pilotée par une entrée externe, à l'instant courant.
    pub fn fixer_entree(&mut self, entree: usize, valeur: Valeur) {
        let v = valeur.code();
        let ancien = self.entree_val[entree];
        if ancien != v {
            self.entree_val[entree] = v;
            let net = self.entrees[entree];
            self.pilote_change(net, ancien, v);
            if self.a_traiter {
                self.fin_phase_1(false);
            }
        }
    }

    /// Avance le temps de `duree` unités.
    pub fn avancer(&mut self, duree: u64) {
        if self.en_rapide() {
            let periode = self.rapide.as_ref().map_or(1, |r| r.periode());
            if duree.is_multiple_of(periode) {
                self.periodes_rapides(duree / periode);
                return;
            }
            self.quitter_rapide();
        }
        let fin = self.t.saturating_add(duree);
        while self.t < fin {
            self.quitter_instant();
            let instant = self.prochain_instant(fin);
            self.aller(instant, true);
        }
        if self.sommeil.is_some() {
            self.synchroniser();
        }
        if self.rapide.is_some() {
            self.essayer_rapide();
        }
    }

    /// Vrai si plus rien ne changera sans action extérieure (hors horloges) : aucun
    /// changement de sortie n'est programmé, et la cible de chaque porte est déjà
    /// celle que demandent ses entrées.
    ///
    /// La réponse ne dépend que des valeurs et de l'état des portes, pas de la
    /// façon dont le simulateur s'organise : une porte qu'il doit réévaluer mais
    /// dont l'évaluation ne changerait rien ne compte pas.
    #[inline]
    pub fn est_stable(&self) -> bool {
        self.est_stable_eveille() && self.sommeil.as_ref().is_none_or(|s| s.tout_dort_au_repos())
    }

    /// Comme [`Simulateur::est_stable`], sans regarder les régions en sommeil.
    #[inline]
    fn est_stable_eveille(&self) -> bool {
        self.nb_suiv == 0
            && self.nb_attente == 0
            && self.sales[..self.nb_sales].iter().all(|&g| {
                let p = &self.portes[g as usize];
                let a = self.val[p.e0 as usize];
                let b = self.val[p.e1 as usize];
                let cible =
                    TABLE[((p.etat & TRI) >> 7) as usize | ((a << 2 | b) & 15) as usize] as u16;
                let actuelle = if self.etats_perimes {
                    self.val[p.s as usize] as u16
                } else {
                    (p.etat & CIBLE) >> 2
                };
                cible == actuelle
            })
    }

    /// Avance jusqu'au premier instant où le circuit est au repos, pendant au plus
    /// `budget` unités de temps. Un circuit déjà au repos n'avance pas.
    ///
    /// Un circuit qui oscille épuise le budget et rend [`Stabilite::Instable`].
    /// Les horloges continuent de battre : avec une horloge, « au repos » veut
    /// dire que tout s'est propagé avant le prochain front.
    pub fn stabiliser(&mut self, budget: u64) -> Stabilite {
        self.quitter_rapide();
        let fin = self.t.saturating_add(budget);
        // Les régions en sommeil qui bougent en ce moment sont réveillées : toutes
        // celles qui dorment encore sont au repos jusqu'à leur prochaine activité.
        if self.sommeil.is_some() {
            self.reveiller_instables();
        }
        let resultat = loop {
            if self.est_stable_eveille() {
                break Stabilite::Stable { t: self.t };
            }
            if self.t >= fin {
                break Stabilite::Instable { t: self.t };
            }
            self.quitter_instant();
            let mut instant = self.instant_suivant_pour_stabiliser(fin);
            if self.sommeil.is_some() && self.prochaine_activite_endormie() <= instant {
                // Une région en sommeil bougerait avant que le circuit soit au repos :
                // on les réveille toutes, et le calcul continue porte par porte.
                self.reveiller_toutes();
                instant = self.instant_suivant_pour_stabiliser(fin);
            }
            self.aller(instant, false);
        };
        if self.sommeil.is_some() {
            self.synchroniser();
        }
        resultat
    }

    /// Après la sortie de l'instant courant : l'instant suivant si plus rien n'est
    /// programmé (le repos y commence), sinon le prochain événement avant `fin`.
    fn instant_suivant_pour_stabiliser(&self, fin: u64) -> u64 {
        let limite = if self.nb_attente == 0 && self.nb_suiv == 0 {
            self.t + 1
        } else {
            fin
        };
        self.prochain_instant(limite)
    }

    /// Pose une sonde sur un net et renvoie son numéro. Les changements du net
    /// sont ensuite enregistrés dans le journal, à commencer par sa valeur actuelle.
    pub fn sonder(&mut self, net: NetId) -> u32 {
        assert!((net as usize) < self.val.len(), "net inconnu");
        self.quitter_rapide();
        if self.sommeil.is_some() {
            self.rendre_visible(net);
        }
        self.sondes.push((net, u8::MAX));
        self.activite = true;
        self.sondes.len() as u32 - 1
    }

    /// Rend et vide le journal des sondes, valeurs de l'instant courant comprises.
    pub fn vider_journal(&mut self) -> Vec<Changement> {
        self.relever();
        std::mem::take(&mut self.journal)
    }

    /// Nets actuellement en court-circuit franc : au moins un pilote à `0` et un
    /// pilote à `1`.
    pub fn conflits(&self) -> Vec<NetId> {
        self.comptes
            .iter()
            .zip(&self.comptes_net)
            .filter(|(c, _)| c[V0 as usize] > 0 && c[V1 as usize] > 0)
            .map(|(_, &n)| n)
            .collect()
    }

    /// Premier court-circuit franc constaté à la fermeture d'un instant depuis le
    /// début : instant et net. Un conflit à l'instant courant, encore ouvert, se
    /// lit avec [`Simulateur::conflits`].
    pub fn premier_conflit(&self) -> Option<(u64, NetId)> {
        self.premier_conflit
    }

    /// Compteurs du travail fait depuis le début.
    pub fn statistiques(&self) -> Statistiques {
        self.stats
    }

    /// Compteurs des régions et de leurs sommeils ; tout à zéro si la netlist n'a
    /// pas de région ou pas d'horloge.
    pub fn sommeil(&self) -> StatistiquesSommeil {
        self.sommeil
            .as_ref()
            .map_or_else(StatistiquesSommeil::default, |s| s.statistiques())
    }

    /// Relève les sondes et les conflits pour l'instant courant.
    fn relever(&mut self) {
        if !self.multi_touches.is_empty() {
            // Premier court-circuit : parmi les nets dont un pilote a changé pendant
            // cet instant, le plus petit de ceux qui ont un pilote à 0 et un pilote à 1.
            // Le choix ne dépend pas de l'ordre dans lequel les portes ont changé.
            let fautif = self
                .multi_touches
                .iter()
                .filter(|&&m| {
                    let c = &self.comptes[m as usize];
                    c[V0 as usize] > 0 && c[V1 as usize] > 0
                })
                .map(|&m| self.comptes_net[m as usize])
                .min();
            if let (None, Some(net)) = (self.premier_conflit, fautif) {
                self.premier_conflit = Some((self.t, net));
            }
            self.multi_touches.clear();
        }
        if !self.activite {
            return;
        }
        self.activite = false;
        self.stats.instants_actifs += 1;
        for (i, (net, derniere)) in self.sondes.iter_mut().enumerate() {
            let v = self.val[*net as usize];
            if v != *derniere {
                *derniere = v;
                self.journal.push(Changement {
                    t: self.t,
                    sonde: i as u32,
                    valeur: Valeur::de_code(v),
                });
            }
        }
    }

    /// Un pilote du net `net` passe de `ancien` à `nouveau`.
    #[inline(always)]
    fn pilote_change(&mut self, net: u32, ancien: u8, nouveau: u8) {
        let n = net as usize;
        let info = self.nets[n];
        let v = if info.multi == AUCUN {
            nouveau
        } else {
            let c = &mut self.comptes[info.multi as usize];
            if ancien != VZ {
                c[ancien as usize] -= 1;
            }
            if nouveau != VZ {
                c[nouveau as usize] += 1;
            }
            if self.premier_conflit.is_none() {
                self.multi_touches.push(info.multi);
            }
            valeur_des_comptes(c)
        };
        if v != self.val[n] {
            self.val[n] = v;
            self.activite = true;
            if info.drap != 0 {
                self.net_touche(net);
            }
            let debut = info.fan_debut as usize;
            let mut ns = self.nb_sales;
            for &h in &self.fan[debut..debut + info.fan_actifs as usize] {
                let h = h >> 1;
                let q = &mut self.portes[h as usize];
                // Sans branchement : la case est écrite dans tous les cas, et ne
                // compte que si la porte n'était pas déjà marquée.
                self.sales[ns] = h;
                ns += usize::from(q.etat & SALE == 0);
                q.etat |= SALE;
            }
            self.nb_sales = ns;
        }
    }

    /// Phase 2 de l'instant courant : évalue une fois chaque porte marquée.
    fn quitter_instant(&mut self) {
        self.relever();
        let n = self.nb_sales;
        if n == 0 {
            return;
        }
        self.nb_sales = 0;
        let t = self.t;
        let t32 = t as u32;
        let sales = std::mem::take(&mut self.sales);
        self.stats.evaluations += n as u64;
        let mut ns = self.nb_suiv;
        for &g in &sales[..n] {
            let gi = g as usize;
            let p = &mut self.portes[gi];
            let a = self.val[p.e0 as usize];
            let b = self.val[p.e1 as usize];
            let e = p.etat & !SALE;
            let cible = TABLE[((e & TRI) >> 7) as usize | ((a << 2 | b) & 15) as usize] as u16;
            if e & UNITE != 0 {
                // Délai 1 : la sortie de l'instant suivant est la cible de celui-ci.
                let attente = u16::from(cible != e & SORTIE);
                p.etat = (e & !(CIBLE | ATTENTE)) | cible << 2 | attente << 4;
                self.unites_suiv[ns] = g;
                ns += attente as usize;
            } else if cible != (e & CIBLE) >> 2 {
                let mut e2 = (e & !CIBLE) | cible << 2;
                if cible == e2 & SORTIE {
                    // Retour à la valeur de sortie : l'impulsion est filtrée.
                    e2 &= !(ATTENTE | MIXTE);
                    self.nb_attente -= 1;
                    self.stats.impulsions_filtrees += 1;
                } else if e2 & ATTENTE == 0 {
                    // La cible quitte la valeur de sortie : échéance dans `delai`.
                    // Le drapeau mixte est toujours déjà baissé ici : il l'est à chaque
                    // annulation et à chaque échéance.
                    debug_assert!(e2 & MIXTE == 0);
                    e2 |= ATTENTE;
                    self.t_tir[gi] = t32.wrapping_add(p.delai as u32);
                    let echeance = t + p.delai as u64;
                    self.roue[(echeance & self.masque) as usize].push(g);
                    self.nb_dans_roue += 1;
                    self.nb_attente += 1;
                } else {
                    // Deuxième valeur différente de la sortie dans la même fenêtre.
                    e2 |= MIXTE;
                    self.t_der[gi] = t32;
                }
                p.etat = e2;
            } else {
                p.etat = e;
            }
            if e & OBSERVEE != 0 {
                self.noter_evaluation(g);
            }
        }
        self.nb_suiv = ns;
        self.sales = sales;
    }

    /// Phase 1 de l'instant courant : applique les changements arrivés à échéance.
    fn entrer_instant(&mut self) {
        let t = self.t;
        let t32 = t as u32;

        // Portes de délai 1 : toutes celles de la liste changent, sans condition.
        let n = self.nb_suiv;
        if n > 0 {
            self.nb_suiv = 0;
            std::mem::swap(&mut self.unites, &mut self.unites_suiv);
            let unites = std::mem::take(&mut self.unites);
            self.stats.tirs += n as u64;
            for &g in &unites[..n] {
                let p = &mut self.portes[g as usize];
                let e = p.etat;
                let ancien = (e & SORTIE) as u8;
                let nouveau = ((e & CIBLE) >> 2) as u8;
                p.etat = (e & !(SORTIE | ATTENTE)) | nouveau as u16;
                let s = p.s;
                if e & OBSERVEE != 0 {
                    self.noter_tir(g, ancien, nouveau);
                }
                self.pilote_change(s, ancien, nouveau);
            }
            self.unites = unites;
        }

        // Autres portes : la case de la roue pour cet instant.
        let case = (t & self.masque) as usize;
        if !self.roue[case].is_empty() {
            let mut seau = std::mem::take(&mut self.roue[case]);
            self.nb_dans_roue -= seau.len();
            for &g in &seau {
                let gi = g as usize;
                let p = &mut self.portes[gi];
                let e = p.etat;
                if e & (ATTENTE | ENDORMIE) != ATTENTE || self.t_tir[gi] != t32 {
                    // Événement annulé ou remplacé, ou porte d'une région en sommeil.
                    continue;
                }
                let ancien = (e & SORTIE) as u8;
                let cible = ((e & CIBLE) >> 2) as u8;
                let nouveau;
                let mut e2 = e;
                if e & MIXTE == 0 {
                    nouveau = cible;
                    e2 &= !ATTENTE;
                    self.nb_attente -= 1;
                } else {
                    nouveau = VX;
                    e2 &= !MIXTE;
                    self.stats.tirs_mixtes += 1;
                    if cible != VX {
                        // La cible est stable depuis `t_der` : elle s'imposera à
                        // `t_der + delai`, strictement après `t`.
                        let reste = self.t_der[gi]
                            .wrapping_add(p.delai as u32)
                            .wrapping_sub(t32);
                        debug_assert!(reste >= 1 && reste < p.delai as u32);
                        self.t_tir[gi] = t32.wrapping_add(reste);
                        let echeance = t + reste as u64;
                        self.roue[(echeance & self.masque) as usize].push(g);
                        self.nb_dans_roue += 1;
                    } else {
                        e2 &= !ATTENTE;
                        self.nb_attente -= 1;
                    }
                }
                p.etat = (e2 & !SORTIE) | nouveau as u16;
                let s = p.s;
                self.stats.tirs += 1;
                if e & OBSERVEE != 0 {
                    self.noter_tir(g, ancien, nouveau);
                }
                if nouveau != ancien {
                    self.pilote_change(s, ancien, nouveau);
                }
            }
            // Aucun événement n'a pu être ajouté à cette case pendant son
            // traitement : les échéances sont à une distance de 1 à taille-1.
            seau.clear();
            debug_assert!(self.roue[case].is_empty());
            self.roue[case] = seau;
        }

        if t == self.prochain_front {
            let mut prochain = u64::MAX;
            for i in 0..self.horloges.len() {
                let h = self.horloges[i];
                if h.prochain_front == t {
                    let nouveau = if h.niveau == V0 { V1 } else { V0 };
                    self.horloges[i].niveau = nouveau;
                    self.horloges[i].prochain_front = t + h.demi_periode;
                    self.pilote_change(h.net, h.niveau, nouveau);
                }
                prochain = prochain.min(self.horloges[i].prochain_front);
            }
            self.prochain_front = prochain;
        }

        if self.a_traiter || t >= self.prochaine_verif {
            self.fin_phase_1(true);
        }
    }

    /// Le prochain instant où quelque chose se passe, sans dépasser `fin`.
    /// L'instant courant doit être fermé et `self.t < fin`.
    fn prochain_instant(&self, fin: u64) -> u64 {
        debug_assert!(self.nb_sales == 0 && self.t < fin);
        if self.nb_suiv > 0 {
            return self.t + 1;
        }
        let mut prochain = fin.min(self.prochain_front).min(self.prochaine_verif);
        if self.nb_dans_roue > 0 {
            // Les échéances sont à moins d'un tour de roue de l'instant courant.
            let limite = prochain.min(self.t + self.masque);
            let mut u = self.t + 1;
            while u < limite {
                if !self.roue[(u & self.masque) as usize].is_empty() {
                    break;
                }
                u += 1;
            }
            prochain = prochain.min(u);
        }
        prochain.max(self.t + 1)
    }

    /// Passe à `instant` et y entre. À un front d'horloge, les régions font d'abord
    /// leur point : fin des observations, mises en sommeil (si `endormir`), débuts
    /// d'observation.
    fn aller(&mut self, instant: u64, endormir: bool) {
        self.t = instant;
        if instant == self.prochain_front && self.sommeil.is_some() {
            self.tic(endormir);
        }
        self.entrer_instant();
    }
}
