//! Grands circuits de mesure, tous en NAND et tampons trois états.
//!
//! Ils servent à mesurer le moteur sur des tailles bien supérieures à celle de
//! l'ordinateur miniature, et à vérifier que ses accélérations ne changent aucune valeur :
//!
//! - [`memoire`] : une mémoire vive de `2^n` octets à bus trois états, avec ses décodeurs
//!   de lignes et de colonnes (475 000 portes pour 4 Ko) ;
//! - [`multiplicateur`] : un multiplicateur combinatoire `n × n` bits (11 000 portes pour
//!   32 bits), où chaque changement d'entrée fait onduler des milliers de portes ;
//! - [`ferme`] : plusieurs ordinateurs miniatures côte à côte, chacun avec son horloge.
//!
//! Chaque constructeur ajoute ses composants à une bibliothèque et rend le nom du composant
//! principal.

use crate::erreur::Erreur;
use crate::modele::{Bibliotheque, Composant};
use crate::ordinateur::{self, Reglages};
use crate::reference;

/// `DECODEUR_n` : `s[v] = 1` quand les `n` bits de `e` valent `v`. Renvoie son nom.
///
/// Un décodeur de `n` bits est fait de deux décodeurs moitié moins larges et d'une porte ET
/// par sortie : le nombre de portes qui bougent à un changement d'adresse reste petit.
pub fn decodeur(bib: &mut Bibliotheque, bits: u32) -> String {
    assert!((1..=16).contains(&bits), "décodeur de 1 à 16 bits");
    let nom = format!("DECODEUR_{bits}");
    if bib.composants.contains_key(&nom) {
        return nom;
    }
    let mut c = Composant::nouveau(&format!(
        "Décodeur {bits} vers {} : s[v] = 1 quand e vaut v",
        1u32 << bits
    ))
    .entree("e", bits)
    .sortie("s", 1 << bits);
    if bits == 1 {
        c = c
            .inst("non", "NON")
            .inst("oui", "NON")
            .fil(&["e", "non.a"])
            .fil(&["non.s", "oui.a", "s[0]"])
            .fil(&["oui.s", "s[1]"]);
    } else {
        let (bas, haut) = (bits / 2, bits - bits / 2);
        let (nom_bas, nom_haut) = (decodeur(bib, bas), decodeur(bib, haut));
        c = c
            .inst("bas", &nom_bas)
            .inst("haut", &nom_haut)
            .fil(&[format!("e[0:{}]", bas - 1), "bas.e".to_string()])
            .fil(&[format!("e[{bas}:{}]", bits - 1), "haut.e".to_string()]);
        let mut lecteurs_bas = vec![Vec::new(); 1 << bas];
        let mut lecteurs_haut = vec![Vec::new(); 1 << haut];
        for v in 0..(1u32 << bits) {
            c = c
                .inst(&format!("et{v}"), "ET")
                .fil(&[format!("et{v}.s"), format!("s[{v}]")]);
            lecteurs_bas[(v & ((1 << bas) - 1)) as usize].push(format!("et{v}.a"));
            lecteurs_haut[(v >> bas) as usize].push(format!("et{v}.b"));
        }
        for (cote, lecteurs) in [("bas", lecteurs_bas), ("haut", lecteurs_haut)] {
            for (k, mut points) in lecteurs.into_iter().enumerate() {
                points.insert(0, format!("{cote}.s[{k}]"));
                c = c.fil(&points);
            }
        }
    }
    bib.ajouter(&nom, c);
    nom
}

/// `MOT8` : un octet de mémoire. Au front montant de `clk`, il prend `d` si `sel` et
/// `ecrire` valent 1 ; il pose son contenu sur le bus `q` quand `sel = 1` et se tait sinon.
fn mot8(bib: &mut Bibliotheque) -> String {
    let nom = "MOT8".to_string();
    if bib.composants.contains_key(&nom) {
        return nom;
    }
    let mut c = Composant::nouveau(
        "Un octet de mémoire : écrit au front montant si sel et ecrire valent 1, lu sur le bus q quand sel = 1",
    )
    .entree("d", 8)
    .entree("sel", 1)
    .entree("ecrire", 1)
    .entree("clk", 1)
    .sortie("q", 8)
    .inst("charge", "ET")
    .inst("reg", "REGISTRE8")
    .fil(&["ecrire", "charge.b"])
    .fil(&["charge.s", "reg.charge"])
    .fil(&["d", "reg.d"])
    .fil(&["clk", "reg.clk"]);
    let mut sels = vec!["sel".to_string(), "charge.a".to_string()];
    for i in 0..8 {
        c = c
            .inst(&format!("t{i}"), "TRI")
            .fil(&[format!("reg.q[{i}]"), format!("t{i}.a")])
            .fil(&[format!("t{i}.s"), format!("q[{i}]")]);
        sels.push(format!("t{i}.e"));
    }
    bib.ajouter(&nom, c.fil(&sels));
    nom
}

