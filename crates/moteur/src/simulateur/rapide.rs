//! Mode rapide cycle par cycle, exact à chaque période d'horloge.
//!
//! ## L'idée
//!
//! Un circuit *synchrone* est fait de bascules D cadencées par une même horloge et de
//! logique sans boucle entre elles. Si cette logique a le temps de s'établir entre
//! deux fronts, une période d'horloge se résume à : la logique s'établit, chaque
//! bascule prend au front montant la valeur de son entrée `d`, la logique s'établit à
//! nouveau. Calculer ces valeurs établies coûte une évaluation par porte dont une
//! entrée change, sans aucun temps intermédiaire : c'est beaucoup moins que de suivre
//! chaque impulsion parasite à l'unité de temps près.
//!
//! ## Ce qui est garanti (théorème 6 de `docs/semantique.md`)
//!
//! Le mode rapide part d'un instant `p` et avance par périodes entières. À chaque
//! instant `p + 2hk` où il s'arrête, l'état du simulateur (la valeur de chaque fil,
//! l'état complet de chaque porte) est *exactement* celui que donne le calcul porte
//! par porte. Entre ces instants, il ne calcule rien : dès qu'on demande autre chose
//! (avancer d'une durée qui n'est pas un multiple de la période, attendre le repos,
//! poser une sonde), le simulateur repart en mode exact depuis cet état, sans perte.
//!
//! Trois vérifications rendent la garantie complète, toutes faites par le programme :
//!
//! 1. **La bascule est prouvée par exhaustion.** Chaque bascule reconnue dans la
//!    netlist (le montage maître-esclave de neuf NAND de la bibliothèque) est simulée
//!    seule, porte par porte, avec ses vrais délais et la vraie demi-période, contre
//!    *toutes* les suites de valeurs de `d` (`0`, `1`, `X`) qui restent stables dans
//!    une fenêtre autour du front montant. L'exploration parcourt tous les états
//!    atteignables et établit : la fenêtre à respecter, l'intervalle où `q` peut
//!    changer après le front, l'état interne de la bascule à chaque phase en fonction
//!    de la valeur mémorisée et de `d`.
//! 2. **Les temps sont analysés.** Pour l'instant d'arrêt `p` choisi, une analyse des
//!    chemins (le théorème 4 étendu aux sources qui changent à des instants connus)
//!    borne le moment où chaque fil peut changer dans la période : tout doit être
//!    établi avant `p + 2h`, et l'entrée `d` de chaque bascule doit rester stable
//!    dans sa fenêtre.
//! 3. **L'état de départ est contrôlé.** Le passage en mode rapide n'a lieu que si
//!    l'état exact du moment est, porte par porte, l'état établi que le mode rapide
//!    suppose.
//!
//! Un circuit qui ne remplit pas ces conditions (boucles hors des bascules reconnues,
//! plusieurs horloges, pilotes multiples, logique trop lente pour la période) est
//! simplement simulé en mode exact.

use super::{
    Options, Simulateur, ATTENTE, AUCUN, CIBLE, ENDORMIE, MIXTE, SALE, SORTIE, TABLE, TRI,
};
use crate::netlist::{GenrePorte, Netlist};
use crate::valeur::{Valeur, TABLE_NAND, VX, VZ};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

mod compile;
use compile::{Compile, Emplacement, Execution, VAL_ADR};

/// Périodes calculées en mode rapide avant de lancer la compilation du circuit : un
/// circuit qu'on ne fait avancer que quelques périodes ne la justifie pas.
const SEUIL_COMPILATION: u64 = 256;

// Rôles des neuf portes d'une bascule maître-esclave.
/// Inverseur de l'horloge.
const INV: usize = 0;
/// Verrou maître : `m1 = nand(d, nclk)`, `m2 = nand(m1, nclk)`, puis la paire croisée.
const M1: usize = 1;
const M2: usize = 2;
const MQ: usize = 3;
const MQN: usize = 4;
/// Verrou esclave : `s1 = nand(mq, clk)`, `s2 = nand(s1, clk)`, puis la paire croisée.
const S1: usize = 5;
const S2: usize = 6;
const Q: usize = 7;
const QN: usize = 8;

const INFINI: i64 = i64::MAX / 4;

/// Plus long recul entre deux essais de passage en mode rapide, en appels.
const RECUL_MAX: u32 = 16;

/// Classes de sources pour l'analyse des temps.
const ENTREES: usize = 0;
const HORLOGE: usize = 1;
const BASCULES: usize = 2;

/// Ce que l'exploration exhaustive établit pour une bascule (délais et demi-période
/// donnés).
#[derive(Debug)]
pub(crate) struct Caracterisation {
    /// `d` doit rester stable de `avant` instants avant le front montant…
    pub(crate) avant: u64,
    /// … jusqu'à `apres` instants après.
    pub(crate) apres: u64,
    /// `q` et `q_n` ne changent qu'entre `c_min` et `c_max` instants après le front.
    pub(crate) c_min: u64,
    pub(crate) c_max: u64,
    /// Sorties des neuf portes une fois établies : `[niveau][d][mémorisée]`
    /// (valeurs `0`, `1`, `X` ; un `Z` sur `d` se lit comme un `X`).
    canon: [[[[u8; 9]; 3]; 3]; 2],
    /// Pour chaque phase de l'horloge : depuis combien d'instants `d` doit être
    /// stable pour que la bascule soit dans son état établi (`u32::MAX` : jamais).
    age_min: Vec<u32>,
    /// Pour chaque phase : les couples (`d`, mémorisée), bit `3 × d + mémorisée`,
    /// dont l'état établi fait partie des états explorés. Le mode rapide ne part que
    /// d'un tel état : tout ce qui peut lui arriver ensuite a été exploré.
    atteignable: Vec<u16>,
}

/// Rang d'une valeur dans les tables : `0`, `1`, `X` (un `Z` compte comme `X`).
#[inline]
fn rang(v: u8) -> usize {
    (v as usize).min(2)
}

/// Valeur mémorisée par une bascule dont l'entrée vaut `v` au front.
#[inline]
fn capture(v: u8) -> u8 {
    if v == VZ {
        VX
    } else {
        v
    }
}

#[inline]
fn inverse(v: u8) -> u8 {
    match v {
        0 => 1,
        1 => 0,
        _ => VX,
    }
}

/// La bascule seule : entrée 0 = `d`, horloge sur le net 0.
fn netlist_bascule(delais: [u16; 9], demi: u64) -> Netlist {
    let mut nl = Netlist::nouvelle();
    let c = nl.net();
    let d = nl.net();
    nl.entree(d);
    nl.horloge(c, demi);
    let o = nl.nets(9);
    let mut porte = |a: u32, b: u32, role: usize| {
        nl.nand(a, b, o[role], delais[role]);
    };
    porte(c, c, INV);
    porte(d, o[INV], M1);
    porte(o[M1], o[INV], M2);
    porte(o[M1], o[MQN], MQ);
    porte(o[M2], o[MQ], MQN);
    porte(o[MQ], c, S1);
    porte(o[S1], c, S2);
    porte(o[S1], o[QN], Q);
    porte(o[S2], o[Q], QN);
    nl
}

