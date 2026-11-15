//! La fenêtre de la documentation (F2) : le sommaire à gauche, la page à droite, avec retour
//! en arrière, recherche et liens d'une page à l'autre. Le contenu vient de
//! [`crate::documentation`] ; ici, on ne fait que le montrer.

use std::collections::BTreeSet;

use egui::{
    Align, Color32, Layout, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, TextEdit, Vec2,
};

use crate::apparence::{Apparence, Palette};
use crate::dessin::Police;
use crate::documentation::{self as doc, Comportement, Famille, Ligne, Page, Signal, Technique};
use crate::etat::Atelier;
use crate::panneaux::Ouverture;

/// L'état de la fenêtre de documentation.
#[derive(Clone, Debug)]
pub struct Documentation {
    /// La page montrée.
    pub page: Page,
    /// Les pages d'avant, pour revenir en arrière.
    arriere: Vec<Page>,
    /// Les pages quittées par « Retour », pour y revenir.
    avant: Vec<Page>,
    /// Ce qu'on cherche dans le sommaire.
    pub recherche: String,
    /// Les briques dont on a demandé à voir la construction avant d'avoir réussi leur quête.
    devoilees: BTreeSet<String>,
    /// Vrai quand la page vient de changer : on remonte en haut.
    remonter: bool,
}

impl Documentation {
    /// La documentation ouverte sur cette page.
    pub fn sur(page: Page) -> Documentation {
        Documentation {
            page,
            arriere: Vec::new(),
            avant: Vec::new(),
            recherche: String::new(),
            devoilees: BTreeSet::new(),
            remonter: true,
        }
    }

    /// Va à une autre page ; « Retour » ramènera ici.
    pub fn aller(&mut self, page: Page) {
        if page != self.page {
            let ancienne = std::mem::replace(&mut self.page, page);
            self.arriere.push(ancienne);
            self.avant.clear();
            self.remonter = true;
        }
    }

    /// Revient à la page d'avant.
    pub fn retour(&mut self) {
        if let Some(p) = self.arriere.pop() {
            let ancienne = std::mem::replace(&mut self.page, p);
            self.avant.push(ancienne);
            self.remonter = true;
        }
    }

    /// Repart vers la page quittée par « Retour ».
    pub fn suivant(&mut self) {
        if let Some(p) = self.avant.pop() {
            let ancienne = std::mem::replace(&mut self.page, p);
            self.arriere.push(ancienne);
            self.remonter = true;
        }
    }

