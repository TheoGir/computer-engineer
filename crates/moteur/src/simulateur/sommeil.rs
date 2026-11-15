//! Mise en sommeil des régions qui répètent le même cycle à chaque période d'horloge.
//!
//! ## L'idée
//!
//! Dans un ordinateur, presque toutes les bascules reçoivent l'horloge et presque
//! aucune ne change : un registre que personne n'écrit refait exactement les mêmes
//! mouvements internes à chaque période. Les recalculer coûte un temps proportionnel
//! à la taille de la mémoire, pour un résultat connu d'avance.
//!
//! Une *région* est une plage de portes désignée par la netlist (un registre, par
//! exemple). Le simulateur observe une région pendant une période `P` (le plus petit
//! multiple commun des périodes des horloges). Si elle se retrouve dans le même
//! état qu'une période plus tôt, il l'endort : il garde la *trace* de cette période
//! (la suite des changements d'état de ses portes) et ne calcule plus rien pour
//! elle. L'état d'une région endormie à un instant quelconque se reconstitue en
//! rejouant sa trace jusqu'à la phase de cet instant.
//!
//! ## Pourquoi c'est exact (théorème 5 de `docs/semantique.md`)
//!
//! L'avenir d'une région ne dépend que de son état et des valeurs futures de ses
//! *nets d'entrée* (ceux qu'elle lit et qu'un pilote extérieur peut changer). Si
//! l'état est le même à `T` et à `T + P`, et si les entrées de la période suivante
//! répètent celles de la période observée, alors la région répète sa trace, par
//! récurrence sur le temps. Le simulateur ne suppose rien sur l'avenir : il
//! *surveille* chaque net d'entrée et réveille la région au premier écart, en lui
//! rendant d'abord l'état exact de cet instant.
//!
//! Trois précautions rendent le raisonnement complet :
//!
//! - une porte dont la sortie est lue hors de la région (ou sondée) ne doit pas
//!   changer pendant la période, sinon la région n'est pas endormie : le reste du
//!   circuit ne voit donc jamais une valeur périmée ;
//! - une entrée n'est pas surveillée quand toutes les portes de la région qui la
//!   lisent ont leur autre entrée à `0` pendant toute la période (un NAND dont une
//!   entrée vaut `0` sort `1` quoi que fasse l'autre). Cette autre entrée est, elle,
//!   surveillée ou interne à la région. C'est ce qui évite de réveiller toute une
//!   mémoire quand le bus de données bouge ;
//! - à chaque retour d'un appel public, l'état de toutes les régions endormies est
//!   reconstitué pour l'instant courant : toutes les lectures sont exactes.
//!
//! Le découpage en régions n'intervient pas dans la preuve : n'importe quel
//! découpage donne les mêmes valeurs, seul le gain de temps change.

use super::{
    valeur_des_comptes, Netlist, Simulateur, ATTENTE, AUCUN, DYNAMIQUE, ENDORMIE, MIXTE, N,
    OBSERVEE, P, SALE, SORTIE, TRI, UNITE, V0, VISIBLE, VZ,
};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, HashMap, VecDeque};
use std::sync::Arc;

// Drapeaux d'un net.
/// Les changements du net sont notés : une région en observation le lit.
const SUIVI: u32 = 1;
/// Le net a une attente : une région endormie compte sur sa valeur.
const ATTENDU: u32 = 2;
/// Le net a changé pendant l'instant courant et attend son traitement.
const TOUCHE: u32 = 4;

/// Au-delà de cette période commune des horloges, pas de sommeil.
const PERIODE_MAX: u64 = 1 << 24;
/// Nombre maximal de périodes entre deux observations d'une région qui ne s'endort pas.
const RECUL_MAX: u32 = 64;
/// Un sommeil de moins de tant de périodes n'a pas remboursé son observation : la
/// région attendra plus longtemps avant la suivante.
const SOMMEIL_COURT: u64 = 8;
/// Après un sommeil d'au moins tant de périodes, la région est observée de nouveau
/// sans attendre.
const SOMMEIL_LONG: u64 = 64;

/// Compteurs des régions et de leurs sommeils.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StatistiquesSommeil {
    /// Nombre de régions de la netlist.
    pub regions: usize,
    /// Nombre de régions endormies en ce moment.
    pub endormies: usize,
    /// Nombre de portes dans les régions endormies en ce moment.
    pub portes_endormies: usize,
    /// Nombre de mises en sommeil depuis le début.
    pub endormissements: u64,
    /// Nombre de réveils depuis le début.
    pub reveils: u64,
    /// Nombre d'observations qui n'ont pas mené au sommeil.
    pub observations_ratees: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Etat {
    Eveillee,
    Observee,
    Endormie,
}

/// Un changement d'état d'une porte dans une période.
///
/// `pos` vaut `2k` pour un changement de sortie à l'entrée de l'instant `k` de la
/// période, et `2k + 1` pour une évaluation à la sortie de cet instant.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Delta {
    pos: u32,
    /// Numéro de la porte dans sa région.
    g: u32,
    /// Les bits dynamiques de la porte après le changement.
    etat: u16,
    /// Temps restant avant l'échéance, si un changement est programmé.
    dtir: u16,
    /// Âge du dernier changement de cible, en mode mixte.
    dder: u16,
}

/// Une entrée de porte de la région branchée sur un net d'entrée de la région.
#[derive(Clone, Copy, Debug)]
struct Lecteur {
    porte: u32,
    slot: u8,
    /// Rang du net lu dans `Region::entrees`.
    entree: u32,
    /// Rang, dans `Region::entrees`, du net de l'autre entrée de la porte ;
    /// `AUCUN` s'il est interne à la région.
    autre: u32,
}

#[derive(Clone, Debug)]
struct Region {
    debut: u32,
    fin: u32,
    etat: Etat,
    /// Nets lus par la région et pilotés au moins en partie de l'extérieur.
    entrees: Vec<u32>,
    /// Pour chaque net d'entrée : il ne peut pas s'écarter de son cycle (horloge
    /// seule sur son net, ou constantes seules).
    garantie: Vec<bool>,
    lecteurs: Vec<Lecteur>,
    /// Nombre de périodes à attendre avant la prochaine observation.
    recul: u32,
    /// Observations abandonnées d'affilée parce que la région bougeait encore.
    tentatives: u8,
    // Observation en cours.
    debut_fenetre: u64,
    instantane: Vec<u16>,
    v0: Vec<u8>,
    journal: Vec<Delta>,
    echec: bool,
    // Sommeil en cours.
    trace: Arc<[Delta]>,
    /// Rang, dans la trace, du prochain delta à appliquer.
    curseur: u32,
    /// Position de la période à laquelle les tableaux du simulateur décrivent la région.
    position: u32,
    /// Demi-pas absolu (`2 × instant + phase`) correspondant à `position`.
    demi_pas: u64,
    /// Nombre de portes ayant un changement programmé à cette position.
    nb_attente: u32,
    /// À la dernière synchronisation, la région avait des portes à réévaluer ou un
    /// changement programmé.
    instable: bool,
    /// Nets dont la région a pris une attente.
    surveilles: Vec<u32>,
    /// Endormie depuis la dernière reconstitution : son état n'est pas encore
    /// celui de l'instant courant.
    fraiche: bool,
    /// Rang dans `Sommeil::endormies`.
    rang: u32,
    endormie_a: u64,
}

