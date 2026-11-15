//! Les briques : des circuits enregistrés, que l'on pose ensuite comme un seul élément.
//!
//! Un [`Modele`] est un plan qui porte un nom. Ses interrupteurs deviennent les broches
//! d'entrée de la brique, ses lampes les broches de sortie. Une brique peut contenir d'autres
//! briques, sans limite de profondeur, mais jamais elle-même.
//!
//! Le [`Projet`] réunit le plan principal et tous les modèles : c'est lui qu'on enregistre.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::document::{Broche, Document, Element, Fil, Genre, Id, Point, Source};
use crate::format::{self, Lecture};

/// Identifiant du format de fichier d'un projet.
pub const FORMAT: &str = "computer-engineer/projet/5";

/// Un circuit enregistré sous un nom, réutilisable comme brique.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Modele {
    /// Nom de la brique.
    pub nom: String,
    /// Ce qu'elle fait, en une phrase.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Son circuit.
    pub plan: Document,
    /// Pour une mémoire morte : le programme, en assembleur, dont son circuit est fabriqué
    /// ([`crate::programme`]). Depuis le format de projet 2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub programme: Option<String>,
}

impl Modele {
    /// Un élément prêt à poser : une brique de ce modèle, sans nom ni fil.
    pub fn instance(&self) -> Element {
        let (e, s) = self.plan.interface();
        Element::brique("", &self.nom, e, s)
    }
}

/// Tout ce que l'utilisateur a construit : le plan principal et ses briques.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Projet {
    /// Format du fichier.
    pub format: String,
    /// Nom du projet.
    pub nom: String,
    /// Clé de la quête en cours, s'il y en a une.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quete: Option<String>,
    /// Les briques.
    pub modeles: Vec<Modele>,
    /// Le plan principal.
    pub principal: Document,
    /// Les bancs d'essai écrits par l'utilisateur ([`crate::essai`]), par plan : `""` pour
    /// le plan principal, le nom de la brique sinon. Depuis le format de projet 4.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub bancs: BTreeMap<String, String>,
    /// Augmente à chaque changement d'un modèle : sert à savoir quand retraduire les briques.
    #[serde(skip)]
    pub revision: u64,
}

impl Projet {
    /// Projet sans brique, avec ce plan principal.
    pub fn nouveau(nom: &str, principal: Document) -> Self {
        Projet {
            format: FORMAT.to_owned(),
            nom: nom.to_owned(),
            quete: None,
            modeles: Vec::new(),
            principal,
            bancs: BTreeMap::new(),
            revision: 0,
        }
    }

    /// Rang du modèle de ce nom.
    pub fn index(&self, nom: &str) -> Option<usize> {
        self.modeles.iter().position(|m| m.nom == nom)
    }

    /// Le modèle de ce nom.
    pub fn modele(&self, nom: &str) -> Option<&Modele> {
        self.modeles.iter().find(|m| m.nom == nom)
    }

    /// Le plan d'un lieu : le plan principal (`None`) ou celui d'un modèle.
    pub fn plan(&self, lieu: Option<&str>) -> Option<&Document> {
        match lieu {
            None => Some(&self.principal),
            Some(nom) => self.modele(nom).map(|m| &m.plan),
        }
    }

    /// Remplace le plan d'un lieu. Si c'est un modèle, toutes ses briques posées ailleurs
    /// prennent ses nouvelles broches. Renvoie `false` si le modèle n'existe pas.
    pub fn deposer(&mut self, lieu: Option<&str>, plan: Document) -> bool {
        match lieu {
            None => {
                self.principal = plan;
                true
            }
            Some(nom) => {
                let Some(i) = self.index(nom) else {
                    return false;
                };
                if self.modeles[i].plan.signature() != plan.signature()
                    || self.modeles[i].plan.interface() != plan.interface()
                {
                    self.revision += 1;
                }
                self.modeles[i].plan = plan;
                self.actualiser(nom);
                true
            }
        }
    }

    /// Les modèles posés directement dans un plan.
    fn poses(plan: &Document) -> impl Iterator<Item = &str> {
        plan.elements
            .iter()
            .filter(|e| e.genre == Genre::Brique)
            .map(|e| e.modele.as_str())
    }