    /// Dessine la fenêtre. Rend `false` quand on la ferme, et une quête ou un exemple à
    /// ouvrir si on l'a demandé.
    pub fn montrer(
        &mut self,
        ctx: &egui::Context,
        at: &Atelier,
        a: &Apparence,
        cadre: egui::Frame,
    ) -> (bool, Option<Ouverture>) {
        let mut ouverte = true;
        let mut ouvrir = None;
        let c = a.palette();
        egui::Window::new(tr!("Documentation"))
            .id(egui::Id::unique("fenetre-documentation"))
            .open(&mut ouverte)
            .collapsible(false)
            .resizable(true)
            .default_size({
                // Grande, pour lire à l'aise, sans sortir de l'écran.
                let ecran = ctx.content_rect().size();
                [
                    (ecran.x * 0.8).clamp(560.0, 1120.0),
                    (ecran.y * 0.86).clamp(360.0, 860.0),
                ]
            })
            .min_size([560.0, 360.0])
            .frame(cadre)
            .show(ctx, |ui| {
                // Alt+← et Alt+→, comme dans un navigateur.
                let (gauche, droite) = ui.input(|i| {
                    (
                        i.modifiers.alt && i.key_pressed(egui::Key::ArrowLeft),
                        i.modifiers.alt && i.key_pressed(egui::Key::ArrowRight),
                    )
                });
                if gauche {
                    self.retour();
                }
                if droite {
                    self.suivant();
                }
                let hauteur = ui.available_height().max(300.0);
                let etroite = ui.available_width() < 760.0;
                let largeur_sommaire = if etroite { 190.0 } else { 236.0 };
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(
                        Vec2::new(largeur_sommaire, hauteur),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.set_min_size(Vec2::new(largeur_sommaire, hauteur));
                            self.sommaire(ui, a, c);
                        },
                    );
                    ui.separator();
                    let reste = ui.available_width();
                    ui.allocate_ui_with_layout(
                        Vec2::new(reste, hauteur),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.set_min_size(Vec2::new(reste, hauteur));
                            self.barre(ui, c);
                            ui.separator();
                            let mut defile = ScrollArea::vertical()
                                .id_salt("doc-page")
                                .auto_shrink(false);
                            if std::mem::take(&mut self.remonter) {
                                defile = defile.vertical_scroll_offset(0.0);
                            }
                            defile.show(ui, |ui| {
                                ui.set_max_width(reste - 16.0);
                                let page = self.page.clone();
                                match &page {
                                    Page::Sommaire => self.accueil(ui, a, c),
                                    Page::Notion(cle) => {
                                        self.notion(ui, cle, at, a, c, &mut ouvrir)
                                    }
                                    Page::Brique(nom) => {
                                        self.brique(ui, nom, at, a, c, &mut ouvrir)
                                    }
                                }
                                ui.add_space(24.0);
                            });
                        },
                    );
                });
            });
        (ouverte, ouvrir)
    }

    /// La colonne de gauche : la recherche, puis le sommaire (ou ce qu'on a trouvé).
    fn sommaire(&mut self, ui: &mut egui::Ui, a: &Apparence, c: &Palette) {
        ui.add(
            TextEdit::singleline(&mut self.recherche)
                .hint_text(tr!("Chercher : une brique, un mot…"))
                .desired_width(f32::INFINITY),
        );
        ui.add_space(6.0);
        let mut choisie = None;
        ScrollArea::vertical()
            .id_salt("doc-sommaire")
            .auto_shrink(false)
            .show(ui, |ui| {
                let terme = self.recherche.trim().to_owned();
                if !terme.is_empty() {
                    let trouvees = doc::chercher(&terme);
                    if trouvees.is_empty() {
                        ui.label(
                            RichText::new(tr!("Rien trouvé pour « {terme} ».", terme = terme))
                                .color(c.doux.c()),
                        );
                    }
                    for page in trouvees {
                        if entree(ui, &doc::titre(&page), page == self.page, a, c) {
                            choisie = Some(page);
                        }
                    }
                    return;
                }
                if entree(
                    ui,
                    tr!("Accueil de la documentation"),
                    self.page == Page::Sommaire,
                    a,
                    c,
                ) {
                    choisie = Some(Page::Sommaire);
                }
                for partie in doc::sommaire() {
                    ui.add_space(10.0);
                    rubrique(ui, &partie.titre, a, c);
                    for page in partie.pages {
                        if entree(ui, &doc::titre(&page), page == self.page, a, c) {
                            choisie = Some(page);
                        }
                    }
                }
            });
        if let Some(p) = choisie {
            self.aller(p);
        }
    }

    /// La barre au-dessus de la page : retour, suivant, et où l'on est.
    fn barre(&mut self, ui: &mut egui::Ui, c: &Palette) {
        ui.horizontal(|ui| {
            let retour = ui
                .add_enabled(!self.arriere.is_empty(), egui::Button::new("‹"))
                .on_hover_text(tr!("Page d'avant (Alt+←)"));
            if retour.clicked() {
                self.retour();
            }
            let suivant = ui
                .add_enabled(!self.avant.is_empty(), egui::Button::new("›"))
                .on_hover_text(tr!("Page suivante (Alt+→)"));
            if suivant.clicked() {
                self.suivant();
            }
            ui.add_space(6.0);
            let ou = match &self.page {
                Page::Sommaire => tr!("Documentation").to_owned(),
                Page::Notion(cle) => match doc::notion(cle).map(|n| n.famille) {
                    Some(Famille::Piece) => tr!("Documentation › Les pièces de base").to_owned(),
                    _ => tr!("Documentation › Les notions").to_owned(),
                },
                Page::Brique(nom) => match doc::quete_de(nom) {
                    Some(f) => tr!(
                        "Documentation › {chapitre}",
                        chapitre = crate::langue::traduire(f.chapitre)
                    ),
                    None => tr!("Documentation › Briques intermédiaires").to_owned(),
                },
            };
            ui.label(RichText::new(ou).color(c.discret.c()));
        });
    }

    /// La page d'accueil : à quoi sert la documentation, et toutes les pages en vignettes.
    fn accueil(&mut self, ui: &mut egui::Ui, a: &Apparence, c: &Palette) {
        titre(ui, tr!("Documentation"), a, c);
        paragraphe(
            ui,
            tr!("Chaque pièce de la palette, chaque notion qui revient et chaque brique de la bibliothèque a sa page : son rôle, ses broches, ce qu'elle fait (table de vérité ou chronogramme, simulés sur le corrigé), comment elle se construit en NAND, et les quêtes où on la rencontre."),
            c,
        );
        paragraphe(
            ui,
            tr!("F2 ouvre la page de l'élément sélectionné dans le plan. Alt+← revient à la page d'avant."),
            c,
        );
        let mut choisie = None;
        for partie in doc::sommaire() {
            ui.add_space(12.0);
            intertitre(ui, &partie.titre, a, c);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                for page in partie.pages {
                    if lien(ui, &doc::titre(&page), c) {
                        choisie = Some(page);
                    }
                }
            });
        }
        if let Some(p) = choisie {
            self.aller(p);
        }
    }

    /// Une page écrite à la main.
    fn notion(
        &mut self,
        ui: &mut egui::Ui,
        cle: &str,
        at: &Atelier,
        a: &Apparence,
        c: &Palette,
        ouvrir: &mut Option<Ouverture>,
    ) {
        let Some(n) = doc::notion(cle) else {
            return;
        };
        titre(ui, crate::langue::traduire(n.titre), a, c);
        ui.label(
            RichText::new(crate::langue::traduire(n.resume))
                .font(Police::Corps.taille(a.typo.fort * a.formes.echelle_texte))
                .color(c.doux.c()),
        );
        ui.add_space(4.0);
        ecrire(ui, crate::langue::traduire(n.texte), a, c);
        let mut choisie = None;
        if !n.voir.is_empty() {
            ui.add_space(10.0);
            intertitre(ui, tr!("À lire aussi"), a, c);
            ui.horizontal_wrapped(|ui| {
                for cle in n.voir {
                    let page = doc::lien(cle);
                    if lien(ui, &doc::titre(&page), c) {
                        choisie = Some(page);
                    }
                }
            });
        }
        if !n.quetes.is_empty() {
            ui.add_space(10.0);
            intertitre(ui, tr!("Dans les quêtes"), a, c);
            for q in n.quetes {
                if let Some(f) = crate::quete::Quete::fiche(q) {
                    bouton_de_quete(ui, f, at, c, ouvrir);
                }
            }
        }
        if cle == "ordinateur" {
            ui.add_space(6.0);
            if let Some(rang) = crate::exemples::EXEMPLES
                .iter()
                .position(|e| e.nom == "Ordinateur miniature")
            {
                if ui
                    .button(tr!("Ouvrir l'exemple « Ordinateur miniature »"))
                    .clicked()
                {
                    *ouvrir = Some(Ouverture::Exemple(rang));
                }
            }
        }
        if let Some(p) = choisie {
            self.aller(p);
        }
    }

    /// La page d'une brique de la bibliothèque.
    fn brique(
        &mut self,
        ui: &mut egui::Ui,
        nom: &str,
        at: &Atelier,
        a: &Apparence,
        c: &Palette,
        ouvrir: &mut Option<Ouverture>,
    ) {
        let page = Page::Brique(nom.to_owned());
        titre(ui, &doc::titre(&page), a, c);
        let Some(t) = doc::technique(nom) else {
            paragraphe(ui, tr!("Cette brique ne se calcule pas seule."), c);
            return;
        };
        // La carte d'identité : nom, famille, portes, temps de réponse.
        ui.horizontal_wrapped(|ui| {
            pastille(ui, &crate::langue::nom_affiche(nom), true, c);
            pastille(
                ui,
                if t.memoire {
                    tr!("avec mémoire")
                } else {
                    tr!("combinatoire")
                },
                false,
                c,
            );
            if let Some(n) = t.nand {
                pastille(
                    ui,
                    &tr!(
                        "{n} porte{s} NAND",
                        n = doc::milliers(n),
                        s = crate::langue::s(n)
                    ),
                    false,
                    c,
                );
            }
            if let Some(r) = t.reponse {
                pastille(
                    ui,
                    &tr!(
                        "répond en {r} unité{s} de temps",
                        r = r,
                        s = crate::langue::s(r)
                    ),
                    false,
                    c,
                );
            }
        });
        ui.add_space(6.0);
        let composant = crate::quete::corriges().composants.get(nom);
        if let Some(d) = composant
            .map(|c| c.description.trim())
            .filter(|d| !d.is_empty())
        {
            ui.label(
                RichText::new(crate::langue::traduire_texte(d))
                    .font(Police::Corps.taille(a.typo.fort * a.formes.echelle_texte))
                    .color(c.texte.c()),
            );
        }
        let fiche = doc::quete_de(nom);
        let savoir = fiche.and_then(|f| crate::savoir::savoir(f.cle));
        let rubriques = savoir.map(|s| s.rubriques());
        ui.add_space(8.0);
        schema(ui, nom, &t, a, c);
        if let Some(r) = &rubriques {
            for (titre_r, texte) in &r[..2] {
                section(ui, titre_r, texte, a, c);
            }
        }

        // Les broches.
        ui.add_space(10.0);
        intertitre(ui, tr!("Les broches"), a, c);
        broches(ui, &t, a, c);

        // Ce qu'elle fait.
        ui.add_space(10.0);
        match &t.comportement {
            Comportement::Table { lignes, total } => {
                intertitre(ui, tr!("Sa table de vérité"), a, c);
                if *total as usize > lignes.len() {
                    note(
                        ui,
                        &tr!(
                            "{n} lignes sur {total}, tirées au hasard : la table entière serait trop longue. Chaque ligne est simulée sur le corrigé.",
                            n = lignes.len(),
                            total = doc::milliers(*total)
                        ),
                        c,
                    );
                } else {
                    note(ui, tr!("Toutes les combinaisons d'entrées, simulées sur le corrigé. Les nombres sont écrits en décimal."), c);
                }
                table(ui, &t, lignes, a, c);
            }
            Comportement::Chronogramme { signaux, horloge } => {
                intertitre(ui, tr!("Son chronogramme"), a, c);
                note(
                    ui,
                    if *horloge {
                        tr!("Le corrigé simulé depuis la mise sous tension, pendant douze fronts d'horloge : raz au premier cycle ; une mémoire est écrite puis relue ; les autres entrées au hasard. Les traits verticaux marquent les fronts montants ; X (hachuré) veut dire « pas encore connu ».")
                    } else {
                        tr!("Le corrigé simulé depuis la mise sous tension, les entrées changées au hasard, une colonne par changement. X (hachuré) veut dire « pas encore connu ».")
                    },
                    c,
                );
                chronogramme(ui, signaux, *horloge, a, c);
            }
            Comportement::Aucun => {}
        }

        // Comment la construire.
        ui.add_space(10.0);
        intertitre(ui, tr!("Comment la construire"), a, c);
        let reussie = fiche.is_some_and(|f| at.parcours.faite(f.cle));
        let mut choisie = None;
        if fiche.is_some() && !reussie && !self.devoilees.contains(nom) {
            note(ui, tr!("Les pièces qu'assemble le corrigé sont un indice de sa quête : elles restent cachées jusqu'à ce que tu la réussisses."), c);
            if ui.button(tr!("Montrer quand même")).clicked() {
                self.devoilees.insert(nom.to_owned());
            }
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr!("Construite avec :")).color(c.doux.c()));
                for (genre, n) in &t.pieces {
                    let texte = format!("{n} × {}", crate::langue::nom_affiche(genre));
                    let cible = match genre.as_str() {
                        "NAND" => Some(Page::Notion("nand")),
                        "HORLOGE" => Some(Page::Notion("horloge")),
                        g if doc::est_documentee(g) => Some(Page::Brique(g.to_owned())),
                        _ => None,
                    };
                    match cible {
                        Some(p) => {
                            if lien(ui, &texte, c) {
                                choisie = Some(p);
                            }
                        }
                        None => {
                            ui.label(RichText::new(texte).color(c.texte.c()));
                        }
                    }
                }
            });
        }
        if let Some(n) = t.nand {
            let mut phrase = tr!(
                "Une fois tout mis à plat : {n} porte{s} NAND.",
                n = doc::milliers(n),
                s = crate::langue::s(n)
            );
            if let Some(m) = t.minimum.filter(|m| m.nand < n) {
                phrase.push(' ');
                phrase.push_str(&tr!(
                    "La plus petite construction connue en demande {m}.",
                    m = doc::milliers(m.nand)
                ));
            } else if let Some(m) = t.minimum {
                if matches!(m.statut, ce_circuit::minimum::Statut::Prouve) {
                    phrase.push(' ');
                    phrase.push_str(tr!("C'est le minimum, et c'est démontré."));
                }
            }
            paragraphe(ui, &phrase, c);
        }

        if let Some(r) = &rubriques {
            for (titre_r, texte) in &r[2..] {
                section(ui, titre_r, texte, a, c);
            }
        }

        // Où on la retrouve.
        let utilisee = doc::utilisee_par(nom);
        if !utilisee.is_empty() {
            ui.add_space(10.0);
            intertitre(ui, tr!("Utilisée par"), a, c);
            ui.horizontal_wrapped(|ui| {
                for u in utilisee {
                    let p = Page::Brique(u);
                    if lien(ui, &doc::titre(&p), c) {
                        choisie = Some(p);
                    }
                }
            });
        }
        let quetes = doc::quetes_de(nom);
        if !quetes.is_empty() {
            ui.add_space(10.0);
            intertitre(ui, tr!("Dans les quêtes"), a, c);
            for f in quetes {
                bouton_de_quete(ui, f, at, c, ouvrir);
            }
        }
        if let Some(p) = choisie {
            self.aller(p);
        }
    }
}