/// Ce qu'une région endormie attend d'un net : sa valeur à chaque phase de la période.
#[derive(Clone, Debug)]
struct Attente {
    net: u32,
    /// `(phase, valeur)` : la valeur du net à partir de cette phase, jusqu'au point
    /// suivant (cycliquement). Trié par phase, jamais vide.
    points: Vec<(u32, u8)>,
    /// Phases où la valeur change ; vide pour un net attendu constant.
    changements: Vec<u32>,
    /// Nombre de régions endormies qui comptent sur ce net.
    refs: u32,
    /// Régions à réveiller au premier écart (peut contenir des régions déjà réveillées).
    dormeuses: Vec<u32>,
    /// Change à chaque libération : périme les entrées de l'agenda.
    generation: u32,
}

#[derive(Clone, Debug, Default)]
struct Suivi {
    refs: u32,
    /// `(instant, valeur en fin d'entrée dans l'instant)`.
    journal: VecDeque<(u64, u8)>,
}

#[derive(Clone)]
pub(super) struct Sommeil {
    periode: u64,
    regions: Vec<Region>,
    /// Région de chaque porte, ou `AUCUN`.
    region_de: Vec<u32>,
    /// Rang, dans `fan`, de chaque entrée de chaque porte.
    pos: Vec<[u32; 2]>,
    /// Régions éveillées : `(instant à partir duquel observer, région)`.
    essais: BinaryHeap<Reverse<(u64, u32)>>,
    /// Observations en cours : `(instant de fin, régions)`, dans l'ordre des fins.
    fenetres: VecDeque<(u64, Vec<u32>)>,
    net_suivi: Vec<u32>,
    suivis: Vec<Suivi>,
    suivis_libres: Vec<u32>,
    net_attente: Vec<u32>,
    attentes: Vec<Attente>,
    attentes_libres: Vec<u32>,
    /// `(instant, attente, génération)` : prochain changement attendu de chaque net rythmé.
    agenda: BinaryHeap<Reverse<(u64, u32, u32)>>,
    touches: Vec<u32>,
    endormies: Vec<u32>,
    /// Nombre de régions endormies instables à la dernière synchronisation.
    instables: u32,
    /// Phase (instant modulo la période) → nombre de régions endormies qui y bougent.
    phases: BTreeMap<u32, u32>,
    /// Instant pour lequel les régions endormies sont reconstituées, sauf celles de
    /// `nouvelles`.
    synchro: Option<u64>,
    /// Régions endormies depuis la dernière reconstitution.
    nouvelles: Vec<u32>,
    /// Traces partagées entre régions qui font les mêmes mouvements.
    traces: HashMap<u64, Vec<Arc<[Delta]>>>,
    stats: StatistiquesSommeil,
}

fn pgcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        pgcd(b, a % b)
    }
}

impl Sommeil {
    /// Prépare les régions de `netlist`. Rend `None` s'il n'y a rien à endormir :
    /// pas de région, pas d'horloge, ou une période commune démesurée.
    pub(super) fn nouveau(
        netlist: &Netlist,
        portes: &mut [P],
        nets: &[N],
        fan: &[u32],
    ) -> Option<Box<Sommeil>> {
        if netlist.regions.is_empty() || netlist.horloges.is_empty() {
            return None;
        }
        let mut periode = 1u64;
        for h in &netlist.horloges {
            let p = h.demi_periode.checked_mul(2)?;
            periode = (periode / pgcd(periode, p)).checked_mul(p)?;
            if periode > PERIODE_MAX {
                return None;
            }
        }
        let nb_nets = netlist.nb_nets as usize;
        let nb_portes = portes.len();

        // Pilotes de chaque net, et nets qui ne peuvent pas s'écarter de leur cycle.
        let nb_pilotes = netlist.nb_pilotes();
        let mut pilotes_fixes = vec![0u32; nb_nets];
        for &(net, _) in &netlist.constantes {
            pilotes_fixes[net as usize] += 1;
        }
        let mut horloge_seule = vec![false; nb_nets];
        for h in &netlist.horloges {
            horloge_seule[h.net as usize] = nb_pilotes[h.net as usize] == 1;
        }
        let garanti =
            |n: usize| horloge_seule[n] || (nb_pilotes[n] > 0 && pilotes_fixes[n] == nb_pilotes[n]);

        let mut region_de = vec![AUCUN; nb_portes];
        for (r, plage) in netlist.regions.iter().enumerate() {
            for g in plage.clone() {
                region_de[g as usize] = r as u32;
            }
        }

        let mut dedans = vec![0u32; nb_nets];
        let mut regions = Vec::with_capacity(netlist.regions.len());
        let mut essais = BinaryHeap::with_capacity(netlist.regions.len());
        for (r, plage) in netlist.regions.iter().enumerate() {
            let (debut, fin) = (plage.start as usize, plage.end as usize);
            for p in &portes[debut..fin] {
                dedans[p.s as usize] += 1;
            }
            let externe = |n: u32| nb_pilotes[n as usize] > dedans[n as usize];
            let mut rang_entree: HashMap<u32, u32> = HashMap::new();
            let mut entrees = Vec::new();
            let mut garantie = Vec::new();
            let mut lecteurs = Vec::new();
            for (g, &p) in portes.iter().enumerate().take(fin).skip(debut) {
                let mut rang = |n: u32| -> u32 {
                    if !externe(n) {
                        return AUCUN;
                    }
                    *rang_entree.entry(n).or_insert_with(|| {
                        entrees.push(n);
                        garantie.push(garanti(n as usize));
                        entrees.len() as u32 - 1
                    })
                };
                let (r0, r1) = (rang(p.e0), rang(p.e1));
                if r0 != AUCUN {
                    lecteurs.push(Lecteur {
                        porte: g as u32,
                        slot: 0,
                        entree: r0,
                        autre: r1,
                    });
                }
                if r1 != AUCUN && p.e1 != p.e0 {
                    lecteurs.push(Lecteur {
                        porte: g as u32,
                        slot: 1,
                        entree: r1,
                        autre: r0,
                    });
                }
            }
            // Sorties visibles hors de la région : un pilote extérieur partage le
            // net, ou une porte extérieure le lit.
            for p in &mut portes[debut..fin] {
                let s = p.s as usize;
                let info = nets[s];
                let lecteurs_du_net =
                    &fan[info.fan_debut as usize..(info.fan_debut + info.fan_actifs) as usize];
                if nb_pilotes[s] > dedans[s]
                    || lecteurs_du_net
                        .iter()
                        .any(|&h| region_de[(h >> 1) as usize] != r as u32)
                {
                    p.etat |= VISIBLE;
                }
            }
            for p in &portes[debut..fin] {
                dedans[p.s as usize] = 0;
            }
            essais.push(Reverse((0, r as u32)));
            regions.push(Region {
                debut: debut as u32,
                fin: fin as u32,
                etat: Etat::Eveillee,
                entrees,
                garantie,
                lecteurs,
                recul: 1,
                tentatives: 0,
                debut_fenetre: 0,
                instantane: Vec::new(),
                v0: Vec::new(),
                journal: Vec::new(),
                echec: false,
                trace: Arc::from(Vec::new()),
                curseur: 0,
                position: 0,
                demi_pas: 0,
                nb_attente: 0,
                instable: false,
                surveilles: Vec::new(),
                fraiche: false,
                rang: 0,
                endormie_a: 0,
            });
        }

        let mut pos = vec![[AUCUN; 2]; nb_portes];
        for (i, &h) in fan.iter().enumerate() {
            pos[(h >> 1) as usize][(h & 1) as usize] = i as u32;
        }

        Some(Box::new(Sommeil {
            periode,
            stats: StatistiquesSommeil {
                regions: regions.len(),
                ..StatistiquesSommeil::default()
            },
            regions,
            region_de,
            pos,
            essais,
            fenetres: VecDeque::new(),
            net_suivi: vec![AUCUN; nb_nets],
            suivis: Vec::new(),
            suivis_libres: Vec::new(),
            net_attente: vec![AUCUN; nb_nets],
            attentes: Vec::new(),
            attentes_libres: Vec::new(),
            agenda: BinaryHeap::new(),
            touches: Vec::new(),
            endormies: Vec::new(),
            instables: 0,
            phases: BTreeMap::new(),
            synchro: None,
            nouvelles: Vec::new(),
            traces: HashMap::new(),
        }))
    }