/// `MEMOIRE_n` : mémoire vive de `2^n` octets (`n` de 2 à 16). Renvoie son nom.
///
/// Broches : `adresse[n]`, `d[8]`, `ecrire`, `clk` → `q[8]`. Le mot désigné par l'adresse
/// pose son contenu sur le bus `q` ; au front montant de `clk`, il prend `d` si `ecrire = 1`.
/// Les mots sont rangés en lignes et en colonnes : un décodeur pour chaque moitié de
/// l'adresse, et une porte ET par mot.
pub fn memoire(bib: &mut Bibliotheque, bits: u32) -> String {
    assert!((2..=16).contains(&bits), "mémoire de 4 à 65 536 octets");
    let nom = format!("MEMOIRE_{bits}");
    if bib.composants.contains_key(&nom) {
        return nom;
    }
    let (bits_col, bits_ligne) = (bits / 2, bits - bits / 2);
    let (nb_col, nb_lignes) = (1u32 << bits_col, 1u32 << bits_ligne);
    let mot = mot8(bib);

    // Une rangée : `nb_col` mots qui partagent la même ligne de sélection.
    let nom_rangee = format!("RANGEE_{bits_col}");
    if !bib.composants.contains_key(&nom_rangee) {
        let mut r = Composant::nouveau(&format!("Une rangée de {nb_col} octets de mémoire"))
            .entree("ligne", 1)
            .entree("col", nb_col)
            .entree("d", 8)
            .entree("ecrire", 1)
            .entree("clk", 1)
            .sortie("q", 8);
        let mut lignes = vec!["ligne".to_string()];
        let mut d = vec!["d".to_string()];
        let mut ecrire = vec!["ecrire".to_string()];
        let mut clk = vec!["clk".to_string()];
        let mut q = vec!["q".to_string()];
        for j in 0..nb_col {
            r = r
                .inst(&format!("sel{j}"), "ET")
                .inst(&format!("m{j}"), &mot)
                .fil(&[format!("col[{j}]"), format!("sel{j}.b")])
                .fil(&[format!("sel{j}.s"), format!("m{j}.sel")]);
            lignes.push(format!("sel{j}.a"));
            d.push(format!("m{j}.d"));
            ecrire.push(format!("m{j}.ecrire"));
            clk.push(format!("m{j}.clk"));
            q.push(format!("m{j}.q"));
        }
        bib.ajouter(
            &nom_rangee,
            r.fil(&lignes).fil(&d).fil(&ecrire).fil(&clk).fil(&q),
        );
    }

    let (dec_col, dec_ligne) = (decodeur(bib, bits_col), decodeur(bib, bits_ligne));
    let mut c = Composant::nouveau(&format!(
        "Mémoire vive de {} octets à bus trois états : q = contenu de la case adresse ; au front montant de clk, cette case prend d si ecrire = 1",
        1u32 << bits
    ))
    .entree("adresse", bits)
    .entree("d", 8)
    .entree("ecrire", 1)
    .entree("clk", 1)
    .sortie("q", 8)
    .inst("colonnes", &dec_col)
    .inst("lignes", &dec_ligne)
    .fil(&[format!("adresse[0:{}]", bits_col - 1), "colonnes.e".to_string()])
    .fil(&[
        format!("adresse[{bits_col}:{}]", bits - 1),
        "lignes.e".to_string(),
    ]);
    let mut col = vec!["colonnes.s".to_string()];
    let mut d = vec!["d".to_string()];
    let mut ecrire = vec!["ecrire".to_string()];
    let mut clk = vec!["clk".to_string()];
    let mut q = vec!["q".to_string()];
    for i in 0..nb_lignes {
        c = c
            .inst(&format!("r{i}"), &nom_rangee)
            .fil(&[format!("lignes.s[{i}]"), format!("r{i}.ligne")]);
        col.push(format!("r{i}.col"));
        d.push(format!("r{i}.d"));
        ecrire.push(format!("r{i}.ecrire"));
        clk.push(format!("r{i}.clk"));
        q.push(format!("r{i}.q"));
    }
    bib.ajouter(&nom, c.fil(&col).fil(&d).fil(&ecrire).fil(&clk).fil(&q));
    nom
}

