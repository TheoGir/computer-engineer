//! Les fenêtres posées par-dessus l'écran : l'aide (F1) et l'éditeur de programme de la
//! mémoire morte (Ctrl+P). Elles utilisent les widgets d'egui, aux couleurs de l'apparence.

use egui::{Align, Color32, Layout, RichText, ScrollArea, TextEdit};

use crate::apparence::Apparence;
use crate::dessin::Police;
use crate::etat::Atelier;
use crate::programme::{PROGRAMMES, TAILLE_MAXIMALE};

/// Les rubriques de l'aide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rubrique {
    /// Pour commencer.
    PremiersPas,
    /// Raccourcis clavier et souris.
    Raccourcis,
    /// Programmer l'ordinateur miniature.
    Programmer,
    /// Où sont les fichiers, sauvegardes, mises à jour, rapports.
    Fichiers,
    /// Version, liens et licences.
    APropos,
}

impl Rubrique {
    /// Toutes, dans l'ordre des onglets.
    pub const TOUTES: [Rubrique; 5] = [
        Rubrique::PremiersPas,
        Rubrique::Raccourcis,
        Rubrique::Programmer,
        Rubrique::Fichiers,
        Rubrique::APropos,
    ];

    /// Le titre de l'onglet.
    pub fn titre(self) -> &'static str {
        match self {
            Rubrique::PremiersPas => "Premiers pas",
            Rubrique::Raccourcis => "Raccourcis",
            Rubrique::Programmer => "Programmer l'ordinateur",
            Rubrique::Fichiers => "Fichiers et mises à jour",
            Rubrique::APropos => "À propos",
        }
    }
}

/// L'éditeur du programme d'une mémoire morte.
#[derive(Clone, Debug, PartialEq)]
pub struct Editeur {
    /// La brique de mémoire morte modifiée.
    pub modele: String,
    /// Le texte en cours d'écriture.
    pub texte: String,
    /// Le résultat du dernier essai d'assemblage : erreur, ou nombre d'instructions.
    pub verdict: Result<usize, String>,
}

impl Editeur {
    /// L'éditeur ouvert sur le programme actuel de cette brique.
    pub fn sur(at: &Atelier, modele: &str) -> Editeur {
        let texte = at
            .projet
            .modele(modele)
            .and_then(|m| m.programme.clone())
            .unwrap_or_default();
        let mut e = Editeur {
            modele: modele.to_owned(),
            texte,
            verdict: Ok(0),
        };
        e.essayer();
        e
    }

    /// Assemble le texte, sans rien charger, pour dire tout de suite ce qui ne va pas.
    pub fn essayer(&mut self) {
        self.verdict = ce_circuit::ordinateur::assembler(&self.texte)
            .map(|code| code.len())
            .map_err(|e| e.to_string());
    }
}

/// Les fenêtres ouvertes.
#[derive(Debug, Default)]
pub struct Fenetres {
    /// L'aide, ouverte sur cette rubrique.
    pub aide: Option<Rubrique>,
    /// L'éditeur de programme.
    pub editeur: Option<Editeur>,
}

/// Le texte de chaque rubrique : des paragraphes, `# ` pour un intertitre, `- ` pour une puce.
pub fn texte_de(rubrique: Rubrique) -> String {
    match rubrique {
        Rubrique::PremiersPas => PREMIERS_PAS.to_owned(),
        Rubrique::Raccourcis => RACCOURCIS.to_owned(),
        Rubrique::Programmer => {
            let mut t = PROGRAMMER.to_owned();
            t.push_str("\n# Les seize instructions\n");
            for (op, sens) in INSTRUCTIONS {
                t.push_str(&format!("- {op} : {sens}\n"));
            }
            t
        }
        Rubrique::Fichiers => FICHIERS.to_owned(),
        Rubrique::APropos => format!(
            "# Computer Engineer {}\nConstruire un ordinateur complet à partir d'une seule porte logique, la NAND, dans un simulateur qui reproduit son comportement logique et temporel.\n\nLes versions : https://github.com/TheoGir/computer-engineer-releases\n\n# Bibliothèques et polices\nComputer Engineer est fait avec des bibliothèques libres, dont voici les licences.\n\n{}",
            env!("CARGO_PKG_VERSION"),
            crate::licences::RESUME
        ),
    }
}

const PREMIERS_PAS: &str = "# Le principe
Tout est fait d'une seule porte, la NAND : sa sortie vaut 0 seulement quand ses deux entrées valent 1. Avec elle, on fabrique toutes les autres portes, puis des additionneurs, des mémoires, un processeur, et enfin un ordinateur entier.

