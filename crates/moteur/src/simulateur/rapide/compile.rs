//! Le mode rapide compilé : la période d'horloge traduite en code machine.
//!
//! Le mode rapide (voir le module parent) calcule chaque période en trois temps :
//! la logique s'établit, les bascules prennent leur entrée au front montant, la
//! logique s'établit à nouveau. Ici, ces trois temps sont écrits une fois pour toutes
//! en WebAssembly, puis traduits en code machine par Cranelift (à travers Wasmtime,
//! sans aucun code `unsafe` de notre côté). Le code obtenu fait *exactement* les
//! mêmes calculs que le mode rapide événementiel, dans un autre ordre :
//!
//! - la logique est découpée en blocs de portes consécutives dans un ordre
//!   topologique ; un bloc n'est évalué que si l'un des fils qu'il lit hors de
//!   lui-même a changé (son drapeau est levé), et alors toutes ses portes le sont ;
//! - au front montant, chaque bascule prend la valeur de son entrée `d` ;
//! - à la fin de la période, chaque bascule dont le couple (entrée, valeur mémorisée)
//!   a changé reçoit son état interne établi, lu dans la table de sa caractérisation.
//!
//! Pourquoi c'est le même résultat : la logique n'a pas de boucle, donc ses valeurs
//! établies sont l'unique point fixe de ses équations pour des sources données ;
//! toute évaluation qui passe chaque porte après celles dont elle dépend, et repasse
//! toute porte dont une entrée a changé, y aboutit (lemme C du théorème 6 de
//! `docs/semantique.md`).

use super::{Plan, Q, QN, TROIS_ETATS};
use crate::simulateur::TABLE;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::{Arc, OnceLock};
use wasm_encoder::{
    CodeSection, ConstExpr, DataSection, ExportKind, ExportSection, Function, FunctionSection,
    Instruction as I, MemArg, MemorySection, MemoryType, Module as ModuleWasm, TypeSection,
    ValType,
};
use wasmtime::{Engine, Instance, Memory, Module, Store, TypedFunc};

/// Tables de la mémoire du code compilé, à des adresses fixes.
const TABLE_ADR: u64 = 0;
const CAPTURE_ADR: u64 = 32;
const INVERSE_ADR: u64 = 36;
const RANG_ADR: u64 = 40;
/// Nombre d'évaluations de portes, sur 64 bits.
const COMPTEUR_ADR: u64 = 48;
/// Valeur de chaque net.
pub(super) const VAL_ADR: usize = 64;

/// Nombre de portes par bloc.
const TAILLE_BLOC: usize = 16;
/// Nombre de blocs par groupe de blocs : un groupe dont aucun bloc n'a son drapeau
/// levé est sauté d'un coup.
const BLOCS_PAR_GROUPE: usize = 16;
/// Nombre de bascules par groupe.
const TAILLE_GROUPE: usize = 8;
/// Nombre approximatif d'instructions WebAssembly par fonction : Cranelift traite
/// les fonctions une à une, et une fonction trop longue lui coûte plus cher.
const INSTRUCTIONS_PAR_FONCTION: usize = 20_000;

/// Le code compilé d'un plan, partagé par toutes les copies du simulateur.
pub(super) struct Compile {
    module: Module,
    /// Bloc de chaque porte de `Plan::logique`.
    pub(super) bloc_de: Vec<u32>,
    pub(super) nb_blocs: usize,
    /// Adresses des drapeaux des blocs, des valeurs mémorisées des bascules et de la
    /// clé (niveau, `d`, valeur) de leur dernier état interne écrit.
    pub(super) drapeaux: usize,
    pub(super) v: usize,
    pub(super) prec: usize,
    /// Drapeaux des groupes de bascules : `nb_groupes` candidats, puis autant de
    /// « pris », puis autant de « touchés ».
    pub(super) candidats: usize,
    pub(super) nb_groupes: usize,
    /// Drapeaux des groupes de blocs (juste après ceux des blocs).
    supers: usize,
}