/// `MEMOIRE_HORLOGE_n` : [`memoire`] et son horloge de demi-période `demi_periode`.
/// Broches : `adresse[n]`, `d[8]`, `ecrire` → `q[8]`.
pub fn memoire_cadencee(bib: &mut Bibliotheque, bits: u32, demi_periode: u64) -> String {
    let nom = format!("MEMOIRE_HORLOGE_{bits}");
    let mem = memoire(bib, bits);
    bib.ajouter(
        &nom,
        Composant::nouveau("Une mémoire vive et son horloge")
            .entree("adresse", bits)
            .entree("d", 8)
            .entree("ecrire", 1)
            .sortie("q", 8)
            .horloge("horloge", demi_periode)
            .inst("mem", &mem)
            .fil(&["horloge.s", "mem.clk"])
            .fil(&["adresse", "mem.adresse"])
            .fil(&["d", "mem.d"])
            .fil(&["ecrire", "mem.ecrire"])
            .fil(&["mem.q", "q"]),
    );
    nom
}

/// `MULT_n` : multiplicateur combinatoire, `p = a × b` sur `2n` bits (`n` de 2 à 32).
/// Renvoie son nom.
///
/// C'est le multiplicateur « en tableau » : une porte ET par produit partiel, et une rangée
/// d'additionneurs complets par bit de `b`.
pub fn multiplicateur(bib: &mut Bibliotheque, bits: u32) -> String {
    assert!((2..=32).contains(&bits), "multiplicateur de 2 à 32 bits");
    let nom = format!("MULT_{bits}");
    if bib.composants.contains_key(&nom) {
        return nom;
    }
    let n = bits;
    let mut c = Composant::nouveau(&format!("Multiplicateur {n} × {n} bits : p = a × b"))
        .entree("a", n)
        .entree("b", n)
        .sortie("p", 2 * n)
        .inst("zero", "ZERO");
    let mut lecteurs_a = vec![Vec::new(); n as usize];
    let mut lecteurs_b = vec![Vec::new(); n as usize];
    let mut zeros = vec!["zero.s".to_string()];
    for j in 0..n {
        for i in 0..n {
            c = c.inst(&format!("pp{i}_{j}"), "ET");
            lecteurs_a[i as usize].push(format!("pp{i}_{j}.a"));
            lecteurs_b[j as usize].push(format!("pp{i}_{j}.b"));
        }
    }
    // `somme[i]` : le bit i du cumul des rangées déjà additionnées, décalé de la rangée.
    let mut somme: Vec<String> = (0..n).map(|i| format!("pp{i}_0.s")).collect();
    let mut retenue = "zero.s".to_string();
    c = c.fil(&[somme[0].clone(), "p[0]".to_string()]);
    for j in 1..n {
        let mut suivante = Vec::with_capacity(n as usize);
        for i in 0..n {
            let add = format!("add{i}_{j}");
            c = c.inst(&add, "ADD_COMPLET");
            // Le cumul décalé d'un bit : son bit i est l'ancien bit i + 1, et son bit de
            // poids fort est l'ancienne retenue.
            let gauche = if i + 1 < n {
                somme[(i + 1) as usize].clone()
            } else {
                retenue.clone()
            };
            if gauche == "zero.s" {
                zeros.push(format!("{add}.a"));
            } else {
                c = c.fil(&[gauche, format!("{add}.a")]);
            }
            c = c.fil(&[format!("pp{i}_{j}.s"), format!("{add}.b")]);
            if i == 0 {
                zeros.push(format!("{add}.re"));
            } else {
                c = c.fil(&[format!("add{}_{j}.rs", i - 1), format!("{add}.re")]);
            }
            suivante.push(format!("{add}.s"));
        }
        retenue = format!("add{}_{j}.rs", n - 1);
        somme = suivante;
        c = c.fil(&[somme[0].clone(), format!("p[{j}]")]);
    }
    for i in 1..n {
        c = c.fil(&[somme[i as usize].clone(), format!("p[{}]", n - 1 + i)]);
    }
    c = c.fil(&[retenue, format!("p[{}]", 2 * n - 1)]);
    for (i, mut points) in lecteurs_a.into_iter().enumerate() {
        points.insert(0, format!("a[{i}]"));
        c = c.fil(&points);
    }
    for (j, mut points) in lecteurs_b.into_iter().enumerate() {
        points.insert(0, format!("b[{j}]"));
        c = c.fil(&points);
    }
    bib.ajouter(&nom, c.fil(&zeros));
    nom
}

