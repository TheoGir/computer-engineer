//! Mise à plat d'un composant hiérarchique en netlist simulable.
//!
//! La mise à plat se fait en deux temps.
//!
//! 1. **Compilation** de chaque composant, une seule fois : ses fils sont
//!    regroupés en *nets locaux* (deux extrémités reliées, même par plusieurs
//!    fils, sont le même net), et chaque net local est soit *attaché à une
//!    broche*, soit *interne*.
//! 2. **Élaboration** : parcours en profondeur de l'arbre des instances. Les nets
//!    internes de chaque instance reçoivent des numéros consécutifs, de sorte que
//!    le numéro d'un net se recalcule à partir de son chemin sans rien stocker par
//!    instance. Un circuit de plusieurs millions de portes garde ainsi une carte
//!    de taille proportionnelle au nombre de *définitions*, pas d'instances.
//!
//! Un composant peut relier deux de ses broches entre elles (un simple fil qui le
//! traverse) : les deux nets du parent ne font alors plus qu'un. Ce cas est géré
//! par une structure union-find, allouée seulement si un tel composant existe.

use crate::erreur::Erreur;
use crate::modele::{Bibliotheque, Composant, Sens, PRIMITIVES};
use ce_moteur::{GenrePorte, NetId, Netlist, Porte, Valeur};
use std::collections::HashMap;

const AUCUN: u32 = u32::MAX;

/// À quoi un net local d'un composant est rattaché.
#[derive(Clone, Copy, Debug)]
enum Attache {
    /// Au bit de broche donné (le premier, si le net touche plusieurs broches).
    Broche(u32),
    /// À rien d'extérieur : c'est le net interne de rang donné.
    Interne(u32),
}

#[derive(Clone, Debug)]
enum Enfant {
    Porte {
        genre: GenrePorte,
        e0: u32,
        e1: u32,
        s: u32,
        delai: u16,
    },
    Constante {
        s: u32,
        valeur: Valeur,
    },
    Horloge {
        s: u32,
        demi_periode: u64,
    },
    Sous {
        def: usize,
        /// Net local relié à chaque bit de broche de l'enfant.
        bits: Vec<u32>,
        /// Décalage des nets internes de l'enfant parmi ceux des enfants.
        base: u64,
    },
}

#[derive(Clone, Debug)]
struct BrocheDef {
    nom: String,
    sens: Sens,
    largeur: u32,
    debut: u32,
}

/// Un composant compilé.
#[derive(Clone, Debug)]
struct Def {
    nom: String,
    broches: Vec<BrocheDef>,
    index_broches: HashMap<String, usize>,
    nb_bits: u32,
    /// Net local de chaque bit de broche.
    bit_local: Vec<u32>,
    attache: Vec<Attache>,
    /// Paires de bits de broches reliés entre eux à l'intérieur du composant.
    fusions: Vec<(u32, u32)>,
    enfants: Vec<Enfant>,
    noms_instances: Vec<String>,
    index_instances: HashMap<String, usize>,
    nb_internes: u32,
    /// Nombre de nets internes de tout le sous-arbre.
    total_nets: u64,
    /// Nombre de portes de tout le sous-arbre.
    total_portes: u64,
    /// Vrai si ce composant ou l'un de ses descendants relie deux broches entre elles.
    a_fusions: bool,
    entrees_flottantes: Vec<String>,
}

/// Résultat de la compilation d'un composant et de tout ce dont il dépend.
#[derive(Clone, Debug)]
pub(crate) struct Compilation {
    defs: Vec<Def>,
    racine: usize,
}

/// Broches d'une primitive : (nom, sens).
fn broches_primitive(genre: &str) -> Option<&'static [(&'static str, Sens)]> {
    match genre {
        "NAND" => Some(&[
            ("a", Sens::Entree),
            ("b", Sens::Entree),
            ("s", Sens::Sortie),
        ]),
        "TRI" => Some(&[
            ("e", Sens::Entree),
            ("a", Sens::Entree),
            ("s", Sens::Sortie),
        ]),
        "ZERO" | "UN" | "HORLOGE" => Some(&[("s", Sens::Sortie)]),
        _ => None,
    }
}

fn nom_valide(nom: &str) -> bool {
    !nom.is_empty()
        && !nom
            .chars()
            .any(|c| matches!(c, '.' | '[' | ']' | ':') || c.is_whitespace())
}