    /// Où la brique `nom` est posée directement : le nom de chaque brique qui la contient,
    /// et `None` pour le plan principal, avec le nombre d'exemplaires posés.
    pub fn usages(&self, nom: &str) -> Vec<(Option<&str>, usize)> {
        let compte = |plan: &Document| Self::poses(plan).filter(|m| *m == nom).count();
        let mut usages = Vec::new();
        let principal = compte(&self.principal);
        if principal > 0 {
            usages.push((None, principal));
        }
        for m in &self.modeles {
            let n = compte(&m.plan);
            if n > 0 {
                usages.push((Some(m.nom.as_str()), n));
            }
        }
        usages
    }

    /// Change la description de la brique `nom`. Faux si elle n'existe pas.
    pub fn decrire(&mut self, nom: &str, description: &str) -> bool {
        let Some(m) = self.modeles.iter_mut().find(|m| m.nom == nom) else {
            return false;
        };
        if m.description != description {
            m.description = description.to_owned();
            self.revision += 1;
        }
        true
    }

    /// Vrai si le modèle `a` est `b` ou contient `b`, directement ou non.
    pub fn utilise(&self, a: &str, b: &str) -> bool {
        let mut vus: Vec<&str> = Vec::new();
        let mut pile = vec![a];
        while let Some(nom) = pile.pop() {
            if nom == b {
                return true;
            }
            if vus.contains(&nom) {
                continue;
            }
            vus.push(nom);
            if let Some(m) = self.modele(nom) {
                pile.extend(Self::poses(&m.plan));
            }
        }
        false
    }

    /// Vrai si l'on peut poser une brique `modele` dans le plan du lieu sans qu'une brique
    /// finisse par se contenir elle-même.
    pub fn peut_poser(&self, lieu: Option<&str>, modele: &str) -> bool {
        self.modele(modele).is_some() && lieu.is_none_or(|l| !self.utilise(modele, l))
    }

    /// Nombre de fois où le modèle est posé, dans tous les plans.
    pub fn nb_utilisations(&self, nom: &str) -> usize {
        std::iter::once(&self.principal)
            .chain(self.modeles.iter().map(|m| &m.plan))
            .map(|p| Self::poses(p).filter(|m| *m == nom).count())
            .sum()
    }

    /// Nombre total de portes NAND d'un plan, briques ouvertes jusqu'au fond.
    pub fn nb_portes(&self, plan: &Document) -> u64 {
        let mut connus: BTreeMap<&str, u64> = BTreeMap::new();
        self.compter(plan, &mut connus, 0)
    }