/// `FERME_k` : `k` ordinateurs miniatures qui exécutent le même programme, chacun avec son
/// horloge. Broches : `raz`, `clavier[8]` → `pc[8k]`, `arret[k]`. Rend la bibliothèque et le
/// nom du composant.
pub fn ferme(
    programme: &[u16],
    reglages: Reglages,
    nombre: u32,
) -> Result<(Bibliotheque, String), Erreur> {
    let mut bib = ordinateur::bibliotheque(programme, reglages)?;
    let nom = format!("FERME_{nombre}");
    let mut c = Composant::nouveau(&format!("{nombre} ordinateurs miniatures côte à côte"))
        .entree("raz", 1)
        .entree("clavier", 8)
        .sortie("pc", 8 * nombre)
        .sortie("arret", nombre);
    let mut raz = vec!["raz".to_string()];
    let mut clavier = vec!["clavier".to_string()];
    for k in 0..nombre {
        c = c
            .inst(&format!("o{k}"), "ORDINATEUR")
            .fil(&[format!("o{k}.pc"), format!("pc[{}:{}]", 8 * k, 8 * k + 7)])
            .fil(&[format!("o{k}.arret"), format!("arret[{k}]")]);
        raz.push(format!("o{k}.raz"));
        clavier.push(format!("o{k}.clavier"));
    }
    bib.ajouter(&nom, c.fil(&raz).fil(&clavier));
    Ok((bib, nom))
}

/// La bibliothèque de référence, point de départ des grands circuits.
pub fn bibliotheque() -> Bibliotheque {
    reference::bibliotheque()
}

/// Une [`memoire`] en marche, pilotée cycle par cycle et comparée à un modèle.
pub struct MemoireEnMarche {
    /// Le banc : le circuit mis à plat et son simulateur.
    pub banc: crate::banc::Banc,
    /// Demi-période de l'horloge.
    pub demi_periode: u64,
    /// Ce que la mémoire doit contenir : `None` pour une case jamais écrite.
    pub modele: Vec<Option<u8>>,
}

impl MemoireEnMarche {
    /// Fabrique une mémoire de `2^bits` octets et se place au milieu d'un niveau bas de
    /// l'horloge, là où les commandes changent sans risque.
    pub fn nouvelle(bits: u32, demi_periode: u64) -> Result<MemoireEnMarche, Erreur> {
        MemoireEnMarche::avec_options(bits, demi_periode, ce_moteur::Options::default())
    }

    /// Comme [`MemoireEnMarche::nouvelle`], en choisissant les réglages du simulateur.
    pub fn avec_options(
        bits: u32,
        demi_periode: u64,
        options: ce_moteur::Options,
    ) -> Result<MemoireEnMarche, Erreur> {
        let mut bib = bibliotheque();
        let nom = memoire_cadencee(&mut bib, bits, demi_periode);
        let mut banc = crate::banc::Banc::nouveau(&bib, &nom)?;
        banc.regler(options)?;
        MemoireEnMarche::de_banc(banc, bits, demi_periode)
    }

    /// Comme [`MemoireEnMarche::nouvelle`], à partir d'un banc déjà préparé (pour choisir les
    /// options du simulateur).
    pub fn de_banc(
        mut banc: crate::banc::Banc,
        bits: u32,
        demi_periode: u64,
    ) -> Result<MemoireEnMarche, Erreur> {
        banc.ecrire("adresse", 0)?;
        banc.ecrire("d", 0)?;
        banc.ecrire("ecrire", 0)?;
        banc.avancer(2 * demi_periode + demi_periode / 2);
        Ok(MemoireEnMarche {
            banc,
            demi_periode,
            modele: vec![None; 1 << bits],
        })
    }

    /// Un cycle d'horloge : présente `adresse`, `d` et `ecrire`, laisse passer un front
    /// montant, et rend ce que la mémoire pose sur son bus (`None` si un bit n'est pas
    /// déterminé). Le modèle est mis à jour.
    pub fn cycle(&mut self, adresse: usize, d: u8, ecrire: bool) -> Option<u8> {
        let poser = |banc: &mut crate::banc::Banc, broche: &str, v: u64| {
            banc.ecrire(broche, v)
                .expect("la mémoire a toujours cette broche")
        };
        poser(&mut self.banc, "adresse", adresse as u64);
        poser(&mut self.banc, "d", u64::from(d));
        poser(&mut self.banc, "ecrire", u64::from(ecrire));
        self.banc.avancer(2 * self.demi_periode);
        if ecrire {
            self.modele[adresse] = Some(d);
        }
        self.banc
            .lire("q")
            .expect("la mémoire a toujours ce signal")
            .map(|v| v as u8)
    }
}