# Le plan
- Glisse une porte NAND, un interrupteur ou une lampe depuis la palette de gauche.
- Tire un fil d'un point de connexion à un autre.
- Clique sur un interrupteur pour le basculer : le courant se propage porte après porte.
- Molette pour zoomer, glisser le fond pour se déplacer, F pour tout recadrer.

# Les briques
Sélectionne plusieurs éléments (Maj + glisser un cadre, ou Maj + clic), puis Ctrl+G : ils deviennent une brique, rangée dans la palette et réutilisable partout. Double-clic sur une brique pour entrer dedans ; Échap ou le fil d'Ariane pour remonter.

# Les quêtes
Le parcours mène pas à pas de la porte NON jusqu'à l'ordinateur. Chaque quête dit quoi construire ; « Vérifier » compare ton circuit à un corrigé prouvé, et chaque quête réussie devient une brique pour les suivantes.

# Les couleurs du courant
- 1 : le fil est allumé.
- 0 : le fil est éteint.
- X : valeur inconnue ou en conflit (deux sorties qui se contredisent, une boucle instable).
- Z : rien ne pilote ce fil.";

const RACCOURCIS: &str = "# Partout
- F1 : ouvrir ou fermer cette aide.

# Dans l'atelier
- Espace : lecture ou pause de la simulation.
- Suppr : supprimer la sélection.
- Ctrl+Z, Ctrl+Y : annuler, rétablir.
- Ctrl+A : tout sélectionner.
- Ctrl+G : regrouper la sélection en brique.
- Ctrl+D : dupliquer la sélection.
- Ctrl+P : écrire le programme de la mémoire morte de l'ordinateur.
- Échap : lâcher la sélection, puis remonter d'une brique.
- F : recadrer le plan.
- Flèches : déplacer la sélection.
- Molette : zoomer ; glisser le fond : se déplacer.
- Double-clic sur une brique : entrer dedans.

# Clavier du circuit
Quand un clavier du circuit est sélectionné, ce que tu tapes va au circuit, comme sur un vrai clavier branché à l'ordinateur.";

const PROGRAMMER: &str = "# La mémoire morte
La mémoire morte de l'ordinateur miniature contient son programme. Ici, elle n'est pas un tableau de nombres caché : c'est un vrai circuit de portes NAND, une ligne par instruction, fabriqué à partir du programme.

Ouvre l'exemple « Ordinateur miniature » depuis tes projets, puis Ctrl+P : écris un programme en assembleur, « Charger dans la mémoire », et l'ordinateur redémarre avec. La mémoire morte est refabriquée porte par porte : double-clique dessus pour voir ses nouvelles portes.

# Écrire un programme
- Une ligne par instruction, par exemple LDI 'A' puis OUT.
- Une étiquette se termine par deux-points (boucle:) ; JMP boucle y saute.
- Une ligne nom = valeur nomme une case de mémoire ou une valeur.
- Un nombre s'écrit 42, 0x2A ou 0b101010 ; un caractère entre apostrophes, 'A'.
- Tout ce qui suit un point-virgule est un commentaire.
- A est l'accumulateur, M[m] la case m de la mémoire vive (16 cases), Z et C les indicateurs zéro et retenue du dernier calcul.";

const FICHIERS: &str = "# Où sont tes projets
Tout est enregistré tout seul, une seconde après chaque modification, dans le dossier de données (sous Windows : %APPDATA%\\Computer Engineer). Mes projets, « Ouvrir le dossier », y mène.
- projets : un fichier JSON lisible par projet ; corbeille : les projets mis à la corbeille.
- parcours.json : les quêtes réussies et les briques gagnées.
- reglages.json : tes paramètres.
- avant-migration : l'original d'un fichier venu d'une version plus ancienne, gardé avant sa mise au format actuel.
- rapports : ce qui s'est passé si l'application s'est arrêtée brusquement. Ils ne contiennent aucun de tes projets.

# Les mises à jour
L'application cherche une nouvelle version au lancement, vérifie sa signature avant de remplacer quoi que ce soit, puis propose de redémarrer. Une version plus récente sait toujours ouvrir les projets des plus anciennes. On peut couper cette recherche dans les paramètres et chercher à la main.

# Premier lancement sous Windows
L'exécutable n'est pas signé par un certificat payant : au tout premier lancement, Windows peut afficher « Windows a protégé votre ordinateur ». Clique sur « Informations complémentaires », puis « Exécuter quand même ». Les mises à jour suivantes, elles, sont vérifiées par leur signature.";