fn etat_compact(sim: &Simulateur) -> [u64; 9] {
    let mut e = [0u64; 9];
    for (g, case) in e.iter_mut().enumerate() {
        let r = sim.etat_relatif_porte(g);
        *case = r[0] as u64
            | (r[1] as u64) << 2
            | (r[2] as u64) << 4
            | (r[3] as u64) << 5
            | (r[4] as u64 & 0xFFFF) << 8
            | (r[5] as u64 & 0xFFFF) << 24;
    }
    e
}

/// L'état compact d'une porte établie à la valeur `v`.
#[inline]
fn compact_etabli(v: u8) -> u64 {
    v as u64 | (v as u64) << 2
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct Cle {
    phase: u32,
    d: u8,
    age: u8,
    /// Valeur prise au dernier front montant ; 3 avant le premier front.
    prise: u8,
    etat: [u64; 9],
}

/// Explore tous les comportements de la bascule quand `d` reste stable de `avant`
/// instants avant chaque front montant à `apres` instants après. Rend `None` si la
/// bascule ne se comporte pas comme une bascule D dans ces conditions.
fn explorer(delais: [u16; 9], demi: u64, avant: u64, apres: u64) -> Option<Caracterisation> {
    let periode = 2 * demi;
    let somme: u64 = delais.iter().map(|&d| d as u64).sum();
    let age_max = (somme + 2).min(periode - 1).min(250) as u8;
    let nl = netlist_bascule(delais, demi);
    let options = Options {
        sommeil: false,
        rapide: false,
        compilation: false,
    };
    let depart = Simulateur::avec_options(&nl, options).ok()?;
    let interdit = |phase: u64| {
        let rel = (phase + periode - demi) % periode;
        rel <= apres || (avant > 0 && rel >= periode - avant)
    };
    let sorties = |sim: &Simulateur| (sim.sortie_porte(Q).code(), sim.sortie_porte(QN).code());

    let mut vus: HashSet<Cle> = HashSet::new();
    let mut file: VecDeque<(Simulateur, u8, u8, u8)> = VecDeque::new();
    // Observations : (phase, d, prise, âge, état).
    let mut obs: Vec<(u32, u8, u8, u8, [u64; 9])> = Vec::new();
    // Phases (comptées depuis le front montant) où q n'a pas encore la valeur prise.
    let mut en_retard: HashSet<u64> = HashSet::new();
    // Phases où q ou q_n change.
    let mut changements: HashSet<u64> = HashSet::new();

    vus.insert(Cle {
        phase: 0,
        d: VX,
        age: 0,
        prise: 3,
        etat: etat_compact(&depart),
    });
    file.push_back((depart, VX, 0, 3));
    while let Some((sim, d, age, prise)) = file.pop_front() {
        let t = sim.temps();
        let phase = t % periode;
        let rel = |ph: u64| (ph + periode - demi) % periode;
        let (q, qn) = sorties(&sim);
        if prise != 3 {
            if (q, qn) != (prise, inverse(prise)) {
                en_retard.insert(rel(phase));
            }
            obs.push((phase as u32, d, prise, age, etat_compact(&sim)));
        }
        let mut choix = vec![d];
        if !interdit(phase) {
            choix.extend([0u8, 1, VX].into_iter().filter(|&v| v != d));
        }
        for d2 in choix {
            let mut s2 = sim.clone();
            let mut age2 = age;
            if d2 != d {
                s2.fixer_entree(0, Valeur::de_code(d2));
                age2 = 0;
            }
            let prise2 = if phase == demi { capture(d2) } else { prise };
            s2.avancer(1);
            if prise2 != 3 && sorties(&s2) != (q, qn) {
                changements.insert(rel((t + 1) % periode));
            }
            let age2 = age2.saturating_add(1).min(age_max);
            let cle = Cle {
                phase: ((t + 1) % periode) as u32,
                d: d2,
                age: age2,
                prise: prise2,
                etat: etat_compact(&s2),
            };
            if vus.insert(cle) {
                file.push_back((s2, d2, age2, prise2));
            }
        }
    }

    // État établi de référence : au dernier instant de chaque niveau, l'état atteint
    // quand `d` est stable depuis le plus longtemps possible. Ce choix ne sert qu'à
    // nommer l'état établi : c'est l'exploration (`age_min`, `atteignable`) qui dit
    // quand la bascule s'y trouve.
    let mut canon = [[[[0u8; 9]; 3]; 3]; 2];
    for (niveau, case) in canon.iter_mut().enumerate() {
        let phase_ref = ((niveau as u64 + 1) * demi - 1) as u32;
        for (u, ligne) in case.iter_mut().enumerate() {
            for (v, sorties) in ligne.iter_mut().enumerate() {
                let choisis = |o: &&(u32, u8, u8, u8, [u64; 9])| {
                    o.0 == phase_ref && o.1 as usize == u && o.2 as usize == v
                };
                let age = obs.iter().filter(choisis).map(|o| o.3).max()?;
                let etats: HashSet<[u64; 9]> = obs
                    .iter()
                    .filter(choisis)
                    .filter(|o| o.3 == age)
                    .map(|o| o.4)
                    .collect();
                if etats.len() != 1 {
                    return None;
                }
                let etat = etats.into_iter().next().expect("un état");
                for g in 0..9 {
                    let s = (etat[g] & 3) as u8;
                    if etat[g] != compact_etabli(s) {
                        return None;
                    }
                    sorties[g] = s;
                }
                if (sorties[Q], sorties[QN]) != (v as u8, inverse(v as u8)) {
                    return None;
                }
            }
        }
    }

    // Âge minimal de d, phase par phase, pour être dans l'état établi.
    let mut pire = vec![-1i64; periode as usize];
    let mut jamais = vec![false; periode as usize];
    for &(phase, u, v, age, etat) in &obs {
        let niveau = usize::from(phase as u64 >= demi);
        let attendu = canon[niveau][rang(u)][v as usize];
        if (0..9).any(|g| etat[g] != compact_etabli(attendu[g])) {
            let p = phase as usize;
            pire[p] = pire[p].max(age as i64);
            if age == age_max {
                jamais[p] = true;
            }
        }
    }
    let mut atteignable = vec![0u16; periode as usize];
    for &(phase, u, v, _, etat) in &obs {
        let niveau = usize::from(phase as u64 >= demi);
        let attendu = canon[niveau][rang(u)][v as usize];
        if (0..9).all(|g| etat[g] == compact_etabli(attendu[g])) {
            atteignable[phase as usize] |= 1 << (3 * rang(u) + v as usize);
        }
    }
    let age_min = (0..periode as usize)
        .map(|p| {
            if jamais[p] {
                u32::MAX
            } else {
                (pire[p] + 1) as u32
            }
        })
        .collect();

    // Fenêtre de changement de q après le front, et q juste après.
    let c_min = *changements.iter().min()?;
    let c_max = *changements.iter().max()?;
    if c_min == 0 || en_retard.iter().any(|&r| r > c_max) || c_max >= periode {
        return None;
    }
    Some(Caracterisation {
        avant,
        apres,
        c_min,
        c_max,
        canon,
        age_min,
        atteignable,
    })
}

type CleCache = ([u16; 9], u64);

fn cache() -> &'static Mutex<HashMap<CleCache, Option<Arc<Caracterisation>>>> {
    static CACHE: OnceLock<Mutex<HashMap<CleCache, Option<Arc<Caracterisation>>>>> =
        OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Caractérise la bascule de délais `delais` cadencée à la demi-période `demi`, avec
/// la plus petite fenêtre de stabilité qui convienne. Le résultat est gardé en mémoire
/// pour tout le programme.
pub(crate) fn caracteriser(delais: [u16; 9], demi: u64) -> Option<Arc<Caracterisation>> {
    if let Some(c) = cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(&(delais, demi))
    {
        return c.clone();
    }
    // Un Z lu par un NAND doit se comporter comme un X : l'exploration n'essaie que X.
    // Et le NAND doit être symétrique : l'exploration branche les entrées dans un ordre,
    // la netlist peut les brancher dans l'autre.
    let z_comme_x = (0..4).all(|b| {
        TABLE_NAND[(VZ << 2 | b) as usize] == TABLE_NAND[(VX << 2 | b) as usize]
            && TABLE_NAND[(b << 2 | VZ) as usize] == TABLE_NAND[(b << 2 | VX) as usize]
            && (0..4)
                .all(|a| TABLE_NAND[(a << 2 | b) as usize] == TABLE_NAND[(b << 2 | a) as usize])
    });
    let mut trouve = None;
    if z_comme_x && demi >= 2 {
        'recherche: for total in 0..=12u64 {
            for apres in 0..=total.min(4) {
                let avant = total - apres;
                if avant + apres + 2 >= 2 * demi {
                    continue;
                }
                if let Some(c) = explorer(delais, demi, avant, apres) {
                    trouve = Some(Arc::new(c));
                    break 'recherche;
                }
            }
        }
    }
    cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert((delais, demi), trouve.clone());
    trouve
}