    /// Vrai si aucune région endormie ne bouge à l'instant de la dernière synchronisation.
    #[inline]
    pub(super) fn tout_dort_au_repos(&self) -> bool {
        self.instables == 0
    }

    pub(super) fn statistiques(&self) -> StatistiquesSommeil {
        let mut s = self.stats;
        s.endormies = self.endormies.len();
        s.portes_endormies = self
            .endormies
            .iter()
            .map(|&r| {
                let reg = &self.regions[r as usize];
                (reg.fin - reg.debut) as usize
            })
            .sum();
        s
    }

    /// Instant de la prochaine activité d'une région endormie après `t`, ou
    /// `u64::MAX` si aucune ne bouge jamais.
    fn prochaine_activite(&self, t: u64) -> u64 {
        self.activite_des(t + 1)
    }

    /// Premier instant, à partir de `t` compris, où une région endormie bouge.
    fn activite_des(&self, t: u64) -> u64 {
        let k = (t % self.periode) as u32;
        if let Some((&phase, _)) = self.phases.range(k..).next() {
            t + (phase - k) as u64
        } else if let Some((&phase, _)) = self.phases.iter().next() {
            t + (self.periode - k as u64) + phase as u64
        } else {
            u64::MAX
        }
    }

    fn suivre(&mut self, nets: &mut [N], net: u32) {
        let i = self.net_suivi[net as usize];
        if i != AUCUN {
            self.suivis[i as usize].refs += 1;
            return;
        }
        let i = match self.suivis_libres.pop() {
            Some(i) => i,
            None => {
                self.suivis.push(Suivi::default());
                self.suivis.len() as u32 - 1
            }
        };
        let s = &mut self.suivis[i as usize];
        s.refs = 1;
        s.journal.clear();
        self.net_suivi[net as usize] = i;
        nets[net as usize].drap |= SUIVI;
    }

    fn ne_plus_suivre(&mut self, nets: &mut [N], net: u32) {
        let i = self.net_suivi[net as usize];
        let s = &mut self.suivis[i as usize];
        s.refs -= 1;
        if s.refs == 0 {
            s.journal.clear();
            self.suivis_libres.push(i);
            self.net_suivi[net as usize] = AUCUN;
            nets[net as usize].drap &= !SUIVI;
        }
    }

    /// Rend une référence sur l'attente du net ; la dernière la libère.
    fn liberer_attente(&mut self, nets: &mut [N], net: u32) {
        let a = self.net_attente[net as usize];
        if a == AUCUN {
            // L'attente a déjà été retirée par un écart.
            return;
        }
        let att = &mut self.attentes[a as usize];
        att.refs -= 1;
        if att.refs == 0 {
            self.retirer_attente(nets, a);
        }
    }

    fn retirer_attente(&mut self, nets: &mut [N], a: u32) {
        let att = &mut self.attentes[a as usize];
        att.generation = att.generation.wrapping_add(1);
        att.refs = 0;
        att.dormeuses.clear();
        let net = att.net;
        self.net_attente[net as usize] = AUCUN;
        nets[net as usize].drap &= !ATTENDU;
        self.attentes_libres.push(a);
    }
}

impl Attente {
    /// Valeur attendue à la phase `k`.
    fn valeur(&self, k: u32) -> u8 {
        let i = self.points.partition_point(|p| p.0 <= k);
        if i == 0 {
            self.points[self.points.len() - 1].1
        } else {
            self.points[i - 1].1
        }
    }

    /// Instant du premier changement attendu strictement après `t`.
    fn prochain_changement(&self, t: u64, periode: u64) -> Option<u64> {
        let premier = *self.changements.first()?;
        let k = (t % periode) as u32;
        let i = self.changements.partition_point(|&c| c <= k);
        Some(match self.changements.get(i) {
            Some(&c) => t + (c - k) as u64,
            None => t + (periode - k as u64) + premier as u64,
        })
    }
}