    fn compter<'a>(
        &'a self,
        plan: &'a Document,
        connus: &mut BTreeMap<&'a str, u64>,
        profondeur: u32,
    ) -> u64 {
        let mut n = 0u64;
        for e in &plan.elements {
            match e.genre {
                Genre::Nand => n += 1,
                Genre::Brique => {
                    let nom = e.modele.as_str();
                    let k = match connus.get(nom) {
                        Some(k) => *k,
                        // Une brique qui se contiendrait elle-même ne se compte pas sans fin.
                        None if profondeur > 64 => 0,
                        None => {
                            let k = self
                                .modele(nom)
                                .map_or(0, |m| self.compter(&m.plan, connus, profondeur + 1));
                            connus.insert(nom, k);
                            k
                        }
                    };
                    n = n.saturating_add(k);
                }
                _ => {}
            }
        }
        n
    }

    /// Nombre de niveaux de briques emboîtées dans un plan (0 s'il n'en contient aucune).
    pub fn profondeur(&self, plan: &Document) -> u32 {
        fn descendre<'a>(
            p: &'a Projet,
            plan: &'a Document,
            connus: &mut BTreeMap<&'a str, u32>,
            niveau: u32,
        ) -> u32 {
            let mut max = 0;
            for nom in Projet::poses(plan) {
                let n = match connus.get(nom) {
                    Some(n) => *n,
                    // Une brique qui se contiendrait elle-même ne se parcourt pas sans fin.
                    None if niveau > 64 => 0,
                    None => {
                        let n = p
                            .modele(nom)
                            .map_or(0, |m| 1 + descendre(p, &m.plan, connus, niveau + 1));
                        connus.insert(nom, n);
                        n
                    }
                };
                max = max.max(n);
            }
            max
        }
        descendre(self, plan, &mut BTreeMap::new(), 0)
    }

    /// Un nom de modèle libre : `base`, sinon `base 2`, `base 3`…
    pub fn nom_libre(&self, base: &str) -> String {
        let base = base.trim();
        let base = if base.is_empty() { tr!("Brique") } else { base };
        if self.modele(base).is_none() {
            return base.to_owned();
        }
        (2..)
            .map(|k| format!("{base} {k}"))
            .find(|n| self.modele(n).is_none())
            .expect("il reste toujours un nom libre")
    }

    /// Ajoute un modèle. Si son nom est pris, il en reçoit un libre. Renvoie le nom retenu.
    pub fn ajouter(&mut self, mut modele: Modele) -> String {
        modele.nom = self.nom_libre(&modele.nom);
        let nom = modele.nom.clone();
        self.modeles.push(modele);
        self.revision += 1;
        nom
    }

    fn plans_mut(&mut self) -> impl Iterator<Item = &mut Document> {
        std::iter::once(&mut self.principal).chain(self.modeles.iter_mut().map(|m| &mut m.plan))
    }

    /// Renomme un modèle, partout où il est posé.
    pub fn renommer(&mut self, ancien: &str, nouveau: &str) -> Result<(), String> {
        let nouveau = nouveau.trim();
        if nouveau.is_empty() {
            return Err(tr!("Une brique a besoin d'un nom.").to_owned());
        }
        if nouveau == ancien {
            return Ok(());
        }
        if self.modele(nouveau).is_some() {
            return Err(tr!(
                "Une brique s'appelle déjà « {nouveau} ».",
                nouveau = nouveau
            ));
        }
        let Some(i) = self.index(ancien) else {
            return Err(tr!("La brique « {ancien} » n'existe pas.", ancien = ancien));
        };
        self.modeles[i].nom = nouveau.to_owned();
        for plan in self.plans_mut() {
            for e in &mut plan.elements {
                if e.genre == Genre::Brique && e.modele == ancien {
                    e.modele = nouveau.to_owned();
                }
            }
        }
        // Son banc d'essai la suit.
        if let Some(banc) = self.bancs.remove(ancien) {
            self.bancs.insert(nouveau.to_owned(), banc);
        }
        self.revision += 1;
        Ok(())
    }

    /// Retire un modèle qui n'est posé nulle part.
    pub fn retirer(&mut self, nom: &str) -> Result<(), String> {
        let n = self.nb_utilisations(nom);
        if n > 0 {
            return Err(tr!(
                "« {nom} » est encore posée {n} fois : retire d'abord ces briques.",
                nom = nom,
                n = n
            ));
        }
        let Some(i) = self.index(nom) else {
            return Err(tr!("La brique « {nom} » n'existe pas.", nom = nom));
        };
        self.modeles.remove(i);
        self.bancs.remove(nom);
        self.revision += 1;
        Ok(())
    }

    /// Donne à toutes les briques posées du modèle `nom` ses broches actuelles. Les fils
    /// branchés sur une broche qui existe toujours, à la même largeur, sont conservés.
    pub fn actualiser(&mut self, nom: &str) {
        let Some(m) = self.modele(nom) else {
            return;
        };
        let (e, s) = m.plan.interface();
        let mut change = false;
        for plan in self.plans_mut() {
            change |= actualiser_instances(plan, nom, &e, &s);
        }
        if change {
            self.revision += 1;
        }
    }

    /// Le projet en JSON lisible.
    pub fn en_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("un projet se sérialise toujours")
    }

    /// Lit un projet et répare ce qui peut l'être (ordre des éléments, fils orphelins).
    pub fn de_json(texte: &str) -> Result<Projet, String> {
        Projet::de_json_lu(texte).map(|(p, _)| p)
    }

    /// Comme [`Projet::de_json`], et dit si le fichier a dû être migré depuis un format plus
    /// ancien ([`crate::format`](mod@crate::format)).
    pub fn de_json_lu(texte: &str) -> Result<(Projet, Lecture), String> {
        let (valeur, lecture) = format::PROJET.lire(texte)?;
        let mut p: Projet =
            serde_json::from_value(valeur).map_err(|e| tr!("fichier illisible : {e}", e = e))?;
        for plan in p.plans_mut() {
            plan.reparer();
        }
        let noms: Vec<String> = p.modeles.iter().map(|m| m.nom.clone()).collect();
        for nom in &noms {
            p.actualiser(nom);
        }
        p.revision = 1;
        Ok((p, lecture))
    }
}