impl Compile {
    /// Lève le drapeau du bloc `b` (et celui de son groupe de blocs).
    pub(super) fn lever(&self, m: &mut [u8], b: usize) {
        m[self.drapeaux + b] = 1;
        m[self.supers + b / BLOCS_PAR_GROUPE] = 1;
    }

    /// Lève tous les drapeaux des blocs.
    pub(super) fn lever_tout(&self, m: &mut [u8]) {
        m[self.drapeaux..self.v].fill(1);
    }

    /// Le groupe de la bascule `b`.
    pub(super) fn groupe(b: usize) -> usize {
        b / TAILLE_GROUPE
    }
}

impl std::fmt::Debug for Compile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Compile")
            .field("nb_blocs", &self.nb_blocs)
            .finish_non_exhaustive()
    }
}

fn moteur() -> Option<&'static Engine> {
    static MOTEUR: OnceLock<Option<Engine>> = OnceLock::new();
    MOTEUR
        .get_or_init(|| Engine::new(&wasmtime::Config::new()).ok())
        .as_ref()
}

#[inline]
fn adr(a: u64) -> MemArg {
    MemArg {
        offset: a,
        align: 0,
        memory_index: 0,
    }
}

/// Un corps de fonction en cours d'écriture, avec son nombre d'instructions.
struct Corps {
    f: Function,
    n: usize,
}

impl Corps {
    fn new(locaux: u32) -> Corps {
        let locaux: Vec<(u32, ValType)> = if locaux > 0 {
            vec![(locaux, ValType::I32)]
        } else {
            Vec::new()
        };
        Corps {
            f: Function::new(locaux),
            n: 0,
        }
    }

    #[inline]
    fn i(&mut self, ins: I<'_>) {
        self.f.instruction(&ins);
        self.n += 1;
    }

    /// Empile l'octet à l'adresse fixe `a`.
    fn lire(&mut self, a: u64) {
        self.i(I::I32Const(0));
        self.i(I::I32Load8U(adr(a)));
    }

    /// Écrit la constante `v` à l'adresse fixe `a`.
    fn ecrire(&mut self, a: u64, v: i32) {
        self.i(I::I32Const(0));
        self.i(I::I32Const(v));
        self.i(I::I32Store8(adr(a)));
    }
}

/// Des fonctions sans résultat écrites à la suite, découpées pour rester courtes ;
/// une fonction « parente » les appelle dans l'ordre.
struct Suite {
    /// Type des fonctions (0 : sans paramètre ; 1 : un paramètre i32 transmis).
    type_: u32,
    locaux: u32,
    finies: Vec<Function>,
    courante: Corps,
}

impl Suite {
    fn new(type_: u32, locaux: u32) -> Suite {
        Suite {
            type_,
            locaux,
            finies: Vec::new(),
            courante: Corps::new(locaux),
        }
    }

    /// Ferme la fonction courante si elle est assez longue.
    fn couper(&mut self) {
        if self.courante.n >= INSTRUCTIONS_PAR_FONCTION {
            self.fermer();
        }
    }

    fn fermer(&mut self) {
        let mut c = std::mem::replace(&mut self.courante, Corps::new(self.locaux));
        c.i(I::End);
        self.finies.push(c.f);
    }
}

/// Les fonctions du module, numérotées dans l'ordre où elles sont ajoutées.
struct Fonctions {
    types: Vec<u32>,
    corps: Vec<Function>,
}

impl Fonctions {
    fn ajouter(&mut self, type_: u32, f: Function) -> u32 {
        self.types.push(type_);
        self.corps.push(f);
        self.corps.len() as u32 - 1
    }

    /// Ajoute les fonctions de la suite et leur parente ; rend le numéro de la parente.
    fn ajouter_suite(&mut self, mut s: Suite) -> u32 {
        s.fermer();
        let mut parente = Corps::new(0);
        for f in std::mem::take(&mut s.finies) {
            let k = self.ajouter(s.type_, f);
            if s.type_ == 1 {
                parente.i(I::LocalGet(0));
            }
            parente.i(I::Call(k));
        }
        parente.i(I::End);
        self.ajouter(s.type_, parente.f)
    }
}

/// Adresses des drapeaux des blocs et de ceux des groupes de blocs.
#[derive(Clone, Copy)]
struct Drapeaux {
    blocs: u64,
    supers: u64,
}