/// Les instructions du processeur, avec leur effet.
pub const INSTRUCTIONS: [(&str, &str); 16] = [
    ("NOP", "ne fait rien"),
    ("STA m", "M[m] ← A, range l'accumulateur dans la case m"),
    ("IN", "A ← la touche du clavier"),
    ("OUT", "affiche le caractère A à l'écran"),
    ("LDA m", "A ← M[m]"),
    ("ADD m", "A ← A + M[m]"),
    ("SUB m", "A ← A − M[m]"),
    ("NAND m", "A ← non (A et M[m])"),
    ("LDI n", "A ← n"),
    ("ADDI n", "A ← A + n"),
    ("SUBI n", "A ← A − n"),
    ("NANDI n", "A ← non (A et n)"),
    ("JMP a", "saute à a"),
    ("JZ a", "saute à a si le dernier résultat vaut zéro"),
    ("JC a", "saute à a s'il y a eu une retenue"),
    ("HLT", "arrête l'ordinateur"),
];

/// Dessine un texte d'aide : intertitres, puces, paragraphes.
fn ecrire(ui: &mut egui::Ui, texte: &str, a: &Apparence) {
    let c = a.palette();
    let e = a.formes.echelle_texte;
    for ligne in texte.lines() {
        if let Some(titre) = ligne.strip_prefix("# ") {
            ui.add_space(10.0);
            ui.label(
                RichText::new(titre)
                    .font(Police::CorpsDemi.taille(a.typo.fort * e))
                    .color(c.texte.c()),
            );
            ui.add_space(2.0);
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
    }
}

impl Fenetres {
    /// Ouvre ou ferme l'aide.
    pub fn basculer_aide(&mut self) {
        self.aide = match self.aide {
            Some(_) => None,
            None => Some(Rubrique::PremiersPas),
        };
    }

    /// Ouvre l'éditeur de programme sur la mémoire morte la plus à propos, ou dit pourquoi
    /// il n'y en a pas.
    pub fn ouvrir_l_editeur(&mut self, at: &mut Atelier) {
        match at.memoire_programmable() {
            Some(m) => self.editeur = Some(Editeur::sur(at, &m)),
            None => at.dire_longtemps(
                "Ce projet n'a pas de mémoire morte programmable. Ouvre l'exemple « Ordinateur miniature » dans tes projets, puis Ctrl+P.",
            ),
        }
    }

    /// Vrai si une fenêtre est ouverte.
    pub fn ouvertes(&self) -> bool {
        self.aide.is_some() || self.editeur.is_some()
    }

    /// Dessine les fenêtres ouvertes. L'éditeur n'apparaît que dans l'atelier.
    pub fn montrer(&mut self, ctx: &egui::Context, at: &mut Atelier, a: &Apparence, atelier: bool) {
        let c = a.palette();
        let e = a.formes.echelle_texte;
        // Le cadre des fenêtres : une carte vitrine, comme les panneaux de l'atelier.
        let cadre = egui::Frame::window(&ctx.global_style())
            .fill(c.carte.c())
            .stroke(egui::Stroke::new(1.0, c.bord.c()))
            .corner_radius(a.formes.rayon_vitrine)
            .inner_margin(a.formes.rembourrage);
        // Bouton principal : accent plein, texte blanc, un seul par fenêtre.
        let principal = |texte: &str| {
            egui::Button::new(
                RichText::new(texte)
                    .font(Police::CorpsDemi.taille(a.typo.corps * e))
                    .color(Color32::WHITE),
            )
            .fill(c.accent.c())
            .stroke(egui::Stroke::NONE)
            .min_size(egui::vec2(0.0, a.formes.controle + 4.0))
        };
        if let Some(rubrique) = &mut self.aide {
            let mut ouverte = true;
            egui::Window::new("Aide")
                .id(egui::Id::unique("fenetre-aide"))
                .open(&mut ouverte)
                .collapsible(false)
                .resizable(true)
                .default_size([620.0, 520.0])
                .frame(cadre)
                .show(ctx, |ui| {
                    // Les rubriques en contrôle segmenté : la rubrique choisie est enfoncée,
                    // son texte en accent.
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                        for r in Rubrique::TOUTES {
                            let choisie = *rubrique == r;
                            let bouton =
                                egui::Button::new(RichText::new(r.titre()).color(if choisie {
                                    c.accent_clair.c()
                                } else {
                                    c.doux.c()
                                }))
                                .fill(if choisie {
                                    c.enfonce.c()
                                } else {
                                    Color32::TRANSPARENT
                                })
                                .stroke(if choisie {
                                    egui::Stroke::new(1.0, c.bord_fort.c())
                                } else {
                                    egui::Stroke::NONE
                                });
                            if ui.add(bouton).clicked() {
                                *rubrique = r;
                            }
                        }
                    });
                    ui.add_space(4.0);
                    ui.separator();
                    ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                        ecrire(ui, &texte_de(*rubrique), a);
                        if *rubrique == Rubrique::APropos {
                            ui.add_space(8.0);
                            if ui.button("Lire les textes complets des licences").clicked() {
                                let dossier = std::env::temp_dir().join("Computer Engineer");
                                let ouvert = crate::licences::ecrire(&dossier)
                                    .and_then(|f| crate::incident::ouvrir_dans_l_explorateur(&f));
                                if let Err(e) = ouvert {
                                    at.dire_longtemps(&format!(
                                        "Les licences n'ont pas pu être ouvertes : {e}"
                                    ));
                                }
                            }
                        }
                    });
                });
            if !ouverte {
                self.aide = None;
            }
        }

        if !atelier {
            return;
        }
        let Some(editeur) = &mut self.editeur else {
            return;
        };
        let mut ouverte = true;
        let mut charger = false;
        let danger = c.danger.c();
        egui::Window::new(format!("Programme de la mémoire « {} »", editeur.modele))
            .id(egui::Id::unique("fenetre-programme"))
            .open(&mut ouverte)
            .collapsible(false)
            .resizable(true)
            .default_size([640.0, 560.0])
            .frame(cadre)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Partir d'un exemple :").color(c.doux.c()));
                    for (nom, texte) in PROGRAMMES {
                        if ui.button(nom).clicked() {
                            editeur.texte = texte.to_owned();
                            editeur.essayer();
                        }
                    }
                });
                ui.add_space(4.0);
                let hauteur = (ui.available_height() - 70.0).max(160.0);
                ScrollArea::vertical()
                    .id_salt("texte-programme")
                    .max_height(hauteur)
                    .show(ui, |ui| {
                        let reponse = ui.add(
                            TextEdit::multiline(&mut editeur.texte)
                                .code_editor()
                                .desired_width(f32::INFINITY)
                                .desired_rows(18),
                        );
                        if reponse.changed() {
                            editeur.essayer();
                        }
                    });
                ui.add_space(4.0);
                match &editeur.verdict {
                    Ok(n) => {
                        ui.label(
                            RichText::new(format!(
                                "{n} instruction{} sur {TAILLE_MAXIMALE} possibles : prêt à charger.",
                                if *n > 1 { "s" } else { "" }
                            ))
                            .color(c.un.c()),
                        );
                    }
                    Err(e) => {
                        ui.label(RichText::new(format!("À corriger, {e}")).color(danger));
                    }
                }
                ui.add_space(4.0);
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    let pret = matches!(editeur.verdict, Ok(n) if n > 0);
                    if ui
                        .add_enabled(pret, principal("Charger dans la mémoire"))
                        .on_hover_text(
                            "Refabrique la mémoire morte en NAND avec ce programme et redémarre l'ordinateur.",
                        )
                        .clicked()
                    {
                        charger = true;
                    }
                    ui.label(
                        RichText::new("Aide-mémoire : F1, « Programmer l'ordinateur ».")
                            .color(c.discret.c()),
                    );
                });
            });
        if charger {
            let (modele, texte) = (editeur.modele.clone(), editeur.texte.clone());
            match at.programmer(&modele, &texte) {
                Ok(c) => at.dire(&format!(
                    "Nouveau programme chargé : {} instructions, une mémoire morte de {} portes NAND. L'ordinateur redémarre.",
                    c.instructions, c.portes
                )),
                Err(e) => editeur.verdict = Err(e),
            }
        }
        if !ouverte {
            self.editeur = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaque_rubrique_a_du_texte_et_la_liste_des_instructions_est_complete() {
        for r in Rubrique::TOUTES {
            assert!(texte_de(r).len() > 100, "{r:?}");
        }
        let noms: Vec<&str> = INSTRUCTIONS
            .iter()
            .map(|(op, _)| op.split(' ').next().unwrap())
            .collect();
        for op in ce_circuit::ordinateur::Op::TOUTES {
            assert!(noms.contains(&op.nom()), "{}", op.nom());
        }
    }

    #[test]
    fn les_raccourcis_de_l_aide_existent() {
        // Ceux qui sont branchés dans app.rs.
        let t = texte_de(Rubrique::Raccourcis);
        for r in ["F1", "Ctrl+P", "Ctrl+G", "Ctrl+Z", "Espace", "Échap"] {
            assert!(t.contains(r), "{r}");
        }
    }
}