// --- Petits morceaux d'interface -------------------------------------------------------------

fn titre(ui: &mut egui::Ui, texte: &str, a: &Apparence, c: &Palette) {
    ui.add_space(4.0);
    ui.label(
        RichText::new(texte)
            .font(Police::Titre.taille(a.typo.grand * a.formes.echelle_texte))
            .color(c.texte.c()),
    );
    ui.add_space(2.0);
}

fn intertitre(ui: &mut egui::Ui, texte: &str, a: &Apparence, c: &Palette) {
    ui.add_space(4.0);
    ui.label(
        RichText::new(texte)
            .font(Police::CorpsDemi.taille(a.typo.fort * a.formes.echelle_texte))
            .color(c.texte.c()),
    );
    ui.add_space(2.0);
}

/// Une petite capitale de rubrique, dans le sommaire.
fn rubrique(ui: &mut egui::Ui, texte: &str, a: &Apparence, c: &Palette) {
    ui.label(
        RichText::new(texte.to_uppercase())
            .font(Police::CorpsDemi.taille(a.typo.legende * a.formes.echelle_texte))
            .color(c.discret.c()),
    );
}

fn paragraphe(ui: &mut egui::Ui, texte: &str, c: &Palette) {
    ui.label(RichText::new(texte).color(c.doux.c()));
    ui.add_space(4.0);
}