/// Lève les drapeaux des blocs `blocs` et de leurs groupes de blocs.
fn lever(c: &mut Corps, dr: Drapeaux, blocs: &[u32]) {
    let mut dernier = u64::MAX;
    for &b in blocs {
        c.ecrire(dr.blocs + b as u64, 1);
        let s = b as u64 / BLOCS_PAR_GROUPE as u64;
        if s != dernier {
            c.ecrire(dr.supers + s, 1);
            dernier = s;
        }
    }
}

/// Un ordre topologique de la logique qui garde ensemble les portes voisines dans la
/// netlist (donc d'un même composant) : à chaque pas, la porte prête de plus petit
/// numéro.
fn ordre_local(plan: &Plan) -> Vec<u32> {
    let n = plan.logique.len();
    let mut pilote = vec![u32::MAX; plan.nb_nets];
    for (l, p) in plan.logique.iter().enumerate() {
        pilote[p.s as usize] = l as u32;
    }
    let mut degre = vec![0u32; n];
    let mut suivantes: Vec<Vec<u32>> = vec![Vec::new(); n];
    for (l, p) in plan.logique.iter().enumerate() {
        for e in [p.e0, p.e1] {
            let k = pilote[e as usize];
            if k != u32::MAX {
                degre[l] += 1;
                suivantes[k as usize].push(l as u32);
            }
        }
    }
    let cle = |l: u32| plan.logique[l as usize].porte & !TROIS_ETATS;
    let mut pretes: BinaryHeap<Reverse<(u32, u32)>> = (0..n as u32)
        .filter(|&l| degre[l as usize] == 0)
        .map(|l| Reverse((cle(l), l)))
        .collect();
    let mut ordre = Vec::with_capacity(n);
    while let Some(Reverse((_, l))) = pretes.pop() {
        ordre.push(l);
        for &k in &suivantes[l as usize] {
            degre[k as usize] -= 1;
            if degre[k as usize] == 0 {
                pretes.push(Reverse((cle(k), k)));
            }
        }
    }
    debug_assert_eq!(ordre.len(), n, "la logique du plan est sans boucle");
    ordre
}

/// Écrit et compile le module du plan ; `None` si Wasmtime n'est pas disponible
/// sur cette machine.
pub(super) fn compiler(plan: &Plan) -> Option<Arc<Compile>> {
    let moteur = moteur()?;
    let (octets, c) = ecrire_module(plan);
    let module = Module::new(moteur, octets).ok()?;
    Some(Arc::new(Compile {
        module,
        bloc_de: c.bloc_de,
        nb_blocs: c.nb_blocs,
        drapeaux: c.drapeaux,
        v: c.v,
        prec: c.prec,
        candidats: c.candidats,
        nb_groupes: c.nb_groupes,
        supers: c.supers,
    }))
}