/// Une bascule reconnue dans la netlist.
#[derive(Clone, Debug)]
struct Bascule {
    /// Les neuf portes, dans l'ordre des rôles.
    portes: [u32; 9],
    /// Net de l'entrée `d`.
    d: u32,
    /// Nets de sortie des neuf portes.
    sorties: [u32; 9],
    carac: Arc<Caracterisation>,
}

/// Une porte de la logique, telle que le mode rapide l'évalue.
#[derive(Clone, Copy, Debug)]
struct PorteLogique {
    e0: u32,
    e1: u32,
    s: u32,
    /// Numéro de la porte dans le simulateur ; bit de poids fort : tampon trois états.
    porte: u32,
}

const TROIS_ETATS: u32 = 1 << 31;

/// Ce que l'analyse de la netlist a établi une fois pour toutes.
#[derive(Debug)]
pub(super) struct Plan {
    horloge: u32,
    demi: u64,
    bascules: Vec<Bascule>,
    /// Pour chaque porte : `AUCUN` si elle est dans la logique, sinon sa bascule.
    bascule_de: Vec<u32>,
    /// La logique, rangée par niveau croissant (1 pour les portes qui ne lisent que
    /// des sources) : de quoi l'évaluer sans aller lire le reste du simulateur.
    logique: Vec<PorteLogique>,
    /// Rang dans `logique` de chaque porte de la logique, `AUCUN` pour les autres.
    rang_logique: Vec<u32>,
    /// Niveau de chaque porte de `logique`.
    niveau: Vec<u32>,
    nb_niveaux: usize,
    /// Pour chaque net et chaque classe de sources : plus court et plus long délai.
    tot: [Vec<i64>; 3],
    tard: [Vec<i64>; 3],
    /// Nets dont l'analyse des temps vérifie qu'ils sont établis à l'instant d'arrêt.
    nets_lus: Vec<u32>,
    /// Ce que le mode rapide doit prévenir quand un net change, au format CSR : les
    /// lecteurs du net `n` sont `lecteurs[debut[n]..debut[n + 1]]`, chacun une porte de
    /// la logique (son niveau, au moins 1, dans les 32 bits de poids fort) ou une
    /// bascule dont c'est l'entrée `d` (niveau 0).
    debut: Vec<u32>,
    lecteurs: Vec<u64>,
    /// La file des portes à évaluer est découpée par niveau : celles du niveau `k` vont
    /// dans `file[debut_niveau[k]..]`. Une porte y entre au plus une fois à la fois.
    debut_niveau: Vec<u32>,
    c_min: i64,
    c_max: i64,
    nb_nets: usize,
    /// Le code compilé, une fois prêt (`None` : la compilation a échoué).
    compilation: OnceLock<Option<Arc<Compile>>>,
    /// La compilation est lancée.
    lancee: AtomicBool,
}

/// Lecteurs de chaque net (portes distinctes).
fn lecteurs(netlist: &Netlist) -> Vec<Vec<u32>> {
    let mut l = vec![Vec::new(); netlist.nb_nets as usize];
    for (g, p) in netlist.portes.iter().enumerate() {
        l[p.e0 as usize].push(g as u32);
        if p.e1 != p.e0 {
            l[p.e1 as usize].push(g as u32);
        }
    }
    l
}

/// Les deux entrées de la porte `g` sont `{a, b}` (dans un ordre quelconque).
fn entrees_sont(netlist: &Netlist, g: u32, a: u32, b: u32) -> bool {
    let p = &netlist.portes[g as usize];
    p.genre == GenrePorte::Nand && ((p.e0 == a && p.e1 == b) || (p.e0 == b && p.e1 == a))
}

/// L'autre entrée de la porte `g`, sachant que l'une est `a`.
fn autre_entree(netlist: &Netlist, g: u32, a: u32) -> Option<u32> {
    let p = &netlist.portes[g as usize];
    if p.e0 == a {
        Some(p.e1)
    } else if p.e1 == a {
        Some(p.e0)
    } else {
        None
    }
}