fn note(ui: &mut egui::Ui, texte: &str, c: &Palette) {
    ui.label(RichText::new(texte).color(c.discret.c()));
    ui.add_space(2.0);
}

fn section(ui: &mut egui::Ui, titre_s: &str, texte: &str, a: &Apparence, c: &Palette) {
    if texte.trim().is_empty() {
        return;
    }
    ui.add_space(8.0);
    intertitre(ui, titre_s, a, c);
    paragraphe(ui, texte, c);
}

/// Une entrée du sommaire : nue au repos, enfoncée quand c'est la page montrée.
fn entree(ui: &mut egui::Ui, texte: &str, choisie: bool, a: &Apparence, c: &Palette) -> bool {
    let bouton = egui::Button::new(
        RichText::new(texte)
            .font(Police::Corps.taille(a.typo.corps * a.formes.echelle_texte))
            .color(if choisie { c.texte.c() } else { c.doux.c() }),
    )
    .fill(if choisie {
        c.enfonce.c()
    } else {
        Color32::TRANSPARENT
    })
    .stroke(Stroke::NONE)
    .frame_when_inactive(choisie)
    .wrap_mode(egui::TextWrapMode::Truncate)
    .min_size(Vec2::new(ui.available_width(), 0.0));
    let r = ui.add(bouton);
    if choisie {
        // Le repère d'accent à gauche de l'entrée choisie.
        let barre = Rect::from_min_size(
            r.rect.left_top() + Vec2::new(0.0, 5.0),
            Vec2::new(2.0, r.rect.height() - 10.0),
        );
        ui.painter().rect_filled(barre, 1.0, c.accent_clair.c());
    }
    r.clicked()
}