/// Donne aux briques du modèle `nom` posées dans `plan` les broches `e` et `s`.
/// Renvoie `true` si quelque chose a changé.
pub fn actualiser_instances(plan: &mut Document, nom: &str, e: &[Broche], s: &[Broche]) -> bool {
    let mut change = false;
    // Pour chaque brique touchée : la nouvelle place de chacune de ses anciennes sorties.
    let mut sorties: Vec<(Id, Vec<Option<usize>>)> = Vec::new();
    for el in &mut plan.elements {
        if el.genre != Genre::Brique || el.modele != nom {
            continue;
        }
        if el.broches_e == e && el.broches_s == s {
            continue;
        }
        change = true;
        let entrees = e
            .iter()
            .map(|b| {
                el.broches_e
                    .iter()
                    .position(|a| a.origine == b.origine && a.largeur == b.largeur)
                    .and_then(|i| el.entrees.get(i).copied().flatten())
            })
            .collect();
        let places = el
            .broches_s
            .iter()
            .map(|a| {
                s.iter()
                    .position(|b| b.origine == a.origine && b.largeur == a.largeur)
            })
            .collect();
        sorties.push((el.id, places));
        el.entrees = entrees;
        el.broches_e = e.to_vec();
        el.broches_s = s.to_vec();
    }
    if !change {
        return false;
    }
    for el in &mut plan.elements {
        for src in &mut el.entrees {
            let Some(source) = *src else { continue };
            if let Some((_, places)) = sorties.iter().find(|(id, _)| *id == source.id) {
                *src = places
                    .get(source.sortie)
                    .copied()
                    .flatten()
                    .map(|j| Source::nouvelle(source.id, j));
            }
        }
    }
    plan.debrancher_les_fils_faux();
    true
}