/// Reconnaît la bascule dont l'inverseur d'horloge est la porte `inv`.
fn reconnaitre(
    netlist: &Netlist,
    lect: &[Vec<u32>],
    horloge: u32,
    inv: u32,
) -> Option<([u32; 9], u32)> {
    let sortie = |g: u32| netlist.portes[g as usize].s;
    let lus = |n: u32| &lect[n as usize];
    let seuls = |n: u32, attendus: &[u32]| {
        let l = lus(n);
        l.len() == attendus.len() && attendus.iter().all(|a| l.contains(a))
    };
    if !entrees_sont(netlist, inv, horloge, horloge) {
        return None;
    }
    let nclk = sortie(inv);
    let l = lus(nclk);
    if l.len() != 2 {
        return None;
    }
    for (m1, m2) in [(l[0], l[1]), (l[1], l[0])] {
        let sm = sortie(m1);
        if !entrees_sont(netlist, m2, sm, nclk)
            || netlist.portes[m1 as usize].genre != GenrePorte::Nand
        {
            continue;
        }
        let Some(d) = autre_entree(netlist, m1, nclk) else {
            continue;
        };
        let rm = sortie(m2);
        // Paire croisée du maître : mq lit sm, mqn lit rm.
        let Some(&mq) = lus(sm).iter().find(|&&g| g != m2) else {
            continue;
        };
        if !seuls(sm, &[m2, mq]) {
            continue;
        }
        let [mqn] = lus(rm)[..] else {
            continue;
        };
        let (mq_net, mqn_net) = (sortie(mq), sortie(mqn));
        if !entrees_sont(netlist, mq, sm, mqn_net) || !entrees_sont(netlist, mqn, rm, mq_net) {
            continue;
        }
        // Esclave.
        let Some(&s1) = lus(mq_net).iter().find(|&&g| g != mqn) else {
            continue;
        };
        if !seuls(mq_net, &[mqn, s1])
            || !seuls(mqn_net, &[mq])
            || !entrees_sont(netlist, s1, mq_net, horloge)
        {
            continue;
        }
        let ss = sortie(s1);
        let lss = lus(ss);
        if lss.len() != 2 {
            continue;
        }
        for (s2, q) in [(lss[0], lss[1]), (lss[1], lss[0])] {
            if !entrees_sont(netlist, s2, ss, horloge) {
                continue;
            }
            let rs = sortie(s2);
            let [qn] = lus(rs)[..] else {
                continue;
            };
            let (q_net, qn_net) = (sortie(q), sortie(qn));
            if !entrees_sont(netlist, q, ss, qn_net) || !entrees_sont(netlist, qn, rs, q_net) {
                continue;
            }
            let portes = [inv, m1, m2, mq, mqn, s1, s2, q, qn];
            let distinctes: HashSet<u32> = portes.iter().copied().collect();
            let internes = [nclk, sm, rm, mq_net, mqn_net, ss, rs];
            if distinctes.len() == 9 && !internes.contains(&d) && d != horloge {
                return Some((portes, d));
            }
        }
    }
    None
}

impl Plan {
    /// Analyse la netlist ; `None` si elle ne se prête pas au mode rapide.
    pub(super) fn analyser(netlist: &Netlist) -> Option<Plan> {
        let [h] = netlist.horloges[..] else {
            return None;
        };
        if netlist.nb_pilotes().iter().any(|&n| n > 1) || netlist.entrees.contains(&h.net) {
            return None;
        }
        let nb_portes = netlist.portes.len();
        let lect = lecteurs(netlist);

        // Les bascules.
        let mut bascule_de = vec![AUCUN; nb_portes];
        let mut bascules = Vec::new();
        for &inv in &lect[h.net as usize] {
            if bascule_de[inv as usize] != AUCUN {
                continue;
            }
            if let Some((portes, d)) = reconnaitre(netlist, &lect, h.net, inv) {
                if portes.iter().any(|&g| bascule_de[g as usize] != AUCUN) {
                    return None;
                }
                let delais = portes.map(|g| netlist.portes[g as usize].delai);
                let carac = caracteriser(delais, h.demi_periode)?;
                for &g in &portes {
                    bascule_de[g as usize] = bascules.len() as u32;
                }
                let sorties = portes.map(|g| netlist.portes[g as usize].s);
                bascules.push(Bascule {
                    portes,
                    d,
                    sorties,
                    carac,
                });
            }
        }
        // Aucune porte hors d'une bascule ne lit un fil interne d'une bascule.
        for b in &bascules {
            for role in [INV, M1, M2, MQ, MQN, S1, S2] {
                let net = netlist.portes[b.portes[role] as usize].s;
                if lect[net as usize].iter().any(|&g| !b.portes.contains(&g)) {
                    return None;
                }
            }
        }

        // La logique : ordre topologique, sans boucle.
        let mut pilote = vec![AUCUN; netlist.nb_nets as usize];
        for (g, p) in netlist.portes.iter().enumerate() {
            pilote[p.s as usize] = g as u32;
        }
        let logique = |g: u32| g != AUCUN && bascule_de[g as usize] == AUCUN;
        let mut degre = vec![0u32; nb_portes];
        for (g, p) in netlist.portes.iter().enumerate() {
            if logique(g as u32) {
                for n in [p.e0, p.e1] {
                    if logique(pilote[n as usize]) {
                        degre[g] += 1;
                    }
                }
            }
        }
        let mut niveau = vec![0u32; nb_portes];
        let mut ordre: Vec<u32> = (0..nb_portes as u32)
            .filter(|&g| logique(g) && degre[g as usize] == 0)
            .collect();
        for &g in &ordre {
            niveau[g as usize] = 1;
        }
        let mut i = 0;
        while i < ordre.len() {
            let g = ordre[i];
            i += 1;
            for &h in &lect[netlist.portes[g as usize].s as usize] {
                if logique(h) {
                    let hp = &netlist.portes[h as usize];
                    let k = u32::from(hp.e0 == netlist.portes[g as usize].s)
                        + u32::from(hp.e1 == netlist.portes[g as usize].s);
                    let hu = h as usize;
                    niveau[hu] = niveau[hu].max(niveau[g as usize] + 1);
                    degre[hu] -= k;
                    if degre[hu] == 0 {
                        ordre.push(h);
                    }
                }
            }
        }
        let nb_logique = (0..nb_portes as u32).filter(|&g| logique(g)).count();
        if ordre.len() != nb_logique {
            return None;
        }
        let nb_niveaux = niveau.iter().copied().max().unwrap_or(0) as usize;

        // Délais au plus tôt et au plus tard depuis chaque classe de sources.
        let nb_nets = netlist.nb_nets as usize;
        let mut tot = [
            vec![INFINI; nb_nets],
            vec![INFINI; nb_nets],
            vec![INFINI; nb_nets],
        ];
        let mut tard = [
            vec![-INFINI; nb_nets],
            vec![-INFINI; nb_nets],
            vec![-INFINI; nb_nets],
        ];
        for &n in &netlist.entrees {
            tot[ENTREES][n as usize] = 0;
            tard[ENTREES][n as usize] = 0;
        }
        tot[HORLOGE][h.net as usize] = 0;
        tard[HORLOGE][h.net as usize] = 0;
        for b in &bascules {
            for role in [Q, QN] {
                let n = netlist.portes[b.portes[role] as usize].s as usize;
                tot[BASCULES][n] = 0;
                tard[BASCULES][n] = 0;
            }
        }
        for &g in &ordre {
            let p = &netlist.portes[g as usize];
            let d = p.delai as i64;
            for k in 0..3 {
                let a = tot[k][p.e0 as usize].min(tot[k][p.e1 as usize]);
                let z = tard[k][p.e0 as usize].max(tard[k][p.e1 as usize]);
                if a < INFINI {
                    tot[k][p.s as usize] = a + d;
                    tard[k][p.s as usize] = z + d;
                }
            }
        }
        let mut nets_lus: Vec<u32> = Vec::new();
        for &g in &ordre {
            let p = &netlist.portes[g as usize];
            nets_lus.extend([p.e0, p.e1, p.s]);
        }
        nets_lus.extend(bascules.iter().map(|b| b.d));
        nets_lus.sort_unstable();
        nets_lus.dedup();

        // Les lecteurs utiles au mode rapide : ni les portes internes des bascules, ni
        // l'horloge de chacune.
        let mut debut = Vec::with_capacity(nb_nets + 1);
        let mut lecteurs_rapides = Vec::new();
        let mut d_de: Vec<Vec<u32>> = vec![Vec::new(); nb_nets];
        for (b, bascule) in bascules.iter().enumerate() {
            d_de[bascule.d as usize].push(b as u32);
        }
        // La logique rangée par niveau.
        let mut par_niveau = ordre.clone();
        par_niveau.sort_by_key(|&g| niveau[g as usize]);
        let mut rang_logique = vec![AUCUN; nb_portes];
        for (i, &g) in par_niveau.iter().enumerate() {
            rang_logique[g as usize] = i as u32;
        }
        let portes_logiques: Vec<PorteLogique> = par_niveau
            .iter()
            .map(|&g| {
                let p = &netlist.portes[g as usize];
                let tri = if p.genre == GenrePorte::Tri {
                    TROIS_ETATS
                } else {
                    0
                };
                PorteLogique {
                    e0: p.e0,
                    e1: p.e1,
                    s: p.s,
                    porte: g | tri,
                }
            })
            .collect();
        let niveaux: Vec<u32> = par_niveau.iter().map(|&g| niveau[g as usize]).collect();
        for n in 0..nb_nets {
            debut.push(lecteurs_rapides.len() as u32);
            lecteurs_rapides.extend(
                lect[n]
                    .iter()
                    .filter(|&&g| logique(g))
                    .map(|&g| (niveau[g as usize] as u64) << 32 | rang_logique[g as usize] as u64),
            );
            lecteurs_rapides.extend(d_de[n].iter().map(|&b| b as u64));
        }
        debut.push(lecteurs_rapides.len() as u32);
        let mut debut_niveau = vec![0u32; nb_niveaux + 2];
        for &g in &ordre {
            debut_niveau[niveau[g as usize] as usize + 1] += 1;
        }
        for k in 1..debut_niveau.len() {
            debut_niveau[k] += debut_niveau[k - 1];
        }

        let c_min = bascules.iter().map(|b| b.carac.c_min).min().unwrap_or(1) as i64;
        let c_max = bascules.iter().map(|b| b.carac.c_max).max().unwrap_or(1) as i64;
        Some(Plan {
            horloge: h.net,
            demi: h.demi_periode,
            bascules,
            bascule_de,
            logique: portes_logiques,
            rang_logique,
            niveau: niveaux,
            nb_niveaux,
            tot,
            tard,
            nets_lus,
            debut,
            lecteurs: lecteurs_rapides,
            debut_niveau,
            c_min,
            c_max,
            nb_nets,
            compilation: OnceLock::new(),
            lancee: AtomicBool::new(false),
        })
    }