/// Un lien vers une autre page : une petite carte au texte en accent.
fn lien(ui: &mut egui::Ui, texte: &str, c: &Palette) -> bool {
    ui.add(
        egui::Button::new(RichText::new(texte).color(c.accent_clair.c()))
            .fill(c.carte_2.c())
            .stroke(Stroke::new(1.0, c.bord.c())),
    )
    .clicked()
}

/// Une pastille d'information, en chasse fixe pour un nom de brique.
fn pastille(ui: &mut egui::Ui, texte: &str, mono: bool, c: &Palette) {
    let mut t = RichText::new(texte).color(c.doux.c());
    if mono {
        t = t.monospace().color(c.texte.c());
    }
    egui::Frame::new()
        .fill(c.carte_2.c())
        .stroke(Stroke::new(1.0, c.bord.c()))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::symmetric(6, 2))
        .show(ui, |ui| ui.label(t));
}

/// Le bouton qui ouvre une quête, avec une coche si elle est déjà réussie.
fn bouton_de_quete(
    ui: &mut egui::Ui,
    f: &'static crate::quete::Fiche,
    at: &Atelier,
    c: &Palette,
    ouvrir: &mut Option<Ouverture>,
) {
    ui.horizontal(|ui| {
        let faite = at.parcours.faite(f.cle);
        if ui
            .button(tr!(
                "Ouvrir la quête « {titre} »",
                titre = crate::langue::traduire(f.titre)
            ))
            .clicked()
        {
            *ouvrir = Some(Ouverture::Quete(f.cle.to_owned()));
        }
        let etat = if faite {
            RichText::new(tr!("✓ réussie")).color(c.un.c())
        } else {
            RichText::new(crate::langue::traduire(f.chapitre)).color(c.discret.c())
        };
        ui.label(etat);
    });
}

/// Dessine un texte balisé : intertitres `# `, puces `- `, tableaux `| `, paragraphes.
pub fn ecrire(ui: &mut egui::Ui, texte: &str, a: &Apparence, c: &Palette) {
    let lignes: Vec<&str> = texte.lines().collect();
    let mut i = 0;
    let mut tableaux = 0;
    while i < lignes.len() {
        let ligne = lignes[i];
        if ligne.starts_with("| ") {
            let debut = i;
            while i < lignes.len() && lignes[i].starts_with("| ") {
                i += 1;
            }
            tableaux += 1;
            tableau(ui, &lignes[debut..i], tableaux, a, c);
            continue;
        }
        if let Some(t) = ligne.strip_prefix("# ") {
            ui.add_space(8.0);
            intertitre(ui, t, a, c);
        } else if let Some(puce) = ligne.strip_prefix("- ") {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("•").color(c.accent_clair.c()));
                ui.label(RichText::new(puce).color(c.doux.c()));
            });
        } else if ligne.trim().is_empty() {
            ui.add_space(4.0);
        } else {
            ui.label(RichText::new(ligne).color(c.doux.c()));
        }
        i += 1;
    }
}

/// Un tableau écrit dans le texte : la première ligne est l'en-tête.
fn tableau(ui: &mut egui::Ui, lignes: &[&str], rang: usize, a: &Apparence, c: &Palette) {
    ui.add_space(4.0);
    cadre_de_tableau(c).show(ui, |ui| {
        egui::Grid::new(("doc-tableau", rang))
            .striped(true)
            .spacing(Vec2::new(16.0, 4.0))
            .show(ui, |ui| {
                for (k, ligne) in lignes.iter().enumerate() {
                    let ligne = ligne.trim_start_matches("| ");
                    for case in ligne.split(" | ") {
                        let t = RichText::new(case.trim());
                        ui.label(if k == 0 {
                            t.font(Police::CorpsDemi.taille(a.typo.petit * a.formes.echelle_texte))
                                .color(c.texte.c())
                        } else {
                            t.color(c.doux.c())
                        });
                    }
                    ui.end_row();
                }
            });
    });
    ui.add_space(4.0);
}