/// Une extrémité de fil découpée : instance éventuelle, broche, plage éventuelle.
struct Point<'a> {
    instance: Option<&'a str>,
    broche: &'a str,
    plage: Option<(u32, u32)>,
}

fn analyser_point(texte: &str) -> Result<Point<'_>, String> {
    let (corps, plage) = match texte.find('[') {
        None => (texte, None),
        Some(i) => {
            let reste = &texte[i + 1..];
            let Some(dedans) = reste.strip_suffix(']') else {
                return Err("crochet fermant manquant".to_string());
            };
            let lire = |s: &str| {
                s.parse::<u32>()
                    .map_err(|_| format!("« {s} » n'est pas un indice de bit"))
            };
            let plage = match dedans.split_once(':') {
                None => {
                    let k = lire(dedans)?;
                    (k, k)
                }
                Some((a, b)) => (lire(a)?, lire(b)?),
            };
            (&texte[..i], Some(plage))
        }
    };
    let (instance, broche) = match corps.split_once('.') {
        None => (None, corps),
        Some((i, b)) => (Some(i), b),
    };
    if broche.is_empty() || broche.contains('.') || instance == Some("") {
        return Err("attendu « broche » ou « instance.broche »".to_string());
    }
    Ok(Point {
        instance,
        broche,
        plage,
    })
}

fn bits_de_plage(
    composant: &str,
    texte: &str,
    plage: Option<(u32, u32)>,
    largeur: u32,
) -> Result<Vec<u32>, Erreur> {
    match plage {
        None => Ok((0..largeur).collect()),
        Some((a, b)) => {
            if a >= largeur || b >= largeur {
                return Err(Erreur::BitHorsBornes {
                    composant: composant.to_string(),
                    point: texte.to_string(),
                    largeur,
                });
            }
            Ok(if a <= b {
                (a..=b).collect()
            } else {
                (b..=a).rev().collect()
            })
        }
    }
}

enum TypeInstance {
    Primitive(&'static [(&'static str, Sens)]),
    Def(usize),
}

struct InfoInstance {
    ty: TypeInstance,
    /// Rang du premier bit de broche de l'instance parmi les points du composant.
    debut: u32,
}

struct Compilateur<'a> {
    bib: &'a Bibliotheque,
    defs: Vec<Def>,
    index: HashMap<String, usize>,
    pile: Vec<String>,
}

fn trouver(uf: &mut [u32], mut x: u32) -> u32 {
    while uf[x as usize] != x {
        uf[x as usize] = uf[uf[x as usize] as usize];
        x = uf[x as usize];
    }
    x
}

fn unir(uf: &mut [u32], a: u32, b: u32) {
    let (ra, rb) = (trouver(uf, a), trouver(uf, b));
    if ra != rb {
        // La racine est toujours le plus petit numéro : le résultat est déterministe.
        let (petit, grand) = if ra < rb { (ra, rb) } else { (rb, ra) };
        uf[grand as usize] = petit;
    }
}

impl<'a> Compilateur<'a> {
    fn visiter(&mut self, nom: &str, depuis: Option<&str>) -> Result<usize, Erreur> {
        if let Some(&i) = self.index.get(nom) {
            return Ok(i);
        }
        if let Some(pos) = self.pile.iter().position(|n| n == nom) {
            let mut cycle: Vec<String> = self.pile[pos..].to_vec();
            cycle.push(nom.to_string());
            return Err(Erreur::DependanceCirculaire(cycle));
        }
        if PRIMITIVES.contains(&nom) {
            return Err(Erreur::NomReserve(nom.to_string()));
        }
        let Some(composant) = self.bib.composants.get(nom) else {
            return Err(Erreur::ComposantInconnu {
                nom: nom.to_string(),
                dans: depuis.map(str::to_string),
            });
        };
        if !nom_valide(nom) {
            return Err(Erreur::NomInvalide {
                composant: nom.to_string(),
                nom: nom.to_string(),
            });
        }
        self.pile.push(nom.to_string());
        let def = self.compiler_un(nom, composant)?;
        self.pile.pop();
        let i = self.defs.len();
        self.defs.push(def);
        self.index.insert(nom.to_string(), i);
        Ok(i)
    }