    fn periode(&self) -> u64 {
        2 * self.demi
    }

    /// Les intervalles (décalés depuis l'instant d'arrêt) où le net peut changer pendant
    /// une période, avec pour chacun : sa cause vient-elle après le front montant ?
    /// Les causes sont les entrées (changées à l'instant d'arrêt), chacun des deux
    /// fronts d'horloge et les sorties des bascules (après le front montant).
    fn intervalles(&self, net: u32, montant: i64, descendant: i64) -> Vec<(i64, i64, bool)> {
        let n = net as usize;
        let mut v = Vec::new();
        if self.tot[ENTREES][n] < INFINI {
            v.push((self.tot[ENTREES][n], self.tard[ENTREES][n], false));
        }
        if self.tot[HORLOGE][n] < INFINI {
            for front in [montant, descendant] {
                v.push((
                    front + self.tot[HORLOGE][n],
                    front + self.tard[HORLOGE][n],
                    // Le front montant lui-même vient après la prise de `d`.
                    front >= montant,
                ));
            }
        }
        if self.tot[BASCULES][n] < INFINI {
            v.push((
                montant + self.c_min + self.tot[BASCULES][n],
                montant + self.c_max + self.tard[BASCULES][n],
                true,
            ));
        }
        v
    }

    /// Décalages du prochain front montant et du prochain front descendant après la
    /// phase `phase`, dans `1..=2h`.
    fn fronts(&self, phase: u64) -> (i64, i64) {
        let periode = self.periode();
        let decalage = |cible: u64| {
            let d = (cible + periode - phase) % periode;
            (if d == 0 { periode } else { d }) as i64
        };
        (decalage(self.demi), decalage(0))
    }

    /// Le mode rapide peut-il s'arrêter aux instants de phase `phase` ?
    fn admissible(&self, phase: u64) -> bool {
        let periode = self.periode() as i64;
        if phase == 0 || phase == self.demi {
            // Un front à l'instant d'arrêt : ses effets ne sont pas encore calculés.
            return false;
        }
        let (montant, descendant) = self.fronts(phase);
        // Tout est établi avant l'instant d'arrêt suivant.
        for &n in &self.nets_lus {
            if self
                .intervalles(n, montant, descendant)
                .iter()
                .any(|&(_, fin, _)| fin > periode - 1)
            {
                return false;
            }
        }
        for b in &self.bascules {
            let c = &b.carac;
            let iv = self.intervalles(b.d, montant, descendant);
            // Le mode rapide établit la logique cause par cause, dans l'ordre : `d` est
            // pris au front montant après les causes qui le précèdent. Il faut donc que
            // `d` ait fini de changer pour celles-ci avant la fenêtre de ce front (et
            // après celle du front précédent), et ne change pour les autres qu'après
            // elle (et avant celle du front suivant).
            let (s, h) = (c.avant as i64, c.apres as i64);
            for &(debut, fin, apres_le_front) in &iv {
                let (min, max) = if apres_le_front {
                    (montant + h, montant + periode - s)
                } else {
                    (montant - periode + h, montant - s)
                };
                if debut <= min || fin >= max {
                    return false;
                }
            }
            // Bascule établie à l'instant d'arrêt.
            let dernier = iv.iter().map(|&(_, z, _)| z).max();
            let age = dernier.map_or(INFINI, |z| periode - z);
            let requis = c.age_min[phase as usize];
            if requis == u32::MAX || age < requis as i64 {
                return false;
            }
        }
        true
    }
}