fn cadre_de_tableau(c: &Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(c.carte_2.c())
        .stroke(Stroke::new(1.0, c.bord.c()))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(10, 6))
}

/// Une valeur lue : décimal pour un nombre, `X` quand elle n'est pas connue.
fn valeur(v: Option<u64>) -> String {
    v.map_or_else(|| "X".to_owned(), |v| v.to_string())
}

/// La même en binaire, poids fort à gauche, pour l'info-bulle d'un nombre.
fn en_binaire(v: Option<u64>, largeur: u32) -> String {
    v.map_or_else(
        || "X".repeat(largeur as usize),
        |v| format!("{v:0width$b}", width = largeur as usize),
    )
}

/// Le tableau des broches : nom, sens, bits, rôle.
fn broches(ui: &mut egui::Ui, t: &Technique, a: &Apparence, c: &Palette) {
    cadre_de_tableau(c).show(ui, |ui| {
        egui::Grid::new("doc-broches")
            .striped(true)
            .spacing(Vec2::new(16.0, 4.0))
            .show(ui, |ui| {
                let entete = |ui: &mut egui::Ui, s: &str| {
                    ui.label(
                        RichText::new(s)
                            .font(Police::CorpsDemi.taille(a.typo.petit * a.formes.echelle_texte))
                            .color(c.texte.c()),
                    );
                };
                entete(ui, tr!("Broche"));
                entete(ui, tr!("Sens"));
                entete(ui, tr!("Bits"));
                entete(ui, tr!("Rôle"));
                ui.end_row();
                let lignes = t
                    .entrees
                    .iter()
                    .map(|b| (b, true))
                    .chain(t.sorties.iter().map(|b| (b, false)));
                for ((nom, largeur), entree) in lignes {
                    ui.label(
                        RichText::new(crate::langue::nom_affiche(nom))
                            .monospace()
                            .color(c.texte.c()),
                    );
                    ui.label(
                        RichText::new(if entree {
                            tr!("entrée")
                        } else {
                            tr!("sortie")
                        })
                        .color(c.doux.c()),
                    );
                    ui.label(
                        RichText::new(largeur.to_string())
                            .monospace()
                            .color(c.doux.c()),
                    );
                    let role =
                        doc::role_de_broche(nom, entree).map_or("—", crate::langue::traduire);
                    ui.label(RichText::new(role).color(c.doux.c()));
                    ui.end_row();
                }
            });
    });
}

/// La table de vérité, entrées en doux et sorties en fort ; les 1 d'un bit en vert.
fn table(ui: &mut egui::Ui, t: &Technique, lignes: &[Ligne], a: &Apparence, c: &Palette) {
    ScrollArea::horizontal()
        .id_salt("doc-table")
        .show(ui, |ui| {
            cadre_de_tableau(c).show(ui, |ui| {
                egui::Grid::new("doc-table-grille")
                    .striped(true)
                    .spacing(Vec2::new(14.0, 3.0))
                    .show(ui, |ui| {
                        let police = Police::MonoGras.taille(a.typo.petit * a.formes.echelle_texte);
                        for (nom, _) in &t.entrees {
                            ui.label(
                                RichText::new(crate::langue::nom_affiche(nom))
                                    .font(police.clone())
                                    .color(c.doux.c()),
                            );
                        }
                        ui.label(
                            RichText::new("│")
                                .font(police.clone())
                                .color(c.bord_fort.c()),
                        );
                        for (nom, _) in &t.sorties {
                            ui.label(
                                RichText::new(crate::langue::nom_affiche(nom))
                                    .font(police.clone())
                                    .color(c.texte.c()),
                            );
                        }
                        ui.end_row();
                        let case =
                            |ui: &mut egui::Ui, v: Option<u64>, largeur: u32, sortie: bool| {
                                let couleur = match v {
                                    None => c.x.c(),
                                    Some(1) if largeur == 1 => c.un.c(),
                                    _ if sortie => c.texte.c(),
                                    _ => c.doux.c(),
                                };
                                let r =
                                    ui.label(RichText::new(valeur(v)).monospace().color(couleur));
                                if largeur > 1 {
                                    r.on_hover_text(en_binaire(v, largeur));
                                }
                            };
                        for l in lignes {
                            for (v, (_, largeur)) in l.entrees.iter().zip(&t.entrees) {
                                case(ui, *v, *largeur, false);
                            }
                            ui.label(
                                RichText::new("│")
                                    .font(police.clone())
                                    .color(c.bord_fort.c()),
                            );
                            for (v, (_, largeur)) in l.sorties.iter().zip(&t.sorties) {
                                case(ui, *v, *largeur, true);
                            }
                            ui.end_row();
                        }
                    });
            });
        });
}