    fn compiler_un(&mut self, nom: &str, comp: &Composant) -> Result<Def, Erreur> {
        let invalide = |n: &str| Erreur::NomInvalide {
            composant: nom.to_string(),
            nom: n.to_string(),
        };
        let double = |n: &str| Erreur::NomEnDouble {
            composant: nom.to_string(),
            nom: n.to_string(),
        };

        // Broches du composant : ses bits sont les premiers points.
        let mut broches = Vec::with_capacity(comp.broches.len());
        let mut index_broches = HashMap::new();
        let mut nb_bits = 0u32;
        for b in &comp.broches {
            if !nom_valide(&b.nom) {
                return Err(invalide(&b.nom));
            }
            if b.largeur == 0 {
                return Err(Erreur::LargeurNulle {
                    composant: nom.to_string(),
                    broche: b.nom.clone(),
                });
            }
            if index_broches.insert(b.nom.clone(), broches.len()).is_some() {
                return Err(double(&b.nom));
            }
            broches.push(BrocheDef {
                nom: b.nom.clone(),
                sens: b.sens,
                largeur: b.largeur,
                debut: nb_bits,
            });
            nb_bits = nb_bits.checked_add(b.largeur).ok_or(Erreur::TropGrand)?;
        }

        // Instances : leurs bits de broches sont les points suivants.
        let mut infos = Vec::with_capacity(comp.instances.len());
        let mut index_instances = HashMap::new();
        let mut nb_points = nb_bits;
        for (i, inst) in comp.instances.iter().enumerate() {
            if !nom_valide(&inst.nom) {
                return Err(invalide(&inst.nom));
            }
            if index_instances.insert(inst.nom.clone(), i).is_some() {
                return Err(double(&inst.nom));
            }
            let parametre = |raison: &str| Erreur::Parametre {
                composant: nom.to_string(),
                instance: inst.nom.clone(),
                raison: raison.to_string(),
            };
            let genre = inst.genre.as_str();
            let avec_delai = matches!(genre, "NAND" | "TRI");
            if inst.delai.is_some() && !avec_delai {
                return Err(parametre("« delai » ne s'applique qu'à NAND et TRI"));
            }
            if inst.delai == Some(0) {
                return Err(parametre("le délai doit valoir au moins 1"));
            }
            if genre == "HORLOGE" {
                match inst.demi_periode {
                    None => return Err(parametre("une horloge exige « demi_periode »")),
                    Some(0) => return Err(parametre("la demi-période doit valoir au moins 1")),
                    Some(_) => {}
                }
            } else if inst.demi_periode.is_some() {
                return Err(parametre("« demi_periode » ne s'applique qu'à HORLOGE"));
            }
            let (ty, nb) = match broches_primitive(genre) {
                Some(p) => (TypeInstance::Primitive(p), p.len() as u32),
                None => {
                    let d = self.visiter(genre, Some(nom))?;
                    (TypeInstance::Def(d), self.defs[d].nb_bits)
                }
            };
            infos.push(InfoInstance {
                ty,
                debut: nb_points,
            });
            nb_points = nb_points.checked_add(nb).ok_or(Erreur::TropGrand)?;
        }

        // Points désignés par une extrémité de fil.
        let points = |texte: &str| -> Result<Vec<u32>, Erreur> {
            let p = analyser_point(texte).map_err(|raison| Erreur::Syntaxe {
                composant: nom.to_string(),
                point: texte.to_string(),
                raison,
            })?;
            let inconnue = || Erreur::BrocheInconnue {
                composant: nom.to_string(),
                point: texte.to_string(),
            };
            let (debut, largeur) = match p.instance {
                None => {
                    let b = &broches[*index_broches.get(p.broche).ok_or_else(inconnue)?];
                    (b.debut, b.largeur)
                }
                Some(instance) => {
                    let Some(&i) = index_instances.get(instance) else {
                        return Err(Erreur::InstanceInconnue {
                            composant: nom.to_string(),
                            instance: instance.to_string(),
                        });
                    };
                    let info: &InfoInstance = &infos[i];
                    match &info.ty {
                        TypeInstance::Primitive(liste) => {
                            let k = liste
                                .iter()
                                .position(|(n, _)| *n == p.broche)
                                .ok_or_else(inconnue)?;
                            (info.debut + k as u32, 1)
                        }
                        TypeInstance::Def(d) => {
                            let def = &self.defs[*d];
                            let b = &def.broches
                                [*def.index_broches.get(p.broche).ok_or_else(inconnue)?];
                            (info.debut + b.debut, b.largeur)
                        }
                    }
                }
            };
            Ok(bits_de_plage(nom, texte, p.plage, largeur)?
                .into_iter()
                .map(|k| debut + k)
                .collect())
        };

        // Regroupement des points reliés.
        let mut uf: Vec<u32> = (0..nb_points).collect();
        for fil in &comp.fils {
            let mut listes = Vec::with_capacity(fil.len());
            for texte in fil {
                listes.push(points(texte)?);
            }
            let Some(premiere) = listes.first() else {
                continue;
            };
            if listes.iter().any(|l| l.len() != premiere.len()) {
                return Err(Erreur::LargeursDifferentes {
                    composant: nom.to_string(),
                    fil: fil.clone(),
                });
            }
            for autre in &listes[1..] {
                for (&a, &b) in premiere.iter().zip(autre) {
                    unir(&mut uf, a, b);
                }
            }
        }

        // Numérotation des nets locaux. Les bits de broches du composant sont
        // parcourus en premier : un net qui en touche un lui est attaché.
        let mut local_de_racine = vec![AUCUN; nb_points as usize];
        let mut taille_classe = vec![0u32; nb_points as usize];
        let mut point_local = vec![0u32; nb_points as usize];
        let mut attache = Vec::new();
        let mut fusions = Vec::new();
        let mut nb_internes = 0u32;
        for p in 0..nb_points {
            let r = trouver(&mut uf, p) as usize;
            taille_classe[r] += 1;
            if local_de_racine[r] == AUCUN {
                local_de_racine[r] = attache.len() as u32;
                if p < nb_bits {
                    attache.push(Attache::Broche(p));
                } else {
                    attache.push(Attache::Interne(nb_internes));
                    nb_internes += 1;
                }
            } else if p < nb_bits {
                if let Attache::Broche(premier) = attache[local_de_racine[r] as usize] {
                    fusions.push((premier, p));
                }
            }
            point_local[p as usize] = local_de_racine[r];
        }

        // Enfants, totaux et entrées laissées en l'air.
        let mut enfants = Vec::with_capacity(comp.instances.len());
        let mut total_nets = nb_internes as u64;
        let mut total_portes = 0u64;
        let mut base = 0u64;
        let mut a_fusions = !fusions.is_empty();
        let mut entrees_flottantes = Vec::new();
        for (inst, info) in comp.instances.iter().zip(&infos) {
            let local = |k: u32| point_local[(info.debut + k) as usize];
            let mut signaler = |k: u32, nom_broche: &str, largeur: u32, bit: u32| {
                let r = trouver(&mut uf, info.debut + k) as usize;
                if taille_classe[r] == 1 {
                    entrees_flottantes.push(if largeur == 1 {
                        format!("{}.{}", inst.nom, nom_broche)
                    } else {
                        format!("{}.{}[{}]", inst.nom, nom_broche, bit)
                    });
                }
            };
            match &info.ty {
                TypeInstance::Primitive(liste) => {
                    for (k, (nom_broche, sens)) in liste.iter().enumerate() {
                        if *sens == Sens::Entree {
                            signaler(k as u32, nom_broche, 1, 0);
                        }
                    }
                    let delai = inst.delai.unwrap_or(1);
                    enfants.push(match inst.genre.as_str() {
                        "NAND" => {
                            total_portes += 1;
                            Enfant::Porte {
                                genre: GenrePorte::Nand,
                                e0: local(0),
                                e1: local(1),
                                s: local(2),
                                delai,
                            }
                        }
                        "TRI" => {
                            total_portes += 1;
                            Enfant::Porte {
                                genre: GenrePorte::Tri,
                                e0: local(0),
                                e1: local(1),
                                s: local(2),
                                delai,
                            }
                        }
                        "ZERO" => Enfant::Constante {
                            s: local(0),
                            valeur: Valeur::Zero,
                        },
                        "UN" => Enfant::Constante {
                            s: local(0),
                            valeur: Valeur::Un,
                        },
                        _ => Enfant::Horloge {
                            s: local(0),
                            demi_periode: inst.demi_periode.expect("vérifié plus haut"),
                        },
                    });
                }
                TypeInstance::Def(d) => {
                    let def = &self.defs[*d];
                    for b in &def.broches {
                        if b.sens == Sens::Entree {
                            for bit in 0..b.largeur {
                                signaler(b.debut + bit, &b.nom, b.largeur, bit);
                            }
                        }
                    }
                    enfants.push(Enfant::Sous {
                        def: *d,
                        bits: (0..def.nb_bits).map(local).collect(),
                        base,
                    });
                    base += def.total_nets;
                    total_nets += def.total_nets;
                    total_portes += def.total_portes;
                    a_fusions |= def.a_fusions;
                }
            }
        }

        Ok(Def {
            nom: nom.to_string(),
            bit_local: point_local[..nb_bits as usize].to_vec(),
            broches,
            index_broches,
            nb_bits,
            attache,
            fusions,
            enfants,
            noms_instances: comp.instances.iter().map(|i| i.nom.clone()).collect(),
            index_instances,
            nb_internes,
            total_nets,
            total_portes,
            a_fusions,
            entrees_flottantes,
        })
    }
}