/// L'état du mode rapide dans un simulateur.
#[derive(Clone, Debug)]
pub(super) struct Rapide {
    plan: Arc<Plan>,
    /// Le simulateur est en mode rapide.
    actif: bool,
    admissibles: HashMap<u64, bool>,
    /// Appels à laisser passer avant le prochain essai de passage en mode rapide.
    attente: u32,
    recul: u32,
    /// Périodes calculées depuis le dernier passage en mode rapide.
    periodes: u64,
    /// Portes à évaluer, par niveau (voir `Plan::debut_niveau`), et leur nombre.
    file: Vec<u32>,
    compte: Vec<u32>,
    /// Portes de `Plan::logique` déjà dans la file.
    en_file: Vec<bool>,
    /// Plus haut niveau où la file a des portes.
    niveau_max: usize,
    /// Bascules dont l'entrée `d` a changé depuis le dernier front montant : les
    /// seules qui peuvent changer de valeur au prochain.
    candidats: Vec<u32>,
    candidat: Vec<bool>,
    changees: Vec<u32>,
    /// Valeur mémorisée par chaque bascule.
    v: Vec<u8>,
    touchee: Vec<bool>,
    touchees: Vec<u32>,
    /// Compiler le circuit (voir [`Options::compilation`]).
    compilation: bool,
    /// Périodes calculées en mode rapide depuis la création.
    periodes_total: u64,
    /// Le code compilé, instancié pour ce simulateur.
    execution: Emplacement,
    /// Le code compilé a calculé des périodes depuis la dernière fois que `v`,
    /// `candidats` et `touchees` ont servi : ils sont à reconstituer.
    v_perime: bool,
}

impl Rapide {
    pub(super) fn nouveau(netlist: &Netlist, compilation: bool) -> Option<Box<Rapide>> {
        let plan = Plan::analyser(netlist)?;
        let nb = plan.bascules.len();
        Some(Box::new(Rapide {
            file: vec![0; *plan.debut_niveau.last().unwrap_or(&0) as usize],
            compte: vec![0; plan.nb_niveaux + 1],
            en_file: vec![false; plan.logique.len()],
            niveau_max: 0,
            candidats: Vec::new(),
            candidat: vec![false; nb],
            changees: Vec::new(),
            plan: Arc::new(plan),
            actif: false,
            admissibles: HashMap::new(),
            attente: 0,
            recul: 1,
            periodes: 0,
            v: vec![VX; nb],
            touchee: vec![false; nb],
            touchees: Vec::new(),
            compilation,
            periodes_total: 0,
            execution: Emplacement::default(),
            v_perime: false,
        }))
    }

    pub(super) fn periode(&self) -> u64 {
        self.plan.periode()
    }

    /// L'entrée `d` de la bascule `b` a changé.
    #[inline]
    fn d_change(&mut self, b: u32) {
        self.toucher(b);
        if !self.candidat[b as usize] {
            self.candidat[b as usize] = true;
            self.candidats.push(b);
        }
    }

    #[inline]
    fn toucher(&mut self, b: u32) {
        if !self.touchee[b as usize] {
            self.touchee[b as usize] = true;
            self.touchees.push(b);
        }
    }
}

impl Simulateur {
    /// Le simulateur est-il en mode rapide ?
    #[inline]
    pub(super) fn en_rapide(&self) -> bool {
        self.rapide.as_ref().is_some_and(|r| r.actif)
    }

    /// Le circuit se prête-t-il au mode rapide, et à combien de phases d'arrêt sur
    /// une période ? `None` s'il ne s'y prête pas. Sert aux essais.
    #[doc(hidden)]
    pub fn phases_rapides(&self) -> Option<usize> {
        let plan = &self.rapide.as_ref()?.plan;
        Some((0..plan.periode()).filter(|&p| plan.admissible(p)).count())
    }

    /// Repasse en mode exact. L'état est déjà exactement celui du calcul porte par
    /// porte : il n'y a rien à reconstituer.
    pub(super) fn quitter_rapide(&mut self) {
        self.etablir_etats();
        if let Some(r) = self.rapide.as_mut() {
            if r.actif {
                r.actif = false;
                if let Some(e) = r.execution.0.as_mut() {
                    e.synchro = false;
                }
                if r.periodes == 0 {
                    // Passé en mode rapide pour rien : on attendra plus longtemps.
                    r.recul = (r.recul * 2).min(RECUL_MAX);
                    r.attente = r.recul;
                } else {
                    r.recul = 1;
                }
            }
        }
    }

    /// À la fin d'un appel en mode exact : passe en mode rapide si l'instant courant
    /// s'y prête et si l'état exact est l'état établi que le mode rapide suppose.
    pub(super) fn essayer_rapide(&mut self) {
        let Some(mut rap) = self.rapide.take() else {
            return;
        };
        // Les contrôles qui ne coûtent rien d'abord ; un échec de ceux qui parcourent
        // tout le circuit fait attendre de plus en plus longtemps avant le suivant.
        if !rap.actif
            && self.sondes.is_empty()
            && self.nb_suiv == 0
            && self.nb_attente == 0
            && self.phase_admissible(&mut rap)
        {
            if rap.attente > 0 {
                rap.attente -= 1;
            } else if self.portes_etablies(true) && {
                // Les régions en sommeil reprennent leur état exact et se réveillent :
                // le mode rapide calcule toutes les portes.
                self.reinitialiser_sommeil();
                self.nb_suiv == 0 && self.nb_attente == 0 && self.etat_etabli(&rap)
            } {
                // Aucune porte n'a de changement programmé : la roue ne contient que
                // des événements périmés.
                for case in &mut self.roue {
                    case.clear();
                }
                self.nb_dans_roue = 0;
                let plan = rap.plan.clone();
                for (b, bascule) in plan.bascules.iter().enumerate() {
                    rap.v[b] = (self.portes[bascule.portes[Q] as usize].etat & SORTIE) as u8;
                }
                rap.actif = true;
                rap.periodes = 0;
                rap.candidats.clear();
                for b in 0..plan.bascules.len() as u32 {
                    rap.candidat[b as usize] = true;
                    rap.candidats.push(b);
                }
            } else {
                rap.recul = (rap.recul * 2).min(RECUL_MAX);
                rap.attente = rap.recul;
            }
        }
        self.rapide = Some(rap);
    }

    fn phase_admissible(&self, rap: &mut Rapide) -> bool {
        let plan = &rap.plan;
        let phase = self.t % plan.periode();
        *rap.admissibles
            .entry(phase)
            .or_insert_with(|| plan.admissible(phase))
    }

    /// Chaque porte est établie (sortie = cible = fonction de ses entrées, rien de
    /// programmé), à part celles des régions en sommeil si `sauf_endormies`.
    fn portes_etablies(&self, sauf_endormies: bool) -> bool {
        let ignorees = if sauf_endormies { ENDORMIE } else { 0 };
        self.portes.iter().all(|p| {
            let e = p.etat;
            let sortie = e & SORTIE;
            let a = self.val[p.e0 as usize];
            let b = self.val[p.e1 as usize];
            let vraie = TABLE[((e & TRI) >> 7) as usize | ((a << 2 | b) & 15) as usize] as u16;
            e & ignorees != 0
                || (e & (ATTENTE | MIXTE) == 0 && (e & CIBLE) >> 2 == sortie && vraie == sortie)
        })
    }