/// Garde d'un nom ce qui peut servir de nom de broche.
fn nettoyer(nom: &str) -> String {
    nom.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

/// Ce que l'on peut mettre dans une brique en regroupant une sélection : les interrupteurs,
/// les lampes, les écrans et les claviers restent dehors, ce sont eux qui la font vivre.
pub fn groupable(genre: Genre) -> bool {
    matches!(
        genre,
        Genre::Nand | Genre::Brique | Genre::Horloge | Genre::Decoupe | Genre::Jonction
    )
}

/// Regroupe des éléments d'un plan en une brique.
///
/// Les éléments sont retirés du plan et remplacés par une brique posée au même endroit. Ce
/// qui leur arrivait de l'extérieur devient une broche d'entrée, ce qui en partait vers
/// l'extérieur une broche de sortie ; une entrée ou une sortie laissée libre devient aussi une
/// broche. Le comportement du plan ne change pas.
///
/// Renvoie le modèle à ajouter au projet et l'identifiant de la brique posée, ou `None` si
/// rien dans `ids` ne peut être regroupé.
pub fn regrouper(plan: &mut Document, ids: &[Id], nom: &str) -> Option<(Modele, Id)> {
    let mut groupe: Vec<Id> = ids
        .iter()
        .copied()
        .filter(|id| plan.element(*id).is_some_and(|e| groupable(e.genre)))
        .collect();
    groupe.sort_unstable();
    groupe.dedup();
    if groupe.is_empty() {
        return None;
    }
    let dedans = |id: Id| groupe.binary_search(&id).is_ok();
    let membres: Vec<Element> = groupe
        .iter()
        .filter_map(|id| plan.element(*id).cloned())
        .collect();
    let gauche = membres
        .iter()
        .map(|e| e.pos.x)
        .fold(f32::INFINITY, f32::min);
    let haut = membres
        .iter()
        .map(|e| e.pos.y)
        .fold(f32::INFINITY, f32::min);
    let droite = membres
        .iter()
        .map(|e| e.pos.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let n = membres.len() as f32;
    let centre = Point::nouveau(
        (membres.iter().map(|e| e.pos.x).sum::<f32>() / n / 5.0).round() * 5.0,
        (membres.iter().map(|e| e.pos.y).sum::<f32>() / n / 5.0).round() * 5.0,
    );
    // Dans la brique, les éléments gardent leur disposition, décalée pour laisser la place
    // des broches à gauche.
    let place = |p: Point| Point::nouveau(p.x - gauche + 260.0, p.y - haut + 120.0);

    // 1. Les éléments, sans leurs fils.
    let mut interieur = Document::nouveau();
    let mut copie: BTreeMap<Id, Id> = BTreeMap::new();
    for e in &membres {
        let mut c = e.clone();
        c.entrees = vec![None; c.broches_e.len()];
        copie.insert(e.id, interieur.poser(c, place(e.pos)));
    }

    // 2. Les fils internes ; ce qui vient de l'extérieur ou reste libre devient une broche.
    struct Entrante {
        source: Option<Source>,
        largeur: u32,
        nom: String,
        y: f32,
        arrivees: Vec<Fil>,
    }
    let mut entrantes: Vec<Entrante> = Vec::new();
    for e in &membres {
        for (i, src) in e.entrees.iter().enumerate() {
            let arrivee = Fil {
                puits: copie[&e.id],
                entree: i,
            };
            let y = place(e.entree_en(e.pos, i)).y;
            match src {
                Some(s) if dedans(s.id) => {
                    interieur.relier(Source::nouvelle(copie[&s.id], s.sortie), arrivee);
                }
                Some(s) => match entrantes.iter_mut().find(|x| x.source == Some(*s)) {
                    Some(x) => x.arrivees.push(arrivee),
                    None => entrantes.push(Entrante {
                        source: Some(*s),
                        largeur: plan.largeur_source(*s).unwrap_or(1),
                        nom: nettoyer(&plan.nom_signal(*s)),
                        y,
                        arrivees: vec![arrivee],
                    }),
                },
                None => entrantes.push(Entrante {
                    source: None,
                    largeur: e.broches_e[i].largeur,
                    nom: nettoyer(&format!("{}_{}", e.nom, e.broches_e[i].nom)),
                    y,
                    arrivees: vec![arrivee],
                }),
            }
        }
    }
    entrantes.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal));

    // 3. Les sorties lues dehors, ou par personne, deviennent des broches.
    struct Sortante {
        source: Source,
        largeur: u32,
        nom: String,
        y: f32,
        dehors: Vec<Fil>,
    }
    let fils = plan.fils();
    let mut sortantes: Vec<Sortante> = Vec::new();
    for e in &membres {
        for (j, b) in e.broches_s.iter().enumerate() {
            let source = Source::nouvelle(e.id, j);
            let lecteurs: Vec<Fil> = fils
                .iter()
                .filter(|(_, s)| *s == source)
                .map(|(f, _)| *f)
                .collect();
            let dehors: Vec<Fil> = lecteurs
                .iter()
                .copied()
                .filter(|f| !dedans(f.puits))
                .collect();
            if !dehors.is_empty() || lecteurs.is_empty() {
                sortantes.push(Sortante {
                    source,
                    largeur: b.largeur,
                    nom: nettoyer(&plan.nom_signal(source)),
                    y: place(e.sortie_en(e.pos, j)).y,
                    dehors,
                });
            }
        }
    }
    sortantes.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal));

    // 4. Les broches dans le plan de la brique, espacées de haut en bas.
    let etager = |ys: Vec<f32>| -> Vec<f32> {
        let mut places: Vec<f32> = Vec::new();
        for y in ys {
            let y = (y / 5.0).round() * 5.0;
            let mini = places.last().map_or(f32::NEG_INFINITY, |p| p + 70.0);
            places.push(y.max(mini));
        }
        places
    };
    let nom_de_broche = |d: &Document, voulu: &str, defaut: &str| -> String {
        if !voulu.is_empty() && d.nomme(voulu).is_none() {
            voulu.to_owned()
        } else if voulu.is_empty() {
            d.nom_libre(defaut)
        } else {
            d.nom_libre(&format!("{voulu}_"))
        }
    };
    let mut entrees_dehors: Vec<(Id, Option<Source>)> = Vec::new();
    let ys = etager(entrantes.iter().map(|x| x.y).collect());
    for (x, y) in entrantes.iter().zip(ys) {
        let nom = nom_de_broche(&interieur, &x.nom, "e");
        let id = interieur.poser(Element::entree(&nom, x.largeur), Point::nouveau(80.0, y));
        for a in &x.arrivees {
            interieur.relier(Source::de(id), *a);
        }
        entrees_dehors.push((id, x.source));
    }
    let mut sorties_dehors: Vec<(Id, &Sortante)> = Vec::new();
    let ys = etager(sortantes.iter().map(|x| x.y).collect());
    for (x, y) in sortantes.iter().zip(ys) {
        let nom = nom_de_broche(&interieur, &x.nom, "s");
        let id = interieur.poser(
            Element::sortie(&nom, "", x.largeur),
            Point::nouveau(droite - gauche + 260.0 + 200.0, y),
        );
        interieur.relier(
            Source::nouvelle(copie[&x.source.id], x.source.sortie),
            Fil {
                puits: id,
                entree: 0,
            },
        );
        sorties_dehors.push((id, x));
    }

    // 5. La brique posée à la place de la sélection, branchée comme l'était la sélection.
    let modele = Modele {
        nom: nom.to_owned(),
        description: String::new(),
        plan: interieur,
        programme: None,
    };
    let instance = modele.instance();
    let (broches_e, broches_s) = (instance.broches_e.clone(), instance.broches_s.clone());
    let brique = plan.poser(instance, centre);
    for (i, b) in broches_e.iter().enumerate() {
        if let Some((_, Some(source))) = entrees_dehors.iter().find(|(id, _)| *id == b.origine) {
            plan.relier(
                *source,
                Fil {
                    puits: brique,
                    entree: i,
                },
            );
        }
    }
    for (j, b) in broches_s.iter().enumerate() {
        if let Some((_, x)) = sorties_dehors.iter().find(|(id, _)| *id == b.origine) {
            for f in &x.dehors {
                plan.relier(Source::nouvelle(brique, j), *f);
            }
        }
    }
    for id in &groupe {
        plan.retirer(*id);
    }
    Some((modele, brique))
}