/// Compile `racine` et tous les composants dont il dépend.
pub(crate) fn compiler(bib: &Bibliotheque, racine: &str) -> Result<Compilation, Erreur> {
    let mut c = Compilateur {
        bib,
        defs: Vec::new(),
        index: HashMap::new(),
        pile: Vec::new(),
    };
    let racine = c.visiter(racine, None)?;
    Ok(Compilation {
        defs: c.defs,
        racine,
    })
}

/// Une broche du composant mis à plat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrocheCircuit {
    /// Nom de la broche.
    pub nom: String,
    /// Entrée ou sortie.
    pub sens: Sens,
    /// Net de chaque bit, bit 0 d'abord.
    pub nets: Vec<NetId>,
    /// Pour une entrée : numéro, dans le simulateur, de l'entrée externe du bit 0
    /// (les bits suivants ont les numéros suivants).
    pub premiere_entree: Option<usize>,
}

/// Un composant mis à plat, prêt à être simulé.
#[derive(Clone, Debug)]
pub struct Circuit {
    /// Nom du composant mis à plat.
    pub nom: String,
    /// La netlist à donner au simulateur.
    pub netlist: Netlist,
    /// Les broches du composant.
    pub broches: Vec<BrocheCircuit>,
    /// Remarques non bloquantes (entrées laissées en l'air).
    pub avertissements: Vec<String>,
    comp: Compilation,
    /// Numéro définitif de chaque net provisoire, si des broches sont reliées entre elles.
    canon: Option<Vec<u32>>,
}