    /// Chaque porte est établie, et chaque bascule est dans l'état établi de sa valeur
    /// mémorisée, un état que l'exploration de la bascule a rencontré à cette phase.
    fn etat_etabli(&self, rap: &Rapide) -> bool {
        let plan = &rap.plan;
        if !self.portes_etablies(false) {
            return false;
        }
        let phase = self.t % plan.periode();
        let niveau = usize::from(phase >= plan.demi);
        plan.bascules.iter().all(|b| {
            let v = (self.portes[b.portes[Q] as usize].etat & SORTIE) as u8;
            let u = rang(self.val[b.d as usize]);
            v != VZ
                && b.carac.atteignable[phase as usize] & (1 << (3 * u + v as usize)) != 0
                && (0..9).all(|k| {
                    (self.portes[b.portes[k] as usize].etat & SORTIE) as u8
                        == b.carac.canon[niveau][u][v as usize][k]
                })
        })
    }

    /// Avance de `n` périodes d'horloge en mode rapide.
    pub(super) fn periodes_rapides(&mut self, n: u64) {
        let mut rap = self.rapide.take().expect("mode rapide actif");
        let plan = rap.plan.clone();
        rap.periodes_total += n;
        if !(rap.compilation && self.periodes_compilees(&mut rap, &plan, n)) {
            if let Some(e) = rap.execution.0.as_mut() {
                e.synchro = false;
            }
            if rap.v_perime {
                // Le code compilé a fait avancer les bascules : leur valeur est celle
                // de leur sortie `q`, et chacune peut changer au prochain front.
                rap.v_perime = false;
                rap.candidats.clear();
                rap.candidat.fill(false);
                for (b, bascule) in plan.bascules.iter().enumerate() {
                    rap.v[b] = self.val[bascule.sorties[Q] as usize];
                    rap.d_change(b as u32);
                }
            }
            for _ in 0..n {
                self.periode_rapide(&mut rap, &plan);
            }
        }
        rap.periodes += n;
        self.rapide = Some(rap);
    }

    /// Compile le circuit tout de suite si ce n'est pas déjà fait (sinon, la
    /// compilation se lance d'elle-même, en arrière-plan, quand le circuit a passé
    /// assez de temps en mode rapide). Rend vrai si le code compilé est disponible.
    /// Sert aux essais.
    #[doc(hidden)]
    pub fn compiler(&mut self) -> bool {
        let Some(rap) = self.rapide.as_mut() else {
            return false;
        };
        rap.compilation = true;
        let plan = rap.plan.clone();
        plan.lancee.store(true, Ordering::Relaxed);
        plan.compilation
            .get_or_init(|| compile::compiler(&plan))
            .is_some()
    }

    /// Cesse d'utiliser le code compilé : les périodes suivantes sont calculées par le
    /// mode rapide événementiel, jusqu'au prochain appel de [`Simulateur::compiler`].
    /// Sert aux essais du passage d'un calcul à l'autre.
    #[doc(hidden)]
    pub fn ne_plus_compiler(&mut self) {
        if let Some(rap) = self.rapide.as_mut() {
            rap.compilation = false;
            rap.execution.0 = None;
        }
    }

    /// Calcule `n` périodes avec le code compilé ; faux s'il n'est pas (encore)
    /// disponible.
    fn periodes_compilees(&mut self, rap: &mut Rapide, plan: &Arc<Plan>, n: u64) -> bool {
        if rap.execution.0.is_none() {
            let compile = match plan.compilation.get() {
                Some(Some(c)) => c.clone(),
                Some(None) => return false,
                None => {
                    if rap.periodes_total >= SEUIL_COMPILATION
                        && !plan.lancee.swap(true, Ordering::Relaxed)
                    {
                        let p = plan.clone();
                        // Si le fil ne peut pas être créé, le circuit reste simplement
                        // en mode rapide événementiel.
                        let _ = std::thread::Builder::new()
                            .name("compilation du circuit".into())
                            .spawn(move || {
                                p.compilation.get_or_init(|| compile::compiler(&p));
                            });
                    }
                    return false;
                }
            };
            rap.execution.0 = Execution::nouvelle(compile);
        }
        let Some(e) = rap.execution.0.as_mut() else {
            return false;
        };
        let periode = plan.periode();
        let phase = self.t % periode;
        let niveau = u32::from(phase >= plan.demi);
        let (montant, descendant) = plan.fronts(phase);
        let nb_nets = self.val.len();
        let c = e.compile.clone();
        let synchro = e.synchro;
        let m = e.memoire();
        if !synchro {
            // Toute la mémoire repart de l'état du simulateur. Chaque bascule y est
            // dans l'état établi de sa valeur `q`, mais son entrée a pu changer depuis
            // (une entrée externe, à l'instant d'arrêt) : son état interne sera posé à
            // la fin de la première période dans tous les cas (clé impossible).
            m[VAL_ADR..VAL_ADR + nb_nets].copy_from_slice(&self.val);
            c.lever_tout(m);
            // Tous les groupes candidats et touchés, aucun « pris ».
            let g = c.candidats;
            m[g..g + c.nb_groupes].fill(1);
            m[g + c.nb_groupes..g + 2 * c.nb_groupes].fill(0);
            m[g + 2 * c.nb_groupes..g + 3 * c.nb_groupes].fill(1);
            for (b, bascule) in plan.bascules.iter().enumerate() {
                m[c.v + b] = self.val[bascule.sorties[Q] as usize];
                m[c.prec + b] = u8::MAX;
            }
        } else {
            // Seules les entrées ont pu changer depuis le dernier appel.
            for &net in &self.entrees {
                m[VAL_ADR + net as usize] = self.val[net as usize];
            }
        }
        for i in 0..self.nb_sales {
            let g = self.sales[i] as usize;
            self.portes[g].etat &= !SALE;
            let l = plan.rang_logique[g];
            if l != AUCUN {
                c.lever(m, c.bloc_de[l as usize] as usize);
            } else {
                // Une porte de bascule qui lit une entrée : c'est son entrée `d`.
                let k = c.candidats + Compile::groupe(plan.bascule_de[g] as usize);
                m[k] = 1;
                m[k + 2 * c.nb_groupes] = 1;
            }
        }
        self.nb_sales = 0;
        e.synchro = true;
        let evaluations = e.periodes(n, montant < descendant, niveau);
        self.val
            .copy_from_slice(&e.memoire()[VAL_ADR..VAL_ADR + nb_nets]);
        rap.v_perime = true;
        self.etats_perimes = true;

        let duree = n * periode;
        self.t += duree;
        for h in &mut self.horloges {
            h.prochain_front += duree;
        }
        self.prochain_front += duree;
        self.activite = true;
        self.stats.evaluations += evaluations;
        self.stats.cycles_rapides += n;
        self.stats.cycles_compiles += n;
        true
    }