/// Le schéma de la brique : une boîte, ses entrées à gauche et ses sorties à droite, comme
/// elle se pose dans le plan. Un bus porte un trait oblique et sa largeur.
fn schema(ui: &mut egui::Ui, nom: &str, t: &Technique, a: &Apparence, c: &Palette) {
    let pas = 22.0;
    let n = t.entrees.len().max(t.sorties.len()).max(1) as f32;
    let entete = 30.0;
    let largeur_boite: f32 = 190.0;
    let patte = 34.0;
    let hauteur = entete + n * pas + 10.0;
    let largeur = (largeur_boite + 2.0 * patte + 120.0).min(ui.available_width());
    let (r, p) = ui.allocate_painter(Vec2::new(largeur, hauteur + 8.0), Sense::hover());
    let boite = Rect::from_center_size(
        Pos2::new(r.rect.center().x, r.rect.top() + 4.0 + hauteur / 2.0),
        Vec2::new(largeur_boite, hauteur),
    );
    p.rect(
        boite,
        8.0,
        c.carte_2.c(),
        Stroke::new(1.0, c.bord_fort.c()),
        egui::StrokeKind::Inside,
    );
    let echelle = a.formes.echelle_texte;
    p.text(
        Pos2::new(boite.center().x, boite.top() + entete / 2.0),
        egui::Align2::CENTER_CENTER,
        crate::langue::nom_affiche(nom),
        Police::MonoGras.taille(a.typo.corps * echelle),
        c.texte.c(),
    );
    p.line_segment(
        [
            Pos2::new(boite.left() + 8.0, boite.top() + entete - 4.0),
            Pos2::new(boite.right() - 8.0, boite.top() + entete - 4.0),
        ],
        Stroke::new(1.0, c.trait_.c()),
    );
    let police = Police::Mono.taille(a.typo.petit * echelle);
    let cote = |broches: &[(String, u32)], gauche: bool| {
        // Les broches centrées sur la hauteur disponible, comme dans le plan.
        let decalage = (n - broches.len() as f32) * pas / 2.0;
        for (k, (b, l)) in broches.iter().enumerate() {
            let y = boite.top() + entete + decalage + (k as f32 + 0.5) * pas;
            let (x0, x1) = if gauche {
                (boite.left() - patte, boite.left())
            } else {
                (boite.right(), boite.right() + patte)
            };
            let epaisseur = if *l > 1 { 2.5 } else { 1.5 };
            p.line_segment(
                [Pos2::new(x0, y), Pos2::new(x1, y)],
                Stroke::new(epaisseur, c.bord_fort.c()),
            );
            p.circle_filled(
                Pos2::new(if gauche { x0 } else { x1 }, y),
                3.0,
                c.accent_clair.c(),
            );
            if *l > 1 {
                let xm = (x0 + x1) / 2.0;
                p.line_segment(
                    [Pos2::new(xm - 4.0, y + 5.0), Pos2::new(xm + 4.0, y - 5.0)],
                    Stroke::new(1.0, c.doux.c()),
                );
                p.text(
                    Pos2::new(xm + 3.0, y - 6.0),
                    egui::Align2::LEFT_BOTTOM,
                    l.to_string(),
                    Police::Mono.taille(a.typo.legende * echelle),
                    c.discret.c(),
                );
            }
            let (ancre, x) = if gauche {
                (egui::Align2::LEFT_CENTER, boite.left() + 8.0)
            } else {
                (egui::Align2::RIGHT_CENTER, boite.right() - 8.0)
            };
            p.text(
                Pos2::new(x, y),
                ancre,
                crate::langue::nom_affiche(b),
                police.clone(),
                c.doux.c(),
            );
        }
    };
    cote(&t.entrees, true);
    cote(&t.sorties, false);
}

/// Des hachures obliques dans un rectangle : la marque d'un X, qui ne dépend pas de la couleur.
fn hachurer(p: &egui::Painter, r: Rect, couleur: Color32) {
    let p = p.with_clip_rect(r.intersect(p.clip_rect()));
    let mut x = r.left() - r.height();
    while x < r.right() {
        p.line_segment(
            [Pos2::new(x, r.bottom()), Pos2::new(x + r.height(), r.top())],
            Stroke::new(1.0, couleur),
        );
        x += 6.0;
    }
}