struct Elaboration<'a> {
    defs: &'a [Def],
    netlist: Netlist,
    uf: Option<Vec<u32>>,
}

/// Taille, en portes, des sous-composants signalés au simulateur comme des unités
/// qu'il peut mettre en sommeil ([`Netlist::regions`]). Un registre de 8 bits et un
/// mot de mémoire tiennent dans la borne haute ; en dessous de la borne basse, la
/// surveillance coûterait plus que le calcul. Ces bornes ne changent aucune valeur
/// simulée, seulement la vitesse.
const REGION_MIN: u64 = 8;
const REGION_MAX: u64 = 128;

impl Elaboration<'_> {
    /// Met à plat la définition `d`. `en_region` : un composant englobant est déjà
    /// signalé comme une unité.
    fn descendre(&mut self, d: usize, broches: &[u32], base: u32, en_region: bool) {
        let defs = self.defs;
        let def = &defs[d];
        if let Some(uf) = self.uf.as_mut() {
            for &(a, b) in &def.fusions {
                unir(uf, broches[a as usize], broches[b as usize]);
            }
        }
        let global = |local: u32| match def.attache[local as usize] {
            Attache::Broche(bit) => broches[bit as usize],
            Attache::Interne(k) => base + k,
        };
        let base_enfants = base + def.nb_internes;
        for enfant in &def.enfants {
            match enfant {
                Enfant::Porte {
                    genre,
                    e0,
                    e1,
                    s,
                    delai,
                } => self.netlist.portes.push(Porte {
                    genre: *genre,
                    e0: global(*e0),
                    e1: global(*e1),
                    s: global(*s),
                    delai: *delai,
                }),
                Enfant::Constante { s, valeur } => {
                    self.netlist.constantes.push((global(*s), *valeur))
                }
                Enfant::Horloge { s, demi_periode } => {
                    self.netlist.horloge(global(*s), *demi_periode);
                }
                Enfant::Sous {
                    def: c,
                    bits,
                    base: decalage,
                } => {
                    let broches_enfant: Vec<u32> = bits.iter().map(|&l| global(l)).collect();
                    // Les portes d'un sous-composant sont consécutives : le plus grand
                    // sous-composant de taille raisonnable forme une région.
                    let total = defs[*c].total_portes;
                    let region = !en_region && (REGION_MIN..=REGION_MAX).contains(&total);
                    let debut = self.netlist.portes.len() as u32;
                    self.descendre(
                        *c,
                        &broches_enfant,
                        base_enfants + *decalage as u32,
                        en_region || region,
                    );
                    if region {
                        self.netlist.region(debut..self.netlist.portes.len() as u32);
                    }
                }
            }
        }
    }
}