    /// Après le code compilé, seules les valeurs des nets sont à jour : chaque porte,
    /// établie, a pour sortie et pour cible la valeur de son net de sortie.
    pub(super) fn etablir_etats(&mut self) {
        if self.etats_perimes {
            self.etats_perimes = false;
            for p in &mut self.portes {
                let v = self.val[p.s as usize] as u16;
                p.etat = (p.etat & !(SORTIE | CIBLE)) | v | v << 2;
            }
        }
    }

    fn periode_rapide(&mut self, rap: &mut Rapide, plan: &Plan) {
        let periode = plan.periode();
        let phase = self.t % periode;
        let mut evaluations = 0u64;

        // Les portes qui lisent une entrée changée à l'instant d'arrêt.
        for i in 0..self.nb_sales {
            let g = self.sales[i];
            self.portes[g as usize].etat &= !SALE;
            let l = plan.rang_logique[g as usize];
            if l != AUCUN {
                rap.enfiler(plan, l, plan.niveau[l as usize]);
            } else {
                rap.d_change(plan.bascule_de[g as usize]);
            }
        }
        self.nb_sales = 0;
        self.propager(rap, plan, &mut evaluations);

        let (montant, descendant) = plan.fronts(phase);
        let mut fronts = [(montant, true), (descendant, false)];
        fronts.sort_unstable();
        for (_, est_montant) in fronts {
            if est_montant {
                // Chaque bascule prend la valeur de son entrée, toutes en même temps.
                // Seules celles dont l'entrée a changé peuvent changer.
                let mut changees = std::mem::take(&mut rap.changees);
                for &b in &rap.candidats {
                    rap.candidat[b as usize] = false;
                    let nv = capture(self.val[plan.bascules[b as usize].d as usize]);
                    if nv != rap.v[b as usize] {
                        rap.v[b as usize] = nv;
                        changees.push(b);
                    }
                }
                rap.candidats.clear();
                for &b in &changees {
                    let bascule = &plan.bascules[b as usize];
                    let v = rap.v[b as usize];
                    let q = self.portes[bascule.portes[Q] as usize].s;
                    let qn = self.portes[bascule.portes[QN] as usize].s;
                    self.changer_net_rapide(rap, plan, q, v);
                    self.changer_net_rapide(rap, plan, qn, inverse(v));
                    rap.toucher(b);
                }
                changees.clear();
                rap.changees = changees;
                self.changer_net_rapide(rap, plan, plan.horloge, 1);
            } else {
                self.changer_net_rapide(rap, plan, plan.horloge, 0);
            }
            self.propager(rap, plan, &mut evaluations);
        }

        // État interne établi des bascules qui ont bougé.
        let niveau = usize::from(phase >= plan.demi);
        for &b in &rap.touchees {
            rap.touchee[b as usize] = false;
            let bascule = &plan.bascules[b as usize];
            let attendu = bascule.carac.canon[niveau][rang(self.val[bascule.d as usize])]
                [rap.v[b as usize] as usize];
            for (k, &g) in bascule.portes.iter().enumerate() {
                let p = &mut self.portes[g as usize];
                let v = attendu[k] as u16;
                p.etat = (p.etat & !(SORTIE | CIBLE)) | v | v << 2;
                self.val[p.s as usize] = attendu[k];
            }
        }
        rap.touchees.clear();

        self.t += periode;
        for h in &mut self.horloges {
            h.prochain_front += periode;
        }
        self.prochain_front += periode;
        self.activite = true;
        self.stats.evaluations += evaluations;
        self.stats.cycles_rapides += 1;
    }

    /// Le net prend la valeur `v` ; ses lecteurs dans la logique sont à réévaluer.
    #[inline(always)]
    fn changer_net_rapide(&mut self, rap: &mut Rapide, plan: &Plan, net: u32, v: u8) {
        let n = net as usize;
        if self.val[n] == v {
            return;
        }
        self.val[n] = v;
        rap.prevenir(plan, net);
    }

    /// Établit la logique, niveau par niveau : chaque porte est évaluée une fois,
    /// après toutes celles dont elle dépend.
    fn propager(&mut self, rap: &mut Rapide, plan: &Plan, evaluations: &mut u64) {
        let mut niveau = 1;
        while niveau <= rap.niveau_max {
            let debut = plan.debut_niveau[niveau] as usize;
            // Une porte n'ajoute que des portes de niveau supérieur : la liste de ce
            // niveau ne bouge plus pendant qu'on la parcourt.
            let n = std::mem::take(&mut rap.compte[niveau]) as usize;
            *evaluations += n as u64;
            for i in debut..debut + n {
                let l = rap.file[i];
                rap.en_file[l as usize] = false;
                let p = plan.logique[l as usize];
                let a = self.val[p.e0 as usize];
                let b = self.val[p.e1 as usize];
                let cible = TABLE[(p.porte >> 27 & 16) as usize | ((a << 2 | b) & 15) as usize];
                // La porte était établie : sa sortie est la valeur de son net.
                if cible != self.val[p.s as usize] {
                    self.val[p.s as usize] = cible;
                    let g = (p.porte & !TROIS_ETATS) as usize;
                    let c = cible as u16;
                    self.portes[g].etat = (self.portes[g].etat & !(SORTIE | CIBLE)) | c | c << 2;
                    rap.prevenir(plan, p.s);
                }
            }
            niveau += 1;
        }
        rap.niveau_max = 0;
    }
}

impl Rapide {
    #[inline(always)]
    fn enfiler(&mut self, plan: &Plan, l: u32, niveau: u32) {
        if !self.en_file[l as usize] {
            self.en_file[l as usize] = true;
            let k = niveau as usize;
            self.file[(plan.debut_niveau[k] + self.compte[k]) as usize] = l;
            self.compte[k] += 1;
            self.niveau_max = self.niveau_max.max(k);
        }
    }

    /// Le net `net` vient de changer : ses lecteurs sont à réévaluer.
    #[inline(always)]
    fn prevenir(&mut self, plan: &Plan, net: u32) {
        let n = net as usize;
        let (debut, fin) = (plan.debut[n] as usize, plan.debut[n + 1] as usize);
        for &l in &plan.lecteurs[debut..fin] {
            let niveau = (l >> 32) as u32;
            if niveau != 0 {
                self.enfiler(plan, l as u32, niveau);
            } else {
                // L'entrée d de la bascule a changé.
                self.d_change(l as u32);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_bascule_de_la_bibliotheque_est_caracterisee() {
        let c = caracteriser([1; 9], 64).expect("la bascule D se caractérise");
        assert!(c.c_min >= 2 && c.c_max < 64, "{c:?}");
        assert!(c.avant + c.apres <= 12);
        assert_eq!((c.avant, c.apres, c.c_min, c.c_max), (1, 0, 2, 6));
        // Au milieu de chaque niveau, l'état établi est atteint.
        assert!(c.age_min[32] < u32::MAX && c.age_min[96] < u32::MAX);
        for demi in [8, 16] {
            for delais in [[2, 1, 1, 1, 1, 1, 1, 1, 1], [1, 2, 1, 1, 2, 1, 2, 1, 1]] {
                assert!(caracteriser(delais, demi).is_some(), "{delais:?} {demi}");
            }
        }
    }
}