/// Empreinte d'une trace, pour retrouver vite les traces identiques. Deux traces de
/// même empreinte sont ensuite comparées en entier.
fn empreinte(trace: &[Delta]) -> u64 {
    let mut h = trace.len() as u64;
    for d in trace {
        let mot = (d.pos as u64) << 32
            ^ (d.g as u64) << 48
            ^ (d.etat as u64) << 32
            ^ (d.dtir as u64) << 16
            ^ d.dder as u64
            ^ d.g as u64;
        h = (h.rotate_left(5) ^ mot).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
    h
}

/// Valeur d'un net à chaque phase, d'après sa valeur `v0` juste avant la fenêtre
/// `[debut, debut + periode)` et son journal. Rend les points et les changements.
fn cycle_observe(
    v0: u8,
    journal: &VecDeque<(u64, u8)>,
    debut: u64,
    periode: u64,
) -> (Vec<(u32, u8)>, Vec<u32>) {
    let mut suite: Vec<(u64, u8)> = vec![(debut, v0)];
    for &(t, v) in journal {
        if t < debut || t >= debut + periode {
            continue;
        }
        match suite.last_mut() {
            Some(dernier) if dernier.0 == t => dernier.1 = v,
            _ => suite.push((t, v)),
        }
    }
    let mut points: Vec<(u32, u8)> = suite
        .iter()
        .map(|&(t, v)| ((t % periode) as u32, v))
        .collect();
    points.sort_unstable_by_key(|p| p.0);
    // Ne garder que les points où la valeur change (cycliquement).
    let n = points.len();
    let gardes: Vec<(u32, u8)> = (0..n)
        .filter(|&i| points[i].1 != points[(i + n - 1) % n].1)
        .map(|i| points[i])
        .collect();
    if gardes.is_empty() {
        (vec![points[0]], Vec::new())
    } else {
        let changements = gardes.iter().map(|p| p.0).collect();
        (gardes, changements)
    }
}

impl Simulateur {
    /// Sort le sommeil du simulateur le temps d'un traitement qui touche aux deux.
    fn avec_sommeil<R>(&mut self, f: impl FnOnce(&mut Simulateur, &mut Sommeil) -> R) -> R {
        let mut s = self
            .sommeil
            .take()
            .expect("appelé seulement avec des régions");
        let r = f(self, &mut s);
        self.sommeil = Some(s);
        r
    }

    /// Note l'état d'une porte observée après son évaluation.
    #[cold]
    #[inline(never)]
    pub(super) fn noter_evaluation(&mut self, g: u32) {
        self.noter(g, 1, false);
    }

    /// Note l'état d'une porte observée après un changement de sortie arrivé à échéance.
    #[cold]
    #[inline(never)]
    pub(super) fn noter_tir(&mut self, g: u32, ancien: u8, nouveau: u8) {
        self.noter(g, 0, ancien != nouveau);
    }

    fn noter(&mut self, g: u32, phase: u32, sortie_changee: bool) {
        let Some(s) = self.sommeil.as_mut() else {
            return;
        };
        let gi = g as usize;
        let r = &mut s.regions[s.region_de[gi] as usize];
        if r.echec {
            return;
        }
        let e = self.portes[gi].etat;
        if sortie_changee && e & VISIBLE != 0 {
            // Le reste du circuit voit cette sortie : la région ne peut pas dormir.
            r.echec = true;
            return;
        }
        let t32 = self.t as u32;
        r.journal.push(Delta {
            pos: 2 * (self.t - r.debut_fenetre) as u32 + phase,
            g: g - r.debut,
            etat: e & DYNAMIQUE,
            dtir: if e & ATTENTE == 0 {
                0
            } else if e & UNITE != 0 {
                1
            } else {
                self.t_tir[gi].wrapping_sub(t32) as u16
            },
            dder: if e & MIXTE != 0 {
                t32.wrapping_sub(self.t_der[gi]) as u16
            } else {
                0
            },
        });
    }

    /// Un net suivi ou attendu vient de changer : il sera traité en fin de phase 1.
    #[cold]
    #[inline(never)]
    pub(super) fn net_touche(&mut self, net: u32) {
        let info = &mut self.nets[net as usize];
        if info.drap & TOUCHE == 0 {
            info.drap |= TOUCHE;
            if let Some(s) = self.sommeil.as_mut() {
                s.touches.push(net);
            }
            self.a_traiter = true;
        }
    }

    /// Fin de l'entrée dans l'instant : les nets surveillés ont leur valeur
    /// définitive. On note ceux qui sont suivis, on vérifie ceux qui sont attendus,
    /// et on réveille les régions dont une attente n'est pas tenue.
    pub(super) fn fin_phase_1(&mut self, avec_agenda: bool) {
        self.a_traiter = false;
        if self.sommeil.is_none() {
            return;
        }
        self.avec_sommeil(|sim, s| {
            let t = sim.t;
            let k = (t % s.periode) as u32;
            let mut ecarts: Vec<u32> = Vec::new();
            let touches = std::mem::take(&mut s.touches);
            for &net in &touches {
                let n = net as usize;
                sim.nets[n].drap &= !TOUCHE;
                let v = sim.val[n];
                let i = s.net_suivi[n];
                if i != AUCUN {
                    let journal = &mut s.suivis[i as usize].journal;
                    // Plus aucune observation en cours ne remonte à plus d'une période.
                    while journal.front().is_some_and(|&(t0, _)| t0 + s.periode < t) {
                        journal.pop_front();
                    }
                    journal.push_back((t, v));
                }
                let a = s.net_attente[n];
                if a != AUCUN && s.attentes[a as usize].valeur(k) != v {
                    ecarts.push(a);
                }
            }
            s.touches = touches;
            s.touches.clear();
            if avec_agenda {
                while let Some(&Reverse((quand, a, generation))) = s.agenda.peek() {
                    if quand > t {
                        break;
                    }
                    s.agenda.pop();
                    let att = &s.attentes[a as usize];
                    if att.generation != generation || att.refs == 0 {
                        continue;
                    }
                    if att.valeur(k) != sim.val[att.net as usize] {
                        ecarts.push(a);
                    } else if let Some(suivant) = att.prochain_changement(t, s.periode) {
                        s.agenda.push(Reverse((suivant, a, generation)));
                    }
                }
                sim.prochaine_verif = s.agenda.peek().map_or(u64::MAX, |r| r.0 .0);
            }
            for a in ecarts {
                let att = &mut s.attentes[a as usize];
                if att.refs == 0 {
                    // Déjà traité : le même net figurait deux fois.
                    continue;
                }
                let net = att.net;
                let dormeuses = std::mem::take(&mut att.dormeuses);
                s.retirer_attente(&mut sim.nets, a);
                for r in dormeuses {
                    let reg = &s.regions[r as usize];
                    if reg.etat == Etat::Endormie && reg.surveilles.contains(&net) {
                        reveiller(sim, s, r, true);
                    }
                }
            }
        });
    }

    /// Le point des régions, juste avant d'entrer dans un instant de front d'horloge :
    /// conclut les observations qui se terminent, puis en commence de nouvelles.
    pub(super) fn tic(&mut self, endormir: bool) {
        self.avec_sommeil(|sim, s| {
            let t = sim.t;
            while s.fenetres.front().is_some_and(|f| f.0 <= t) {
                let (fin, regions) = s.fenetres.pop_front().expect("vérifié à l'instant");
                for r in regions {
                    // La comparaison ne vaut qu'exactement une période après le début.
                    conclure(sim, s, r, endormir && fin == t, fin == t);
                }
            }
            let mut nouvelles = Vec::new();
            while let Some(&Reverse((quand, r))) = s.essais.peek() {
                if quand > t {
                    break;
                }
                s.essais.pop();
                if s.regions[r as usize].etat == Etat::Eveillee && commencer(sim, s, r) {
                    nouvelles.push(r);
                }
            }
            if !nouvelles.is_empty() {
                s.fenetres.push_back((t + s.periode, nouvelles));
            }
            sim.prochaine_verif = s.agenda.peek().map_or(u64::MAX, |r| r.0 .0);
        });
    }

    /// Reconstitue l'état de toutes les régions endormies pour l'instant courant
    /// (ouvert) : après cet appel, toutes les lectures du simulateur sont exactes.
    pub(super) fn synchroniser(&mut self) {
        let t = self.t;
        let Some(s) = self.sommeil.as_ref() else {
            return;
        };
        if s.synchro == Some(t) && s.nouvelles.is_empty() {
            return;
        }
        self.avec_sommeil(|sim, s| {
            // Les régions déjà reconstituées à l'instant `avant` sont encore justes si
            // aucune n'a bougé depuis, ou si l'on est un nombre entier de périodes plus
            // tard sans changement programmé en l'air. Seules celles qui se sont
            // endormies entre-temps restent alors à reconstituer.
            let deja_justes = s.synchro.is_some_and(|avant| {
                avant == t
                    || s.activite_des(avant) > t
                    || ((t - avant).is_multiple_of(s.periode) && s.instables == 0)
            });
            let nouvelles = std::mem::take(&mut s.nouvelles);
            if deja_justes {
                for &r in &nouvelles {
                    let reg = &mut s.regions[r as usize];
                    if reg.etat == Etat::Endormie && reg.fraiche {
                        reg.fraiche = false;
                        materialiser(sim, s, r, true);
                        s.instables += u32::from(s.regions[r as usize].instable);
                    }
                }
            } else {
                let mut instables = 0;
                for i in 0..s.endormies.len() {
                    let r = s.endormies[i];
                    s.regions[r as usize].fraiche = false;
                    materialiser(sim, s, r, true);
                    instables += u32::from(s.regions[r as usize].instable);
                }
                s.instables = instables;
            }
            s.nouvelles = nouvelles;
            s.nouvelles.clear();
            s.synchro = Some(t);
        });
    }

    /// Réveille les régions endormies qui bougent à l'instant courant.
    pub(super) fn reveiller_instables(&mut self) {
        self.synchroniser();
        if self.sommeil.as_ref().is_some_and(|s| s.instables > 0) {
            self.avec_sommeil(|sim, s| {
                for r in s.endormies.clone() {
                    if s.regions[r as usize].instable {
                        reveiller(sim, s, r, true);
                    }
                }
                debug_assert_eq!(s.instables, 0);
            });
        }
    }

    /// Réveille toutes les régions endormies. L'instant courant doit être fermé.
    pub(super) fn reveiller_toutes(&mut self) {
        self.avec_sommeil(|sim, s| {
            for r in s.endormies.clone() {
                reveiller(sim, s, r, false);
            }
        });
    }

    /// Réveille toutes les régions et abandonne les observations en cours, à
    /// l'instant courant (ouvert) : le passage en mode rapide, qui calcule toutes les
    /// portes, les laisse ainsi dans leur état exact. Les régions réessaieront de
    /// s'endormir après le retour au calcul porte par porte.
    pub(super) fn reinitialiser_sommeil(&mut self) {
        if self.sommeil.is_none() {
            return;
        }
        self.synchroniser();
        self.avec_sommeil(|sim, s| {
            let t = sim.t;
            for r in s.endormies.clone() {
                reveiller(sim, s, r, true);
            }
            for (_, regions) in std::mem::take(&mut s.fenetres) {
                for r in regions {
                    let ri = r as usize;
                    if s.regions[ri].etat != Etat::Observee {
                        continue;
                    }
                    let (debut, fin) = (s.regions[ri].debut as usize, s.regions[ri].fin as usize);
                    for p in &mut sim.portes[debut..fin] {
                        p.etat &= !OBSERVEE;
                    }
                    for i in 0..s.regions[ri].entrees.len() {
                        if !s.regions[ri].garantie[i] {
                            let net = s.regions[ri].entrees[i];
                            s.ne_plus_suivre(&mut sim.nets, net);
                        }
                    }
                    s.regions[ri].journal = Vec::new();
                    remettre(s, r, t, false);
                }
            }
            s.agenda.clear();
            for net in std::mem::take(&mut s.touches) {
                sim.nets[net as usize].drap &= !TOUCHE;
            }
            sim.a_traiter = false;
            sim.prochaine_verif = u64::MAX;
        });
    }

    /// Instant de la prochaine activité d'une région endormie après l'instant courant.
    pub(super) fn prochaine_activite_endormie(&self) -> u64 {
        self.sommeil
            .as_ref()
            .map_or(u64::MAX, |s| s.prochaine_activite(self.t))
    }

    /// Une sonde est posée sur `net` : les portes qui le pilotent deviennent
    /// visibles, et leurs régions ne dorment plus tant que ce net bouge.
    pub(super) fn rendre_visible(&mut self, net: u32) {
        self.synchroniser();
        self.avec_sommeil(|sim, s| {
            for g in 0..sim.portes.len() {
                if sim.portes[g].s != net {
                    continue;
                }
                let r = s.region_de[g];
                if r == AUCUN {
                    continue;
                }
                sim.portes[g].etat |= VISIBLE;
                match s.regions[r as usize].etat {
                    Etat::Endormie => reveiller(sim, s, r, true),
                    Etat::Observee => s.regions[r as usize].echec = true,
                    Etat::Eveillee => {}
                }
            }
        });
    }
}

/// Remet une région à plus tard après une observation sans sommeil.
fn remettre(s: &mut Sommeil, r: u32, t: u64, doubler: bool) {
    let periode = s.periode;
    let reg = &mut s.regions[r as usize];
    reg.etat = Etat::Eveillee;
    if doubler {
        reg.recul = (reg.recul * 2).min(RECUL_MAX);
    }
    s.essais
        .push(Reverse((t + 1 + (reg.recul as u64 - 1) * periode, r)));
}

/// Commence l'observation de la région `r` à l'instant courant (pas encore entamé).
/// Rend faux si la région bouge encore : elle réessaiera plus tard.
fn commencer(sim: &mut Simulateur, s: &mut Sommeil, r: u32) -> bool {
    let t = sim.t;
    let reg = &mut s.regions[r as usize];
    let plage = reg.debut as usize..reg.fin as usize;
    if sim.portes[plage.clone()]
        .iter()
        .any(|p| p.etat & ATTENTE != 0)
    {
        // Elle réessaie au front suivant ; au bout de deux fois, elle recule.
        reg.tentatives += 1;
        let doubler = reg.tentatives >= 2;
        if doubler {
            reg.tentatives = 0;
        }
        remettre(s, r, t, doubler);
        return false;
    }
    reg.tentatives = 0;
    reg.instantane.clear();
    reg.instantane
        .extend(sim.portes[plage.clone()].iter().map(|p| p.etat & DYNAMIQUE));
    reg.v0.clear();
    reg.v0
        .extend(reg.entrees.iter().map(|&n| sim.val[n as usize]));
    for p in &mut sim.portes[plage] {
        p.etat |= OBSERVEE;
    }
    reg.etat = Etat::Observee;
    reg.debut_fenetre = t;
    reg.journal.clear();
    reg.echec = false;
    for i in 0..s.regions[r as usize].entrees.len() {
        if !s.regions[r as usize].garantie[i] {
            let net = s.regions[r as usize].entrees[i];
            s.suivre(&mut sim.nets, net);
        }
    }
    true
}

/// Termine l'observation de la région `r` : elle s'endort si elle a retrouvé son
/// état de départ sans montrer de changement au reste du circuit.
fn conclure(sim: &mut Simulateur, s: &mut Sommeil, r: u32, endormir_permis: bool, a_l_heure: bool) {
    let t = sim.t;
    let reg = &s.regions[r as usize];
    debug_assert_eq!(reg.etat, Etat::Observee);
    let plage = reg.debut as usize..reg.fin as usize;
    for p in &mut sim.portes[plage.clone()] {
        p.etat &= !OBSERVEE;
    }
    let periodique = a_l_heure
        && !reg.echec
        && sim.portes[plage]
            .iter()
            .zip(&reg.instantane)
            .all(|(p, &i)| p.etat & DYNAMIQUE == i);
    // Une région où rien ne s'est passé ne coûte rien éveillée : l'endormir
    // n'apporterait que de la surveillance.
    let calme = periodique && reg.journal.is_empty();
    if periodique && !calme && endormir_permis {
        endormir(sim, s, r);
    }
    for i in 0..s.regions[r as usize].entrees.len() {
        if !s.regions[r as usize].garantie[i] {
            let net = s.regions[r as usize].entrees[i];
            s.ne_plus_suivre(&mut sim.nets, net);
        }
    }
    if s.regions[r as usize].etat != Etat::Endormie {
        if !periodique {
            s.stats.observations_ratees += 1;
        }
        remettre(s, r, t, !periodique || calme);
    }
    s.regions[r as usize].journal = Vec::new();
}

/// Endort la région `r`, qui vient de passer une période entière pour revenir à son
/// état de départ.
fn endormir(sim: &mut Simulateur, s: &mut Sommeil, r: u32) {
    let t = sim.t;
    let periode = s.periode;
    let ri = r as usize;
    let (debut, fin) = (s.regions[ri].debut as usize, s.regions[ri].fin as usize);
    let debut_fenetre = s.regions[ri].debut_fenetre;

    // La trace de la période, et les nets internes qui y ont changé.
    let mut courant = s.regions[ri].instantane.clone();
    let mut trace: Vec<Delta> = Vec::with_capacity(s.regions[ri].journal.len());
    let mut varies: Vec<u32> = Vec::new();
    for d in &s.regions[ri].journal {
        let ancien = courant[d.g as usize];
        if d.pos & 1 == 0 && (ancien ^ d.etat) & SORTIE != 0 {
            varies.push(sim.portes[debut + d.g as usize].s);
        }
        // Une évaluation qui ne change rien à l'état de la porte n'est pas gardée.
        if d.etat != ancien {
            trace.push(*d);
        }
        courant[d.g as usize] = d.etat;
    }
    varies.sort_unstable();
    varies.dedup();

    // Quelles entrées surveiller ? Une entrée est ignorée quand l'autre entrée de
    // chaque porte qui la lit vaut 0 pendant toute la période.
    let reg = &s.regions[ri];
    let zero_constant_externe = |rang: u32| -> bool {
        let net = reg.entrees[rang as usize];
        if reg.v0[rang as usize] != V0 {
            return false;
        }
        if reg.garantie[rang as usize] {
            // Constantes seules : le net vaut 0 pour toujours. Une horloge, elle, bouge.
            return !sim.horloges.iter().any(|h| h.net == net);
        }
        let i = s.net_suivi[net as usize];
        i != AUCUN
            && s.suivis[i as usize]
                .journal
                .iter()
                .all(|&(quand, v)| quand < debut_fenetre || v == V0)
    };
    let nb_entrees = reg.entrees.len();
    let zero: Vec<bool> = (0..nb_entrees as u32).map(zero_constant_externe).collect();
    let mut besoin = vec![false; nb_entrees];
    let mut nb_lecteurs = vec![0u32; nb_entrees];
    // Portes dont les deux entrées sont externes et à 0 : il suffit d'en surveiller une.
    let mut au_choix: Vec<(u32, u32)> = Vec::new();
    for l in &reg.lecteurs {
        nb_lecteurs[l.entree as usize] += 1;
        let p = sim.portes[l.porte as usize];
        let (autre_net, autre_est_validation) = if l.slot == 0 {
            (p.e1, false)
        } else {
            (p.e0, true)
        };
        // Un NAND est masqué par un 0 sur l'une ou l'autre entrée ; un tampon trois
        // états seulement par sa validation.
        let masquable = autre_net != reg.entrees[l.entree as usize]
            && (p.etat & TRI == 0 || autre_est_validation);
        let ignoree = masquable
            && if l.autre == AUCUN {
                // L'autre entrée est interne : sa valeur découle de l'état de la région.
                sim.val[autre_net as usize] == V0 && varies.binary_search(&autre_net).is_err()
            } else if !zero[l.autre as usize] {
                false
            } else {
                if p.etat & TRI == 0 && zero[l.entree as usize] && l.slot == 0 {
                    // Chacune masque l'autre : on choisira plus bas laquelle surveiller,
                    // pour ne pas justifier chacune par l'autre.
                    au_choix.push((l.entree, l.autre));
                }
                true
            };
        if !ignoree {
            besoin[l.entree as usize] = true;
        }
    }
    for (a, b) in au_choix {
        let (a, b) = (a as usize, b as usize);
        // Une constante à 0 masque sans surveillance ; sinon on surveille de
        // préférence une entrée déjà surveillée, puis celle qui commande le plus de
        // portes (un signal de sélection plutôt qu'un fil de données).
        let constante = |i: usize| reg.garantie[i];
        if besoin[a] || besoin[b] || constante(a) || constante(b) {
            continue;
        }
        if nb_lecteurs[b] > nb_lecteurs[a] {
            besoin[b] = true;
        } else {
            besoin[a] = true;
        }
    }

    // Les attentes : une par net surveillé qui peut s'écarter de son cycle.
    let mut surveilles = Vec::new();
    let mut a_programmer = Vec::new();
    for (i, &net) in reg.entrees.iter().enumerate() {
        if !besoin[i] || reg.garantie[i] {
            continue;
        }
        surveilles.push(net);
        if s.net_attente[net as usize] == AUCUN {
            let suivi = &s.suivis[s.net_suivi[net as usize] as usize];
            let (points, changements) =
                cycle_observe(reg.v0[i], &suivi.journal, debut_fenetre, periode);
            a_programmer.push((net, points, changements));
        }
    }
    for (net, points, changements) in a_programmer {
        let a = match s.attentes_libres.pop() {
            Some(a) => a,
            None => {
                s.attentes.push(Attente {
                    net,
                    points: Vec::new(),
                    changements: Vec::new(),
                    refs: 0,
                    dormeuses: Vec::new(),
                    generation: 0,
                });
                s.attentes.len() as u32 - 1
            }
        };
        let att = &mut s.attentes[a as usize];
        att.net = net;
        att.points = points;
        att.changements = changements;
        att.refs = 0;
        att.dormeuses.clear();
        s.net_attente[net as usize] = a;
        sim.nets[net as usize].drap |= ATTENDU;
        // Le premier changement attendu peut tomber dans l'instant qui commence.
        if let Some(quand) = att.prochain_changement(t - 1, periode) {
            s.agenda.push(Reverse((quand, a, att.generation)));
        }
    }
    for &net in &surveilles {
        let att = &mut s.attentes[s.net_attente[net as usize] as usize];
        att.refs += 1;
        // Les réveils laissent des entrées périmées : on les tasse de temps en temps.
        if att.dormeuses.len() > 2 * att.refs as usize + 32 {
            let regions = &s.regions;
            att.dormeuses.retain(|&d| {
                regions[d as usize].etat == Etat::Endormie
                    && regions[d as usize].surveilles.contains(&net)
            });
            att.dormeuses.sort_unstable();
            att.dormeuses.dedup();
        }
        att.dormeuses.push(r);
    }

    // Les phases où la région bouge.
    let base = debut_fenetre % periode;
    let mut derniere = u32::MAX;
    for d in &trace {
        let k = d.pos >> 1;
        if k != derniere {
            derniere = k;
            *s.phases
                .entry(((base + k as u64) % periode) as u32)
                .or_insert(0) += 1;
        }
    }

    // La trace est partagée avec les régions qui font les mêmes mouvements.
    if s.traces.len() > 2 * s.endormies.len() + 1024 {
        // Ménage : les traces que plus aucune région n'utilise.
        s.traces.retain(|_, memes| {
            memes.retain(|a| Arc::strong_count(a) > 1);
            !memes.is_empty()
        });
    }
    let memes = s.traces.entry(empreinte(&trace)).or_default();
    let trace: Arc<[Delta]> = match memes.iter().find(|a| ***a == *trace) {
        Some(partagee) => partagee.clone(),
        None => {
            let nouvelle: Arc<[Delta]> = Arc::from(trace);
            memes.push(nouvelle.clone());
            nouvelle
        }
    };

    // La région quitte le noyau : ses portes ne sont plus marquées par les nets d'entrée.
    for p in &mut sim.portes[debut..fin] {
        p.etat |= ENDORMIE;
    }
    for i in 0..s.regions[ri].lecteurs.len() {
        let l = s.regions[ri].lecteurs[i];
        detacher(sim, s, l.porte, l.slot);
    }
    let rang = s.endormies.len() as u32;
    s.endormies.push(r);
    let reg = &mut s.regions[ri];
    reg.etat = Etat::Endormie;
    reg.trace = trace;
    reg.curseur = 0;
    reg.position = 0;
    reg.demi_pas = 2 * t;
    reg.nb_attente = 0;
    reg.instable = false;
    reg.surveilles = surveilles;
    reg.rang = rang;
    reg.endormie_a = t;
    reg.fraiche = true;
    s.nouvelles.push(r);
    s.stats.endormissements += 1;
}

/// Retire du noyau une entrée de porte : le net ne marquera plus cette porte.
fn detacher(sim: &mut Simulateur, s: &mut Sommeil, porte: u32, slot: u8) {
    let p = sim.portes[porte as usize];
    let net = if slot == 0 { p.e0 } else { p.e1 };
    let info = &mut sim.nets[net as usize];
    let i = s.pos[porte as usize][slot as usize];
    let dernier = info.fan_debut + info.fan_actifs - 1;
    debug_assert!(i >= info.fan_debut && i <= dernier);
    sim.fan.swap(i as usize, dernier as usize);
    let h = sim.fan[i as usize];
    s.pos[(h >> 1) as usize][(h & 1) as usize] = i;
    s.pos[porte as usize][slot as usize] = dernier;
    info.fan_actifs -= 1;
}

/// Remet dans le noyau une entrée de porte retirée par [`detacher`].
fn rattacher(sim: &mut Simulateur, s: &mut Sommeil, porte: u32, slot: u8) {
    let p = sim.portes[porte as usize];
    let net = if slot == 0 { p.e0 } else { p.e1 };
    let info = &mut sim.nets[net as usize];
    let i = s.pos[porte as usize][slot as usize];
    let premier_inactif = info.fan_debut + info.fan_actifs;
    debug_assert!(i >= premier_inactif);
    sim.fan.swap(i as usize, premier_inactif as usize);
    let h = sim.fan[i as usize];
    s.pos[(h >> 1) as usize][(h & 1) as usize] = i;
    s.pos[porte as usize][slot as usize] = premier_inactif;
    info.fan_actifs += 1;
}

/// Amène les tableaux du simulateur à l'état de la région `r` pour l'instant
/// courant : juste après l'entrée dans l'instant si `ouvert`, juste après sa sortie
/// sinon.
fn materialiser(sim: &mut Simulateur, s: &mut Sommeil, r: u32, ouvert: bool) {
    let periode = s.periode;
    let reg = &mut s.regions[r as usize];
    let (instant, phase) = if ouvert {
        (sim.t, 1u32)
    } else {
        (sim.t + 1, 0u32)
    };
    let base = reg.debut_fenetre % periode;
    let cible = 2 * ((instant + periode - base) % periode) as u32 + phase;
    let cible_demi_pas = 2 * instant + phase as u64;
    let tour = 2 * periode as u32;
    let distance = (cible + tour - reg.position) % tour;

    // Des changements programmés restés en l'air depuis la dernière fois gardent des
    // dates d'une période passée : on les ramène à la période courante.
    let demi_pas_position = cible_demi_pas - distance as u64;
    if reg.nb_attente > 0 && demi_pas_position != reg.demi_pas {
        let decalage = ((demi_pas_position - reg.demi_pas) / 2) as u32;
        for g in reg.debut as usize..reg.fin as usize {
            let e = sim.portes[g].etat;
            if e & ATTENTE != 0 {
                sim.t_tir[g] = sim.t_tir[g].wrapping_add(decalage);
            }
            if e & MIXTE != 0 {
                sim.t_der[g] = sim.t_der[g].wrapping_add(decalage);
            }
        }
    }
    reg.demi_pas = cible_demi_pas;

    let trace = reg.trace.clone();
    let nb = trace.len();
    if distance != 0 && nb > 0 {
        let mut i = reg.curseur as usize % nb;
        let mut faits = 0;
        while faits < nb {
            let d = trace[i];
            let avance = (d.pos + tour - reg.position) % tour;
            if avance >= distance {
                break;
            }
            let quand = ((cible_demi_pas - (distance - avance) as u64) >> 1) as u32;
            let g = (reg.debut + d.g) as usize;
            let p = &mut sim.portes[g];
            let ancien = p.etat;
            let nouveau = (ancien & !DYNAMIQUE) | d.etat;
            p.etat = nouveau;
            if nouveau & ATTENTE != 0 {
                sim.t_tir[g] = quand.wrapping_add(d.dtir as u32);
            }
            if nouveau & MIXTE != 0 {
                sim.t_der[g] = quand.wrapping_sub(d.dder as u32);
            }
            reg.nb_attente = reg.nb_attente + u32::from(nouveau & ATTENTE != 0)
                - u32::from(ancien & ATTENTE != 0);
            if (ancien ^ nouveau) & SORTIE != 0 {
                // Changement de sortie sans lecteur à prévenir : seule la valeur du net suit.
                let net = p.s as usize;
                let m = sim.nets[net].multi;
                let (avant, apres) = ((ancien & SORTIE) as u8, (nouveau & SORTIE) as u8);
                sim.val[net] = if m == AUCUN {
                    apres
                } else {
                    let c = &mut sim.comptes[m as usize];
                    if avant != VZ {
                        c[avant as usize] -= 1;
                    }
                    if apres != VZ {
                        c[apres as usize] += 1;
                    }
                    valeur_des_comptes(c)
                };
            }
            i = if i + 1 == nb { 0 } else { i + 1 };
            faits += 1;
        }
        reg.curseur = i as u32;
    }
    reg.position = cible;
    if ouvert {
        // Instable : un changement programmé, ou des portes à réévaluer à cet instant.
        reg.instable =
            reg.nb_attente > 0 || (nb > 0 && trace[reg.curseur as usize % nb].pos == cible);
    }
}

/// Réveille la région `r` : elle reprend l'état exact de l'instant courant et
/// retourne dans le noyau. `ouvert` : l'instant courant est entamé et pas encore
/// fermé (toutes les portes de la région seront réévaluées à sa sortie).
fn reveiller(sim: &mut Simulateur, s: &mut Sommeil, r: u32, ouvert: bool) {
    let t = sim.t;
    let periode = s.periode;
    let ri = r as usize;
    debug_assert_eq!(s.regions[ri].etat, Etat::Endormie);
    let etait_instable = s.regions[ri].instable;
    materialiser(sim, s, r, ouvert);
    let (debut, fin) = (s.regions[ri].debut as usize, s.regions[ri].fin as usize);
    for g in debut..fin {
        let p = &mut sim.portes[g];
        p.etat &= !ENDORMIE;
        if p.etat & ATTENTE != 0 {
            if p.etat & UNITE != 0 {
                // Programmé à la sortie de l'instant courant pour l'instant suivant.
                debug_assert!(!ouvert);
                sim.unites_suiv[sim.nb_suiv] = g as u32;
                sim.nb_suiv += 1;
            } else {
                let quand = sim.t_tir[g];
                debug_assert!(quand.wrapping_sub(t as u32) >= 1);
                sim.roue[(quand as u64 & sim.masque) as usize].push(g as u32);
                sim.nb_dans_roue += 1;
                sim.nb_attente += 1;
            }
        }
        if ouvert && p.etat & SALE == 0 {
            // Réévaluer une porte dont les entrées n'ont pas changé est sans effet :
            // on les marque toutes, sans chercher lesquelles ont vu un net bouger.
            p.etat |= SALE;
            sim.sales[sim.nb_sales] = g as u32;
            sim.nb_sales += 1;
        }
    }
    for i in 0..s.regions[ri].lecteurs.len() {
        let l = s.regions[ri].lecteurs[i];
        rattacher(sim, s, l.porte, l.slot);
    }
    let surveilles = std::mem::take(&mut s.regions[ri].surveilles);
    for &net in &surveilles {
        s.liberer_attente(&mut sim.nets, net);
    }

    // Les phases où la région bougeait.
    let base = s.regions[ri].debut_fenetre % periode;
    let trace = std::mem::replace(&mut s.regions[ri].trace, Arc::from(Vec::new()));
    let mut derniere = u32::MAX;
    for d in trace.iter() {
        let k = d.pos >> 1;
        if k != derniere {
            derniere = k;
            let phase = ((base + k as u64) % periode) as u32;
            let n = s.phases.get_mut(&phase).expect("phase inscrite au coucher");
            *n -= 1;
            if *n == 0 {
                s.phases.remove(&phase);
            }
        }
    }

    // Retrait de la liste des endormies.
    let rang = s.regions[ri].rang as usize;
    s.endormies.swap_remove(rang);
    if let Some(&deplacee) = s.endormies.get(rang) {
        s.regions[deplacee as usize].rang = rang as u32;
    }
    if etait_instable {
        s.instables -= 1;
    }

    let reg = &mut s.regions[ri];
    reg.instable = false;
    reg.fraiche = false;
    reg.nb_attente = 0;
    reg.etat = Etat::Eveillee;
    // Un sommeil très court n'a rien rapporté : la région attend plus longtemps
    // avant d'être observée de nouveau.
    let duree = t - reg.endormie_a;
    if duree < SOMMEIL_COURT * periode {
        reg.recul = (reg.recul * 2).min(RECUL_MAX);
    } else if duree >= SOMMEIL_LONG * periode {
        reg.recul = 1;
    }
    let recul = reg.recul as u64;
    s.essais.push(Reverse((t + 1 + (recul - 1) * periode, r)));
    s.stats.reveils += 1;
}