impl Circuit {
    /// Met à plat le composant `nom` de la bibliothèque.
    pub fn elaborer(bib: &Bibliotheque, nom: &str) -> Result<Circuit, Erreur> {
        let comp = compiler(bib, nom)?;
        let racine = &comp.defs[comp.racine];
        let nb_provisoires = racine.nb_bits as u64 + racine.total_nets;
        if nb_provisoires > u32::MAX as u64 || racine.total_portes >= (1 << 31) {
            return Err(Erreur::TropGrand);
        }
        let mut e = Elaboration {
            defs: &comp.defs,
            netlist: Netlist::nouvelle(),
            uf: racine
                .a_fusions
                .then(|| (0..nb_provisoires as u32).collect()),
        };
        e.netlist.portes.reserve_exact(racine.total_portes as usize);
        let broches_racine: Vec<u32> = (0..racine.nb_bits).collect();
        e.descendre(comp.racine, &broches_racine, racine.nb_bits, false);
        let Elaboration {
            mut netlist, uf, ..
        } = e;

        // Renumérotation dense si des nets ont été fusionnés.
        let canon = uf.map(|mut uf| {
            let mut canon = vec![AUCUN; uf.len()];
            let mut suivant = 0u32;
            for p in 0..uf.len() as u32 {
                let r = trouver(&mut uf, p);
                if canon[r as usize] == AUCUN {
                    canon[r as usize] = suivant;
                    suivant += 1;
                }
                canon[p as usize] = canon[r as usize];
            }
            canon
        });
        match &canon {
            None => netlist.nb_nets = nb_provisoires as u32,
            Some(c) => {
                netlist.nb_nets = c.iter().max().map_or(0, |m| m + 1);
                for p in &mut netlist.portes {
                    p.e0 = c[p.e0 as usize];
                    p.e1 = c[p.e1 as usize];
                    p.s = c[p.s as usize];
                }
                for (net, _) in &mut netlist.constantes {
                    *net = c[*net as usize];
                }
                for h in &mut netlist.horloges {
                    h.net = c[h.net as usize];
                }
            }
        }

        let definitif = |p: u32| canon.as_ref().map_or(p, |c| c[p as usize]);
        let mut broches = Vec::with_capacity(racine.broches.len());
        for b in &racine.broches {
            let nets: Vec<NetId> = (0..b.largeur).map(|k| definitif(b.debut + k)).collect();
            let premiere_entree = (b.sens == Sens::Entree).then(|| {
                let premiere = netlist.entrees.len();
                for &n in &nets {
                    netlist.entree(n);
                }
                premiere
            });
            broches.push(BrocheCircuit {
                nom: b.nom.clone(),
                sens: b.sens,
                nets,
                premiere_entree,
            });
        }
        netlist.valider()?;

        let avertissements = comp
            .defs
            .iter()
            .flat_map(|d| {
                d.entrees_flottantes
                    .iter()
                    .map(move |e| format!("dans « {} » : l'entrée {e} n'est reliée à rien", d.nom))
            })
            .collect();

        Ok(Circuit {
            nom: nom.to_string(),
            netlist,
            broches,
            avertissements,
            comp,
            canon,
        })
    }

    /// La broche de ce nom, si elle existe.
    pub fn broche(&self, nom: &str) -> Option<&BrocheCircuit> {
        self.broches.iter().find(|b| b.nom == nom)
    }