/// Le module WebAssembly du plan, et sa disposition en mémoire (le champ `module`
/// est à remplacer par le module compilé).
fn ecrire_module(plan: &Plan) -> (Vec<u8>, CompileSansModule) {
    let nb_nets = plan.nb_nets;
    let ordre = ordre_local(plan);
    let nb_blocs = ordre.len().div_ceil(TAILLE_BLOC);
    let mut bloc_de = vec![0u32; plan.logique.len()];
    for (i, &l) in ordre.iter().enumerate() {
        bloc_de[l as usize] = (i / TAILLE_BLOC) as u32;
    }
    // Disposition de la mémoire.
    let nb_bascules = plan.bascules.len();
    let drapeaux = VAL_ADR + nb_nets;
    let nb_supers = nb_blocs.div_ceil(BLOCS_PAR_GROUPE);
    let supers = drapeaux + nb_blocs;
    let v = supers + nb_supers;
    let prec = v + nb_bascules;
    let tmp = prec + nb_bascules;
    // Les bascules vont par groupes : un groupe est « candidat » quand l'entrée `d`
    // d'une de ses bascules a changé depuis le dernier front montant, « pris » entre
    // les deux temps d'un front montant, « touché » quand l'entrée ou la valeur d'une
    // de ses bascules a changé depuis la dernière pose des états internes.
    let nb_groupes = nb_bascules.div_ceil(TAILLE_GROUPE);
    let candidats = tmp + nb_bascules;
    let pris = candidats + nb_groupes;
    let touches = pris + nb_groupes;
    let mut canons: Vec<&super::Caracterisation> = Vec::new();
    let mut canon_de = Vec::with_capacity(nb_bascules);
    for b in &plan.bascules {
        let k = canons
            .iter()
            .position(|&c| std::ptr::eq(c, &*b.carac))
            .unwrap_or_else(|| {
                canons.push(&b.carac);
                canons.len() - 1
            });
        canon_de.push(k);
    }
    let canon = touches + nb_groupes;
    let taille = canon + 162 * canons.len();
    let pages = (taille as u64).div_ceil(65536).max(1);

    // Blocs lecteurs de chaque net.
    let mut lecteurs: Vec<Vec<u32>> = vec![Vec::new(); nb_nets];
    for (l, p) in plan.logique.iter().enumerate() {
        for e in [p.e0, p.e1] {
            let b = bloc_de[l];
            let lb = &mut lecteurs[e as usize];
            if !lb.contains(&b) {
                lb.push(b);
            }
        }
    }
    // Groupes dont une bascule a pour entrée `d` chaque net.
    let mut groupes_d: Vec<Vec<u32>> = vec![Vec::new(); nb_nets];
    for (b, bascule) in plan.bascules.iter().enumerate() {
        let g = (b / TAILLE_GROUPE) as u32;
        let gd = &mut groupes_d[bascule.d as usize];
        if !gd.contains(&g) {
            gd.push(g);
        }
    }
    let dr = Drapeaux {
        blocs: drapeaux as u64,
        supers: supers as u64,
    };
    let val = VAL_ADR as u64;
    // Le net a changé : ce qui le lit est à recalculer (blocs de `blocs`, bascules).
    let signaler = |c: &mut Corps, net: u32, blocs: &[u32]| {
        lever(c, dr, blocs);
        for &g in &groupes_d[net as usize] {
            c.ecrire((candidats as u32 + g) as u64, 1);
            c.ecrire((touches as u32 + g) as u64, 1);
        }
    };

    // La logique, bloc par bloc.
    let mut logique = Suite::new(0, TAILLE_BLOC as u32);
    let mut local_de = vec![u32::MAX; nb_nets];
    for (k, bloc) in ordre.chunks(TAILLE_BLOC).enumerate() {
        let c = &mut logique.courante;
        let s = (k / BLOCS_PAR_GROUPE) as u64;
        if k % BLOCS_PAR_GROUPE == 0 {
            // Un groupe de blocs : sauté si aucun n'a son drapeau levé. Un bloc ne
            // lève que des drapeaux de blocs qui le suivent : en baissant celui du
            // groupe d'abord, rien n'est perdu.
            c.lire(dr.supers + s);
            c.i(I::If(wasm_encoder::BlockType::Empty));
            c.ecrire(dr.supers + s, 0);
        }
        c.lire(dr.blocs + k as u64);
        c.i(I::If(wasm_encoder::BlockType::Empty));
        c.ecrire(dr.blocs + k as u64, 0);
        // compteur += taille du bloc
        c.i(I::I32Const(0));
        c.i(I::I32Const(0));
        c.i(I::I64Load(adr(COMPTEUR_ADR)));
        c.i(I::I64Const(bloc.len() as i64));
        c.i(I::I64Add);
        c.i(I::I64Store(adr(COMPTEUR_ADR)));
        for (j, &l) in bloc.iter().enumerate() {
            let p = plan.logique[l as usize];
            for (i, e) in [p.e0, p.e1].into_iter().enumerate() {
                if local_de[e as usize] != u32::MAX {
                    c.i(I::LocalGet(local_de[e as usize]));
                } else {
                    c.lire(val + e as u64);
                }
                if i == 0 {
                    c.i(I::I32Const(2));
                    c.i(I::I32Shl);
                }
            }
            c.i(I::I32Or);
            let table = TABLE_ADR + if p.porte & TROIS_ETATS != 0 { 16 } else { 0 };
            c.i(I::I32Load8U(adr(table)));
            c.i(I::LocalSet(j as u32));
            local_de[p.s as usize] = j as u32;
            let dehors: Vec<u32> = lecteurs[p.s as usize]
                .iter()
                .copied()
                .filter(|&b| b != k as u32)
                .collect();
            if !dehors.is_empty() || !groupes_d[p.s as usize].is_empty() {
                // Le fil est lu hors du bloc : s'il change, ses lecteurs sont à évaluer.
                c.lire(val + p.s as u64);
                c.i(I::LocalGet(j as u32));
                c.i(I::I32Ne);
                c.i(I::If(wasm_encoder::BlockType::Empty));
                signaler(c, p.s, &dehors);
                c.i(I::End);
            }
            c.i(I::I32Const(0));
            c.i(I::LocalGet(j as u32));
            c.i(I::I32Store8(adr(val + p.s as u64)));
        }
        c.i(I::End);
        for &l in bloc {
            local_de[plan.logique[l as usize].s as usize] = u32::MAX;
        }
        if (k + 1) % BLOCS_PAR_GROUPE == 0 || k + 1 == nb_blocs {
            c.i(I::End);
            logique.couper();
        }
    }

    // Front montant, en deux temps : les bascules des groupes candidats prennent `d`,
    // puis celles qui changent écrivent `q` et `q_n`. Une bascule hors de ces groupes
    // garderait sa valeur : son entrée n'a pas changé depuis qu'elle l'a prise.
    let groupes: Vec<(usize, &[super::Bascule])> = plan
        .bascules
        .chunks(TAILLE_GROUPE)
        .enumerate()
        .map(|(g, bs)| (g * TAILLE_GROUPE, bs))
        .collect();
    let mut prise = Suite::new(0, 0);
    for (g, &(premiere, bs)) in groupes.iter().enumerate() {
        let c = &mut prise.courante;
        c.lire((candidats + g) as u64);
        c.i(I::If(wasm_encoder::BlockType::Empty));
        c.ecrire((candidats + g) as u64, 0);
        c.ecrire((pris + g) as u64, 1);
        for (i, bascule) in bs.iter().enumerate() {
            c.i(I::I32Const(0));
            c.lire(val + bascule.d as u64);
            c.i(I::I32Load8U(adr(CAPTURE_ADR)));
            c.i(I::I32Store8(adr((tmp + premiere + i) as u64)));
        }
        c.i(I::End);
        prise.couper();
    }
    let mut ecriture = Suite::new(0, 1);
    for (g, &(premiere, bs)) in groupes.iter().enumerate() {
        let c = &mut ecriture.courante;
        c.lire((pris + g) as u64);
        c.i(I::If(wasm_encoder::BlockType::Empty));
        c.ecrire((pris + g) as u64, 0);
        for (i, bascule) in bs.iter().enumerate() {
            let b = premiere + i;
            c.lire((tmp + b) as u64);
            c.i(I::LocalTee(0));
            c.lire((v + b) as u64);
            c.i(I::I32Ne);
            c.i(I::If(wasm_encoder::BlockType::Empty));
            for (a, inverse) in [
                (v + b, false),
                (VAL_ADR + bascule.sorties[Q] as usize, false),
                (VAL_ADR + bascule.sorties[QN] as usize, true),
            ] {
                c.i(I::I32Const(0));
                c.i(I::LocalGet(0));
                if inverse {
                    c.i(I::I32Load8U(adr(INVERSE_ADR)));
                }
                c.i(I::I32Store8(adr(a as u64)));
            }
            c.ecrire((touches + g) as u64, 1);
            for s in [bascule.sorties[Q], bascule.sorties[QN]] {
                signaler(c, s, &lecteurs[s as usize]);
            }
            c.i(I::End);
        }
        c.i(I::End);
        ecriture.couper();
    }
    // L'horloge.
    let horloge = |niveau: i32| {
        let mut c = Corps::new(0);
        c.ecrire(val + plan.horloge as u64, niveau);
        lever(&mut c, dr, &lecteurs[plan.horloge as usize]);
        c.i(I::End);
        c.f
    };

    // État interne établi des bascules dont la clé a changé, dans les groupes touchés.
    let mut etat = Suite::new(1, 1);
    for (g, &(premiere, bs)) in groupes.iter().enumerate() {
        let c = &mut etat.courante;
        c.lire((touches + g) as u64);
        c.i(I::If(wasm_encoder::BlockType::Empty));
        c.ecrire((touches + g) as u64, 0);
        for (i, bascule) in bs.iter().enumerate() {
            let b = premiere + i;
            // Paramètre 0 : le niveau ; local 1 : la clé.
            // cle = 9 × niveau + 3 × rang(d) + v
            c.i(I::LocalGet(0));
            c.i(I::I32Const(9));
            c.i(I::I32Mul);
            c.lire(val + bascule.d as u64);
            c.i(I::I32Load8U(adr(RANG_ADR)));
            c.i(I::I32Const(3));
            c.i(I::I32Mul);
            c.i(I::I32Add);
            c.lire((v + b) as u64);
            c.i(I::I32Add);
            c.i(I::LocalTee(1));
            c.lire((prec + b) as u64);
            c.i(I::I32Ne);
            c.i(I::If(wasm_encoder::BlockType::Empty));
            c.i(I::I32Const(0));
            c.i(I::LocalGet(1));
            c.i(I::I32Store8(adr((prec + b) as u64)));
            let base = (canon + 162 * canon_de[b]) as u64;
            for (k, &sortie) in bascule.sorties.iter().enumerate() {
                c.i(I::I32Const(0));
                c.i(I::LocalGet(1));
                c.i(I::I32Const(9));
                c.i(I::I32Mul);
                c.i(I::I32Load8U(adr(base + k as u64)));
                c.i(I::I32Store8(adr(val + sortie as u64)));
            }
            c.i(I::End);
        }
        c.i(I::End);
        etat.couper();
    }

    let mut fonctions = Fonctions {
        types: Vec::new(),
        corps: Vec::new(),
    };
    let propager = fonctions.ajouter_suite(logique);
    let prendre = fonctions.ajouter_suite(prise);
    let ecrire = fonctions.ajouter_suite(ecriture);
    let monter_h = fonctions.ajouter(0, horloge(1));
    let descendre = fonctions.ajouter(0, horloge(0));
    let etablir = fonctions.ajouter_suite(etat);
    // periodes(n, montant_d_abord, niveau)
    let mut p = Corps::new(0);
    p.i(I::Call(propager));
    p.i(I::Block(wasm_encoder::BlockType::Empty));
    p.i(I::Loop(wasm_encoder::BlockType::Empty));
    p.i(I::LocalGet(0));
    p.i(I::I32Eqz);
    p.i(I::BrIf(1));
    let monter = [prendre, ecrire, monter_h, propager];
    let tomber = [descendre, propager];
    p.i(I::LocalGet(1));
    p.i(I::If(wasm_encoder::BlockType::Empty));
    for f in monter.iter().chain(&tomber) {
        p.i(I::Call(*f));
    }
    p.i(I::Else);
    for f in tomber.iter().chain(&monter) {
        p.i(I::Call(*f));
    }
    p.i(I::End);
    p.i(I::LocalGet(2));
    p.i(I::Call(etablir));
    p.i(I::LocalGet(0));
    p.i(I::I32Const(1));
    p.i(I::I32Sub);
    p.i(I::LocalSet(0));
    p.i(I::Br(0));
    p.i(I::End);
    p.i(I::End);
    p.i(I::End);
    let periodes = fonctions.ajouter(2, p.f);

    // Le module.
    let mut module = ModuleWasm::new();
    let mut types = TypeSection::new();
    types.ty().function([], []);
    types.ty().function([ValType::I32], []);
    types
        .ty()
        .function([ValType::I32, ValType::I32, ValType::I32], []);
    module.section(&types);
    let mut fs = FunctionSection::new();
    for &t in &fonctions.types {
        fs.function(t);
    }
    module.section(&fs);
    let mut memoires = MemorySection::new();
    memoires.memory(MemoryType {
        minimum: pages,
        maximum: Some(pages),
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    module.section(&memoires);
    let mut exports = ExportSection::new();
    exports.export("periodes", ExportKind::Func, periodes);
    exports.export("memoire", ExportKind::Memory, 0);
    module.section(&exports);
    let mut code = CodeSection::new();
    for f in &fonctions.corps {
        code.function(f);
    }
    module.section(&code);
    let mut donnees = DataSection::new();
    let mut constantes = Vec::with_capacity(48);
    constantes.extend_from_slice(&TABLE);
    // capture : 0, 1, X, X ; inverse : 1, 0, X, X ; rang : 0, 1, 2, 2.
    constantes.extend_from_slice(&[0, 1, 2, 2, 1, 0, 2, 2, 0, 1, 2, 2]);
    debug_assert_eq!(constantes.len() as u64, RANG_ADR + 4);
    donnees.active(0, &ConstExpr::i32_const(0), constantes);
    let mut tables = Vec::with_capacity(162 * canons.len());
    for carac in &canons {
        for niveau in &carac.canon {
            for d in niveau {
                for etat in d {
                    tables.extend_from_slice(etat);
                }
            }
        }
    }
    donnees.active(0, &ConstExpr::i32_const(canon as i32), tables);
    module.section(&donnees);

    (
        module.finish(),
        CompileSansModule {
            bloc_de,
            nb_blocs,
            drapeaux,
            v,
            prec,
            candidats,
            nb_groupes,
            supers,
        },
    )
}

/// `Compile` avant la compilation du module.
struct CompileSansModule {
    bloc_de: Vec<u32>,
    nb_blocs: usize,
    drapeaux: usize,
    v: usize,
    prec: usize,
    candidats: usize,
    nb_groupes: usize,
    supers: usize,
}

/// Le code compilé, instancié pour un simulateur.
pub(super) struct Execution {
    pub(super) compile: Arc<Compile>,
    store: Store<()>,
    pub(super) memoire: Memory,
    periodes: TypedFunc<(i32, i32, i32), ()>,
    /// La mémoire reflète l'état du simulateur (sinon, il faut l'y recopier).
    pub(super) synchro: bool,
}

impl Execution {
    pub(super) fn nouvelle(compile: Arc<Compile>) -> Option<Execution> {
        let mut store = Store::new(moteur()?, ());
        let instance = Instance::new(&mut store, &compile.module, &[]).ok()?;
        let memoire = instance.get_memory(&mut store, "memoire")?;
        let periodes = instance
            .get_typed_func::<(i32, i32, i32), ()>(&mut store, "periodes")
            .ok()?;
        Some(Execution {
            compile,
            store,
            memoire,
            periodes,
            synchro: false,
        })
    }

    pub(super) fn memoire(&mut self) -> &mut [u8] {
        self.memoire.data_mut(&mut self.store)
    }

    /// Calcule `n` périodes ; rend le nombre de portes évaluées.
    pub(super) fn periodes(&mut self, mut n: u64, montant_d_abord: bool, niveau: u32) -> u64 {
        while n > 0 {
            let k = n.min(i32::MAX as u64);
            self.periodes
                .call(
                    &mut self.store,
                    (k as i32, i32::from(montant_d_abord), niveau as i32),
                )
                .expect("le code compilé du mode rapide ne s'interrompt pas");
            n -= k;
        }
        let m = self.memoire();
        let a = COMPTEUR_ADR as usize;
        let mut c = [0u8; 8];
        c.copy_from_slice(&m[a..a + 8]);
        m[a..a + 8].fill(0);
        u64::from_le_bytes(c)
    }
}

/// Une exécution propre à chaque simulateur : une copie du simulateur repart sans
/// (elle en recrée une au besoin, depuis son propre état).
#[derive(Default)]
pub(super) struct Emplacement(pub(super) Option<Execution>);

impl Clone for Emplacement {
    fn clone(&self) -> Self {
        Emplacement(None)
    }
}

impl std::fmt::Debug for Emplacement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.0.is_some() {
            "Execution"
        } else {
            "Aucune"
        })
    }
}