/// Fait d'un plan entier une brique : ses interrupteurs et ses lampes deviennent ses broches.
pub fn modele_du_plan(plan: &Document, nom: &str) -> Modele {
    Modele {
        nom: nom.to_owned(),
        description: String::new(),
        plan: plan.clone(),
        programme: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(d: &Document, nom: &str) -> Id {
        d.nomme(nom).unwrap().id
    }

    #[test]
    fn regrouper_tout_le_demi_additionneur_donne_une_brique_a_deux_entrees_et_deux_sorties() {
        let mut plan = Document::demi_additionneur();
        let tout: Vec<Id> = plan.elements.iter().map(|e| e.id).collect();
        let (modele, brique) = regrouper(&mut plan, &tout, "DEMI_ADD").unwrap();
        // Dehors : les deux interrupteurs, la brique, les deux lampes.
        assert_eq!(plan.elements.len(), 5);
        let b = plan.element(brique).unwrap();
        let noms = |l: &[Broche]| l.iter().map(|b| b.nom.clone()).collect::<Vec<_>>();
        assert_eq!(noms(&b.broches_e), ["a", "b"]);
        assert_eq!(noms(&b.broches_s), ["s", "c"]);
        assert_eq!(b.entrees[0], Some(Source::de(id(&plan, "a"))));
        assert_eq!(b.entrees[1], Some(Source::de(id(&plan, "b"))));
        let lampe = |nom: &str| {
            plan.source_de(Fil {
                puits: id(&plan, nom),
                entree: 0,
            })
        };
        assert_eq!(lampe("s"), Some(Source::nouvelle(brique, 0)));
        assert_eq!(lampe("c"), Some(Source::nouvelle(brique, 1)));
        // Dedans : cinq portes, deux broches d'entrée, deux de sortie, douze fils.
        assert_eq!(modele.plan.portes().count(), 5);
        assert_eq!(modele.plan.elements.len(), 9);
        assert_eq!(modele.plan.fils().len(), 12);
    }

    #[test]
    fn une_entree_libre_et_une_sortie_libre_deviennent_des_broches() {
        let mut plan = Document::nouveau();
        let g1 = plan.ajouter_porte(Point::nouveau(100.0, 100.0));
        let g2 = plan.ajouter_porte(Point::nouveau(300.0, 100.0));
        for entree in 0..2 {
            plan.relier(Source::de(g1), Fil { puits: g2, entree });
        }
        let (modele, brique) = regrouper(&mut plan, &[g1, g2], "ET").unwrap();
        let b = plan.element(brique).unwrap();
        assert_eq!(b.broches_e.len(), 2, "les deux entrées de g1");
        assert_eq!(
            b.broches_s.len(),
            1,
            "la sortie de g2 ; celle de g1 est lue dedans"
        );
        assert_eq!(b.modele, "ET");
        assert_eq!(modele.plan.interface().0.len(), 2);
    }

    #[test]
    fn une_brique_ne_peut_pas_se_contenir() {
        let mut p = Projet::nouveau("essai", Document::nouveau());
        let mut a = Document::nouveau();
        a.ajouter_porte(Point::default());
        p.ajouter(modele_du_plan(&a, "A"));
        let mut b = Document::nouveau();
        b.poser(p.modele("A").unwrap().instance(), Point::default());
        p.ajouter(modele_du_plan(&b, "B"));
        assert!(p.utilise("B", "A"));
        assert!(!p.utilise("A", "B"));
        assert!(p.peut_poser(None, "B"));
        assert!(p.peut_poser(Some("B"), "A"));
        assert!(!p.peut_poser(Some("A"), "B"), "B contient A");
        assert!(!p.peut_poser(Some("A"), "A"));
        assert_eq!(p.nb_portes(&p.modele("B").unwrap().plan), 1);
        assert_eq!(p.profondeur(&p.modele("B").unwrap().plan), 1);
        assert_eq!(p.nb_utilisations("A"), 1);
        assert!(p.retirer("A").is_err());
        assert!(p.renommer("A", "B").is_err());
        p.renommer("A", "Porte").unwrap();
        assert_eq!(p.modele("B").unwrap().plan.elements[0].modele, "Porte");
        assert_eq!(p.ajouter(modele_du_plan(&a, "Porte")), "Porte 2");
    }

    #[test]
    fn changer_les_broches_d_un_modele_garde_les_fils_qui_tiennent_encore() {
        let mut plan = Document::demi_additionneur();
        let tout: Vec<Id> = plan.elements.iter().map(|e| e.id).collect();
        let (modele, brique) = regrouper(&mut plan, &tout, "DEMI_ADD").unwrap();
        let mut p = Projet::nouveau("essai", plan);
        p.ajouter(modele);
        // Dans le modèle : on retire la lampe c et on ajoute un interrupteur tout en haut.
        let mut dedans = p.modele("DEMI_ADD").unwrap().plan.clone();
        let c = id(&dedans, "c");
        dedans.retirer(c);
        dedans.poser(Element::entree("z", 1), Point::nouveau(80.0, -500.0));
        assert!(p.deposer(Some("DEMI_ADD"), dedans));
        let b = p.principal.element(brique).unwrap();
        let noms = |l: &[Broche]| l.iter().map(|b| b.nom.clone()).collect::<Vec<_>>();
        assert_eq!(noms(&b.broches_e), ["z", "a", "b"]);
        assert_eq!(noms(&b.broches_s), ["s"]);
        assert_eq!(b.entrees[0], None);
        assert_eq!(b.entrees[1], Some(Source::de(id(&p.principal, "a"))));
        let lampe = |nom: &str| {
            p.principal.source_de(Fil {
                puits: id(&p.principal, nom),
                entree: 0,
            })
        };
        assert_eq!(lampe("s"), Some(Source::nouvelle(brique, 0)));
        assert_eq!(lampe("c"), None, "la broche c n'existe plus");
    }

    #[test]
    fn le_projet_fait_l_aller_retour_en_json() {
        let mut plan = Document::demi_additionneur();
        let tout: Vec<Id> = plan.elements.iter().map(|e| e.id).collect();
        let (modele, _) = regrouper(&mut plan, &tout, "DEMI_ADD").unwrap();
        let mut p = Projet::nouveau("essai", plan);
        p.ajouter(modele);
        let relu = Projet::de_json(&p.en_json()).unwrap();
        assert_eq!(relu.principal, p.principal);
        assert_eq!(relu.modeles, p.modeles);
        assert!(Projet::de_json("{}").is_err());
    }
}