    /// Nets désignés par un chemin hiérarchique.
    ///
    /// Un chemin est une suite de noms d'instances séparés par des points, suivie
    /// d'une broche, avec une plage de bits facultative :
    /// `s`, `a[3]`, `add.r`, `cpu.ual.add7.x1.s`, `registre.q[0:3]`.
    /// La broche finale est celle du composant atteint ou, si le dernier nom
    /// d'instance désigne une primitive, celle de cette primitive.
    pub fn nets(&self, chemin: &str) -> Result<Vec<NetId>, Erreur> {
        let erreur = |raison: String| Erreur::Chemin {
            chemin: chemin.to_string(),
            raison,
        };
        let (corps, crochets) = match chemin.find('[') {
            Some(i) => (&chemin[..i], &chemin[i..]),
            None => (chemin, ""),
        };
        let segments: Vec<&str> = corps.split('.').collect();
        let defs = &self.comp.defs;
        let mut d = self.comp.racine;
        let mut broches: Vec<u32> = (0..defs[d].nb_bits).collect();
        let mut base = defs[d].nb_bits;
        let mut i = 0;
        // Descente dans les sous-composants tant qu'il reste plus d'un segment.
        while i + 1 < segments.len() {
            let def = &defs[d];
            let Some(&k) = def.index_instances.get(segments[i]) else {
                return Err(erreur(format!(
                    "« {} » n'a pas d'instance « {} »",
                    def.nom, segments[i]
                )));
            };
            let Enfant::Sous {
                def: c,
                bits,
                base: decalage,
            } = &def.enfants[k]
            else {
                break;
            };
            let global = |local: u32| match def.attache[local as usize] {
                Attache::Broche(bit) => broches[bit as usize],
                Attache::Interne(n) => base + n,
            };
            let suivantes: Vec<u32> = bits.iter().map(|&l| global(l)).collect();
            base = base + def.nb_internes + *decalage as u32;
            broches = suivantes;
            d = *c;
            i += 1;
        }
        let def = &defs[d];
        let global = |local: u32| match def.attache[local as usize] {
            Attache::Broche(bit) => broches[bit as usize],
            Attache::Interne(n) => base + n,
        };
        let reste = &segments[i..];
        let texte = format!("{}{}", reste.join("."), crochets);
        let point = analyser_point(&texte).map_err(&erreur)?;
        let (locaux, largeur): (Vec<u32>, u32) = match point.instance {
            None => {
                let Some(&k) = def.index_broches.get(point.broche) else {
                    return Err(erreur(format!(
                        "« {} » n'a pas de broche « {} »",
                        def.nom, point.broche
                    )));
                };
                let b = &def.broches[k];
                (
                    (0..b.largeur)
                        .map(|n| def.bit_local[(b.debut + n) as usize])
                        .collect(),
                    b.largeur,
                )
            }
            Some(instance) => {
                let Some(&k) = def.index_instances.get(instance) else {
                    return Err(erreur(format!(
                        "« {} » n'a pas d'instance « {} »",
                        def.nom, instance
                    )));
                };
                use GenrePorte::{Nand, Tri};
                let local = match (&def.enfants[k], point.broche) {
                    (
                        Enfant::Porte {
                            genre: Nand, e0, ..
                        },
                        "a",
                    ) => *e0,
                    (
                        Enfant::Porte {
                            genre: Nand, e1, ..
                        },
                        "b",
                    ) => *e1,
                    (Enfant::Porte { genre: Tri, e0, .. }, "e") => *e0,
                    (Enfant::Porte { genre: Tri, e1, .. }, "a") => *e1,
                    (Enfant::Porte { s, .. }, "s")
                    | (Enfant::Constante { s, .. }, "s")
                    | (Enfant::Horloge { s, .. }, "s") => *s,
                    _ => {
                        return Err(erreur(format!(
                            "l'instance « {} » n'a pas de broche « {} »",
                            def.noms_instances[k], point.broche
                        )))
                    }
                };
                (vec![local], 1)
            }
        };
        let choisis = bits_de_plage(&def.nom, chemin, point.plage, largeur)?;
        Ok(choisis
            .into_iter()
            .map(|n| {
                let p = global(locaux[n as usize]);
                self.canon.as_ref().map_or(p, |c| c[p as usize])
            })
            .collect())
    }

    /// Net unique désigné par un chemin (erreur si le chemin désigne un bus entier).
    pub fn net(&self, chemin: &str) -> Result<NetId, Erreur> {
        let nets = self.nets(chemin)?;
        if nets.len() == 1 {
            Ok(nets[0])
        } else {
            Err(Erreur::Chemin {
                chemin: chemin.to_string(),
                raison: format!("désigne {} bits, un seul attendu", nets.len()),
            })
        }
    }

    /// Nombre de portes primitives du circuit.
    pub fn nb_portes(&self) -> usize {
        self.netlist.portes.len()
    }
}