/// Le chronogramme : une ligne par signal, une colonne par pas.
fn chronogramme(ui: &mut egui::Ui, signaux: &[Signal], horloge: bool, a: &Apparence, c: &Palette) {
    let pas = signaux.first().map_or(0, |s| s.valeurs.len());
    if pas == 0 {
        return;
    }
    let echelle = a.formes.echelle_texte;
    let police = Police::Mono.taille(a.typo.petit * echelle);
    let etiquettes = signaux
        .iter()
        .map(|s| {
            ui.painter()
                .layout_no_wrap(
                    crate::langue::nom_affiche(&s.nom).into_owned(),
                    police.clone(),
                    c.texte.c(),
                )
                .size()
                .x
        })
        .fold(40.0_f32, f32::max)
        + 14.0;
    // Une colonne assez large pour un octet écrit en décimal.
    let large = signaux.iter().any(|s| s.largeur > 4);
    let colonne: f32 = if large { 30.0 } else { 22.0 };
    let haut = 22.0;
    let ecart = 6.0;
    let largeur = etiquettes + colonne * pas as f32 + 8.0;
    let hauteur = signaux.len() as f32 * (haut + ecart) + 6.0;
    ScrollArea::horizontal()
        .id_salt("doc-chrono")
        .show(ui, |ui| {
            let (r, p) = ui.allocate_painter(Vec2::new(largeur, hauteur), Sense::hover());
            let fond = r.rect;
            p.rect_filled(fond, 6.0, c.carte_2.c());
            let x0 = fond.left() + etiquettes;
            // Les fronts montants : entre le pas « horloge à 0 » et le pas « horloge à 1 ».
            if horloge {
                for k in (1..pas).step_by(2) {
                    let x = x0 + k as f32 * colonne;
                    p.line_segment(
                        [
                            Pos2::new(x, fond.top() + 2.0),
                            Pos2::new(x, fond.bottom() - 2.0),
                        ],
                        Stroke::new(1.0, c.trait_.c()),
                    );
                }
            }
            let mut y = fond.top() + 4.0;
            let mut entree_avant = true;
            for s in signaux {
                if entree_avant && !s.entree {
                    // Un trait sépare les entrées des sorties.
                    p.line_segment(
                        [
                            Pos2::new(fond.left() + 6.0, y - ecart / 2.0),
                            Pos2::new(fond.right() - 6.0, y - ecart / 2.0),
                        ],
                        Stroke::new(1.0, c.bord.c()),
                    );
                    entree_avant = false;
                }
                let bande =
                    Rect::from_min_size(Pos2::new(x0, y), Vec2::new(colonne * pas as f32, haut));
                p.text(
                    Pos2::new(fond.left() + 8.0, bande.center().y),
                    egui::Align2::LEFT_CENTER,
                    crate::langue::nom_affiche(&s.nom),
                    police.clone(),
                    if s.entree { c.doux.c() } else { c.texte.c() },
                );
                if s.largeur == 1 {
                    un_bit(&p, bande, &s.valeurs, colonne, c);
                } else {
                    bus(&p, bande, &s.valeurs, colonne, a, c);
                }
                y += haut + ecart;
            }
        });
}

/// Un signal d'un bit : haut à 1, bas à 0, hachuré quand il est inconnu.
fn un_bit(p: &egui::Painter, bande: Rect, valeurs: &[Option<u64>], colonne: f32, c: &Palette) {
    let (haut, bas) = (bande.top() + 3.0, bande.bottom() - 3.0);
    let mut precedent: Option<f32> = None;
    for (k, v) in valeurs.iter().enumerate() {
        let x = bande.left() + k as f32 * colonne;
        match v {
            Some(b) => {
                let y = if *b == 1 { haut } else { bas };
                let couleur = if *b == 1 { c.un.c() } else { c.doux.c() };
                if *b == 1 {
                    p.rect_filled(
                        Rect::from_min_max(Pos2::new(x, haut), Pos2::new(x + colonne, bas)),
                        0.0,
                        c.un.alpha(0.12),
                    );
                }
                if let Some(yp) = precedent {
                    if (yp - y).abs() > 0.5 {
                        p.line_segment(
                            [Pos2::new(x, yp), Pos2::new(x, y)],
                            Stroke::new(1.5, couleur),
                        );
                    }
                }
                p.line_segment(
                    [Pos2::new(x, y), Pos2::new(x + colonne, y)],
                    Stroke::new(1.5, couleur),
                );
                precedent = Some(y);
            }
            None => {
                let r = Rect::from_min_max(Pos2::new(x, haut), Pos2::new(x + colonne, bas));
                hachurer(p, r, c.x.alpha(0.7));
                precedent = None;
            }
        }
    }
}

/// Un bus : une case par valeur tenue, avec sa valeur en décimal ; hachurée quand elle est
/// inconnue.
fn bus(
    p: &egui::Painter,
    bande: Rect,
    valeurs: &[Option<u64>],
    colonne: f32,
    a: &Apparence,
    c: &Palette,
) {
    let (haut, bas) = (bande.top() + 2.0, bande.bottom() - 2.0);
    let mut k = 0;
    while k < valeurs.len() {
        let v = valeurs[k];
        let mut fin = k + 1;
        while fin < valeurs.len() && valeurs[fin] == v {
            fin += 1;
        }
        let r = Rect::from_min_max(
            Pos2::new(bande.left() + k as f32 * colonne + 1.0, haut),
            Pos2::new(bande.left() + fin as f32 * colonne - 1.0, bas),
        );
        match v {
            Some(n) => {
                p.rect(
                    r,
                    4.0,
                    c.carte.c(),
                    Stroke::new(1.0, c.bord_fort.c()),
                    egui::StrokeKind::Inside,
                );
                let texte = n.to_string();
                let police = Police::Mono.taille(a.typo.legende * a.formes.echelle_texte);
                let galee = p.layout_no_wrap(texte, police, c.texte.c());
                if galee.size().x + 4.0 <= r.width() {
                    p.galley(r.center() - galee.size() / 2.0, galee, c.texte.c());
                }
            }
            None => {
                p.rect_stroke(
                    r,
                    4.0,
                    Stroke::new(1.0, c.x.alpha(0.7)),
                    egui::StrokeKind::Inside,
                );
                hachurer(p, r.shrink(1.0), c.x.alpha(0.6));
            }
        }
        k = fin;
    }
}
