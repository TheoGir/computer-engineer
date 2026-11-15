//! Les fenêtres posées par-dessus l'écran : l'aide (F1) et l'éditeur de programme de la
//! mémoire morte (Ctrl+P). Elles utilisent les widgets d'egui, aux couleurs de l'apparence.

use egui::{Align, Color32, Layout, RichText, ScrollArea, TextEdit};

use crate::apparence::Apparence;
use crate::dessin::Police;
use crate::etat::Atelier;
use crate::logique::{analyser, Analyse};
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
    /// Algèbre de Boole : lois, De Morgan, simplifier, Karnaugh.
    Algebre,
    /// Les mots de l'électronique numérique, de A à Z.
    Lexique,
    /// Où sont les fichiers, sauvegardes, mises à jour, rapports.
    Fichiers,
    /// Version, liens et licences.
    APropos,
}

impl Rubrique {
    /// Toutes, dans l'ordre des onglets.
    pub const TOUTES: [Rubrique; 7] = [
        Rubrique::PremiersPas,
        Rubrique::Raccourcis,
        Rubrique::Algebre,
        Rubrique::Lexique,
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
            Rubrique::Algebre => "Algèbre de Boole",
            Rubrique::Lexique => "Lexique",
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

/// L'analyse du plan affiché, refaite quand il change.
#[derive(Clone, Debug, PartialEq)]
pub struct VueAnalyse {
    /// La version de l'atelier et le lieu analysés.
    pub pour: (u64, Option<String>),
    /// Le nom du plan analysé.
    pub nom: String,
    /// Ce qu'on en sait, ou pourquoi il ne s'analyse pas.
    pub resultat: Result<Analyse, String>,
}

impl VueAnalyse {
    /// Analyse le plan affiché.
    pub fn de(at: &Atelier) -> VueAnalyse {
        VueAnalyse {
            pour: (at.version, at.lieu().map(str::to_owned)),
            nom: at
                .lieu()
                .map_or_else(|| at.projet.nom.clone(), str::to_owned),
            resultat: analyser(&at.projet, &at.doc),
        }
    }
}

/// Les fenêtres ouvertes.
#[derive(Debug, Default)]
pub struct Fenetres {
    /// L'aide, ouverte sur cette rubrique.
    pub aide: Option<Rubrique>,
    /// Ce qu'on cherche dans l'aide.
    pub recherche: String,
    /// L'éditeur de programme.
    pub editeur: Option<Editeur>,
    /// L'analyse du plan (Ctrl+E).
    pub analyse: Option<VueAnalyse>,
    /// Vrai quand « Exporter en Verilog » vient d'être demandé : l'application écrit le
    /// fichier, car elle seule connaît le dossier des données.
    pub exporter: bool,
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
        Rubrique::Algebre => ALGEBRE.to_owned(),
        Rubrique::Lexique => {
            let mut t = "# De A à Z\nLes mots qu'on croise en construisant un ordinateur. Tape un mot dans « Chercher » pour le trouver aussi dans le reste de l'aide.\n\n".to_owned();
            for (mot, sens) in LEXIQUE {
                t.push_str(&format!("- {mot} : {sens}\n"));
            }
            t
        }
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
- Ctrl+C, Ctrl+X, Ctrl+V : copier, couper, coller la sélection, avec ses fils et ses briques, dans ce projet ou dans un autre. Ce qui est copié peut aussi se coller sous forme de texte, pour l'envoyer à quelqu'un.
- Ctrl+E : analyser le circuit : sa table de vérité, l'expression simplifiée de chaque lampe, et l'export en Verilog.
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

const ALGEBRE: &str = "# Trois opérations
Toute la logique s'écrit avec trois opérations. On les lit ainsi dans l'analyse du circuit (Ctrl+E) et dans les indices des quêtes.
- NON : ¬a vaut 1 quand a vaut 0, et 0 quand a vaut 1.
- ET : a·b vaut 1 seulement quand a et b valent 1.
- OU : a + b vaut 1 dès que a ou b vaut 1.

# Les lois qui servent tout le temps
- Double négation : ¬¬a = a.
- Éléments neutres : a·1 = a et a + 0 = a.
- Éléments absorbants : a·0 = 0 et a + 1 = 1.
- Idempotence : a·a = a et a + a = a.
- Complément : a·¬a = 0 et a + ¬a = 1.
- Commutativité et associativité : dans un ET, ou dans un OU, l'ordre et les parenthèses ne changent rien.
- Distributivité : a·(b + c) = a·b + a·c, et aussi a + b·c = (a + b)·(a + c), qui n'a pas d'équivalent avec les nombres.
- Absorption : a + a·b = a et a·(a + b) = a.
- Fusion : a·b + a·¬b = a. C'est la règle des tables de Karnaugh.

# Les lois de De Morgan
- ¬(a·b) = ¬a + ¬b : une NAND est un OU de ses entrées inversées.
- ¬(a + b) = ¬a·¬b : une NOR est un ET de ses entrées inversées.
Pour s'en souvenir : on inverse chaque variable, et on échange ET et OU.

# Pourquoi la NAND suffit
- NON : ¬a = NAND(a, a), une porte.
- ET : a·b = NON(NAND(a, b)), deux portes.
- OU : a + b = NAND(¬a, ¬b) par De Morgan, trois portes.
Avec NON, ET et OU, on écrit n'importe quelle table de vérité (voir plus bas) : la NAND seule peut donc tout construire. On dit qu'elle est universelle ; la NOR l'est aussi.

# De la table de vérité au circuit
- Repère les lignes où la sortie vaut 1.
- Pour chacune, écris un produit (un ET) de toutes les entrées, avec ¬ devant celles qui valent 0 sur cette ligne : ce produit vaut 1 sur cette ligne, et seulement sur elle.
- La sortie est la somme (le OU) de ces produits. C'est la somme de produits développée : elle marche toujours, mais elle est souvent longue.
Exemple : le OU exclusif vaut 1 pour a = 1, b = 0 et pour a = 0, b = 1, donc s = a·¬b + ¬a·b.

# Simplifier avec une table de Karnaugh
Une table de Karnaugh range la table de vérité dans une grille où deux cases voisines ne diffèrent que d'une variable : les lignes et les colonnes suivent l'ordre 00, 01, 11, 10 (le code de Gray), et la grille se referme sur elle-même, d'un bord à l'autre. On entoure les 1 par groupes rectangulaires de 1, 2, 4, 8… cases (une puissance de deux), les plus grands possible. Chaque groupe donne un produit qui ne garde que les variables constantes dans le groupe, grâce à la fusion a·b + a·¬b = a. On couvre tous les 1 avec le moins de groupes possible.

# La méthode de Quine-McCluskey
C'est la même idée, menée par un programme : on fusionne les produits deux à deux tant qu'ils ne diffèrent que d'une variable, on garde ceux qu'on ne peut plus agrandir (les implicants premiers), puis on choisit le plus petit nombre d'entre eux qui couvre tous les 1 (méthode de Petrick). L'analyse du circuit (Ctrl+E) et le dernier indice des quêtes s'en servent : l'expression montrée a le moins de produits possible. Ce n'est pas forcément la construction qui demande le moins de NAND : factoriser, ou réutiliser une brique, fait souvent mieux.";

/// Le lexique, dans l'ordre alphabétique : le mot, puis ce qu'il veut dire.
pub const LEXIQUE: &[(&str, &str)] = &[
    ("Accumulateur", "le registre du processeur où se fait le calcul ; dans l'ordinateur miniature, il s'appelle A."),
    ("Additionneur", "circuit qui additionne deux nombres binaires, colonne par colonne, en passant la retenue à la suivante."),
    ("Adresse", "le numéro d'une case de mémoire."),
    ("Assembleur", "langage où chaque ligne est une instruction du processeur, écrite avec un nom court (LDI, ADD, JMP) ; c'est aussi le programme qui le traduit en nombres."),
    ("Bascule", "mémoire d'un bit qui ne change qu'au front de l'horloge. La bascule D recopie son entrée d à chaque front montant."),
    ("Binaire", "écriture des nombres avec deux chiffres, 0 et 1. 1101 vaut 8 + 4 + 0 + 1 = 13."),
    ("Bit", "un chiffre binaire, 0 ou 1. Un fil porte un bit."),
    ("Brique", "dans l'application, un circuit enregistré qu'on pose comme une seule pièce, et qu'on peut ouvrir d'un double-clic."),
    ("Bus", "un groupe de fils qui portent ensemble un nombre de plusieurs bits."),
    ("Chemin critique", "le plus long trajet, compté en portes, entre une entrée et une sortie : il fixe le temps de réponse du circuit."),
    ("Chronogramme", "le graphique de la valeur de chaque fil au fil du temps."),
    ("Circuit combinatoire", "circuit sans mémoire : ses sorties ne dépendent que des entrées du moment. Il a une table de vérité."),
    ("Circuit séquentiel", "circuit avec mémoire : ses sorties dépendent aussi de ce qui s'est passé avant."),
    ("Code de Gray", "façon de compter où deux nombres qui se suivent ne diffèrent que d'un bit : 00, 01, 11, 10."),
    ("Complément à deux", "façon d'écrire les nombres négatifs en binaire : on inverse tous les bits et on ajoute 1. Sur 8 bits, −1 s'écrit 11111111."),
    ("Compteur", "registre qui ajoute 1 à son nombre à chaque front d'horloge."),
    ("Compteur de programme", "le registre du processeur qui garde l'adresse de la prochaine instruction."),
    ("Conflit", "deux sorties reliées au même fil qui ne disent pas la même chose : le fil vaut X."),
    ("De Morgan (lois de)", "¬(a·b) = ¬a + ¬b et ¬(a + b) = ¬a·¬b. Voir la rubrique « Algèbre de Boole »."),
    ("Décodeur", "circuit à n entrées et 2ⁿ sorties qui allume seulement la sortie dont le numéro est écrit sur ses entrées."),
    ("Délai", "le temps que met une porte à répondre : ici, une unité de temps par NAND."),
    ("Démultiplexeur", "envoie son entrée vers la sortie choisie par un numéro."),
    ("Front montant", "l'instant où un signal passe de 0 à 1. Le front descendant : de 1 à 0."),
    ("Horloge", "un signal qui bascule régulièrement entre 0 et 1, et cadence tout le circuit."),
    ("Implicant premier", "un produit qui ne vaut 1 que là où la fonction vaut 1, et qu'on ne peut plus agrandir en retirant une variable."),
    ("Instruction", "un ordre élémentaire du processeur : charger un nombre, additionner, sauter ailleurs dans le programme…"),
    ("Interrupteur", "dans l'application, une entrée du circuit : on clique dessus pour la changer."),
    ("Lampe", "dans l'application, une sortie du circuit."),
    ("Mémoire morte (ROM)", "mémoire qu'on lit sans pouvoir l'écrire : elle garde le programme, même éteinte."),
    ("Mémoire vive (RAM)", "mémoire qu'on lit et qu'on écrit, case par case, à l'adresse choisie."),
    ("Multiplexeur", "choisit, d'après un numéro, laquelle de ses entrées recopier sur sa sortie."),
    ("NAND", "« non-ET » : sa sortie vaut 0 seulement quand ses deux entrées valent 1. C'est la seule porte de base de l'application."),
    ("Octet", "un groupe de 8 bits ; il va de 0 à 255."),
    ("Oscillation", "un circuit qui ne se stabilise jamais, comme une boucle d'un nombre impair de portes NON."),
    ("OU exclusif (XOR)", "vaut 1 quand ses deux entrées sont différentes."),
    ("Parité", "dire si le nombre de 1 est pair ou impair ; sert à repérer une erreur de transmission."),
    ("Porte logique", "petit circuit qui calcule une fonction de ses entrées : NON, ET, OU, OU exclusif…"),
    ("Processeur", "le circuit qui lit les instructions du programme une à une et les exécute."),
    ("Registre", "un groupe de bascules qui retiennent un nombre ensemble."),
    ("Retenue", "le bit qui passe à la colonne suivante dans une addition : en binaire, 1 + 1 = 10."),
    ("Somme de produits", "une expression écrite comme un OU de ET, par exemple a·b + ¬a·c."),
    ("Table de vérité", "la liste de toutes les combinaisons d'entrées, avec la valeur des sorties pour chacune."),
    ("Tampon trois états", "une sortie qui peut se débrancher (valeur Z), pour que plusieurs pièces se partagent un bus."),
    ("UAL", "l'unité arithmétique et logique : la partie du processeur qui calcule."),
    ("Universelle (porte)", "une porte avec laquelle on peut tout construire. La NAND l'est, la NOR aussi."),
    ("Verrou", "mémoire d'un bit qui suit son entrée tant qu'il est ouvert, et la garde une fois fermé."),
    ("X", "valeur inconnue, ou en conflit."),
    ("Z", "un fil que rien ne pilote (« haute impédance »)."),
];

/// Écrit un mot sans accents et en minuscules, pour chercher sans se soucier de l'écriture.
pub(crate) fn plier(t: &str) -> String {
    t.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c => c,
        })
        .collect()
}

/// Cherche `terme` dans toute l'aide. Rend un texte d'aide (même balisage que les rubriques) :
/// chaque ligne trouvée, sous « Rubrique › intertitre ». Vide si rien n'est trouvé.
pub fn chercher(terme: &str) -> String {
    let terme = plier(terme.trim());
    if terme.is_empty() {
        return String::new();
    }
    let mut sortie = String::new();
    for r in Rubrique::TOUTES {
        let texte = texte_de(r);
        let mut intertitre = String::new();
        let mut dernier_titre = String::new();
        // Un intertitre qui contient le mot : toute sa partie est rendue.
        let mut toute_la_partie = false;
        for ligne in texte.lines() {
            if let Some(t) = ligne.strip_prefix("# ") {
                intertitre = t.to_owned();
                toute_la_partie = plier(t).contains(&terme);
                continue;
            }
            if ligne.trim().is_empty() || !(toute_la_partie || plier(ligne).contains(&terme)) {
                continue;
            }
            let titre = if intertitre.is_empty() {
                r.titre().to_owned()
            } else {
                format!("{} › {intertitre}", r.titre())
            };
            if titre != dernier_titre {
                sortie.push_str(&format!("# {titre}\n"));
                dernier_titre = titre;
            }
            sortie.push_str(ligne);
            sortie.push('\n');
        }
    }
    sortie
}

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

    /// Ouvre ou ferme l'analyse du plan affiché.
    pub fn basculer_analyse(&mut self, at: &Atelier) {
        self.analyse = match self.analyse {
            Some(_) => None,
            None => Some(VueAnalyse::de(at)),
        };
    }

    /// Vrai si une fenêtre est ouverte.
    pub fn ouvertes(&self) -> bool {
        self.aide.is_some() || self.editeur.is_some() || self.analyse.is_some()
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
                                self.recherche.clear();
                            }
                        }
                    });
                    ui.add_space(4.0);
                    ui.add(
                        TextEdit::singleline(&mut self.recherche)
                            .hint_text("Chercher dans l'aide : un mot, un raccourci…")
                            .desired_width(f32::INFINITY),
                    );
                    ui.add_space(2.0);
                    ui.separator();
                    let recherche = self.recherche.trim().to_owned();
                    ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                        if !recherche.is_empty() {
                            let trouve = chercher(&recherche);
                            if trouve.is_empty() {
                                ui.label(
                                    RichText::new(format!(
                                        "Rien trouvé pour « {recherche} ». Essaie un mot plus court."
                                    ))
                                    .color(c.doux.c()),
                                );
                            } else {
                                ecrire(ui, &trouve, a);
                            }
                            return;
                        }
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
        self.fenetre_d_analyse(ctx, at, a, cadre);
        let Some(editeur) = &mut self.editeur else {
            return;
        };
        let mut ouverte = true;
        let mut charger = false;
        let danger = c.danger.c();
        // L'ordinateur en marche : ce que valent ses registres et sa mémoire vive, et
        // l'instruction qu'il exécute dans le programme chargé.
        let machine = crate::programme::regarder_la_machine(at);
        let charge: Vec<u16> = at
            .projet
            .modele(&editeur.modele)
            .and_then(|m| m.programme.as_deref())
            .and_then(|texte| ce_circuit::ordinateur::assembler(texte).ok())
            .unwrap_or_default();
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
                let place_machine = if machine.is_some() { 190.0 } else { 0.0 };
                let hauteur = (ui.available_height() - 70.0 - place_machine).max(160.0);
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
                if let Some(vue) = &machine {
                    ui.add_space(4.0);
                    ui.separator();
                    vue_machine(ui, vue, &charge, a);
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

impl Fenetres {
    /// La fenêtre « Analyser le circuit » (Ctrl+E) : expressions, table de vérité, export.
    fn fenetre_d_analyse(
        &mut self,
        ctx: &egui::Context,
        at: &Atelier,
        a: &Apparence,
        cadre: egui::Frame,
    ) {
        let Some(vue) = &mut self.analyse else {
            return;
        };
        if vue.pour != (at.version, at.lieu().map(str::to_owned)) {
            *vue = VueAnalyse::de(at);
        }
        let c = a.palette();
        let e = a.formes.echelle_texte;
        let mono = |t: &str, couleur: Color32| {
            RichText::new(t)
                .font(Police::Mono.taille(a.typo.corps * e))
                .color(couleur)
        };
        let mut ouverte = true;
        let mut exporter = false;
        egui::Window::new(format!("Analyse de « {} »", vue.nom))
            .id(egui::Id::unique("fenetre-analyse"))
            .open(&mut ouverte)
            .collapsible(false)
            .resizable(true)
            .default_size([560.0, 520.0])
            .frame(cadre)
            .show(ctx, |ui| {
                match &vue.resultat {
                    Err(raison) => {
                        ui.label(RichText::new(raison).color(c.doux.c()));
                    }
                    Ok(an) => {
                        ui.label(
                            RichText::new(format!(
                                "{} porte{} NAND · {} étape{} au plus entre une entrée et une sortie · {} combinaison{} d'entrées, toutes simulées.",
                                an.portes,
                                if an.portes > 1 { "s" } else { "" },
                                an.profondeur,
                                if an.profondeur > 1 { "s" } else { "" },
                                an.lignes.len(),
                                if an.lignes.len() > 1 { "s" } else { "" },
                            ))
                            .color(c.doux.c()),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("Expressions simplifiées")
                                .font(Police::CorpsDemi.taille(a.typo.fort * e))
                                .color(c.texte.c()),
                        );
                        for (nom, expr) in &an.expressions {
                            match expr {
                                Ok(x) => {
                                    ui.label(mono(&format!("{nom} = {x}"), c.texte.c()));
                                }
                                Err(pourquoi) => {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(mono(&format!("{nom} :"), c.texte.c()));
                                        ui.label(RichText::new(pourquoi).color(c.discret.c()));
                                    });
                                }
                            }
                        }
                        ui.label(
                            RichText::new("· se lit ET, + se lit OU, ¬ se lit NON. Chaque expression a le moins de produits possible (méthode de Quine-McCluskey) ; F1, « Algèbre de Boole », explique comment.")
                                .color(c.discret.c()),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("Table de vérité")
                                .font(Police::CorpsDemi.taille(a.typo.fort * e))
                                .color(c.texte.c()),
                        );
                        let hauteur = (ui.available_height() - 60.0).max(120.0);
                        ScrollArea::both()
                            .id_salt("table-analyse")
                            .max_height(hauteur)
                            .show(ui, |ui| {
                                egui::Grid::new("grille-analyse")
                                    .striped(true)
                                    .spacing([14.0, 2.0])
                                    .show(ui, |ui| {
                                        for (nom, l) in &an.entrees {
                                            ui.label(mono(&largeur(nom, *l), c.doux.c()));
                                        }
                                        ui.label(mono("→", c.discret.c()));
                                        for (nom, l) in &an.sorties {
                                            ui.label(mono(&largeur(nom, *l), c.accent_clair.c()));
                                        }
                                        ui.end_row();
                                        // Les entrées de chaque ligne, écrites colonne par
                                        // colonne ; les lignes comptent en binaire, la
                                        // première colonne en poids fort.
                                        let mut lignes: Vec<(Vec<String>, &Vec<ce_moteur::Valeur>)> = an
                                            .lignes
                                            .iter()
                                            .enumerate()
                                            .map(|(i, ligne)| {
                                                let mut bit = 0u32;
                                                let cases = an
                                                    .entrees
                                                    .iter()
                                                    .map(|(_, l)| {
                                                        let v: String = (0..*l)
                                                            .rev()
                                                            .map(|k| {
                                                                if Analyse::bit_d_entree(i, bit + k) {
                                                                    '1'
                                                                } else {
                                                                    '0'
                                                                }
                                                            })
                                                            .collect();
                                                        bit += l;
                                                        v
                                                    })
                                                    .collect();
                                                (cases, ligne)
                                            })
                                            .collect();
                                        lignes.sort_by(|x, y| x.0.cmp(&y.0));
                                        for (cases, ligne) in lignes {
                                            for v in &cases {
                                                ui.label(mono(v, c.texte.c()));
                                            }
                                            ui.label(mono("→", c.discret.c()));
                                            let mut k = 0usize;
                                            for (_, l) in &an.sorties {
                                                let morceau = &ligne[k..k + *l as usize];
                                                k += *l as usize;
                                                let v: String =
                                                    morceau.iter().rev().map(|v| v.to_string()).collect();
                                                let couleur = if morceau.iter().all(|v| *v == ce_moteur::Valeur::Zero) {
                                                    c.doux.c()
                                                } else {
                                                    c.un.c()
                                                };
                                                ui.label(mono(&v.to_uppercase(), couleur));
                                            }
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                }
                ui.add_space(8.0);
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button("Exporter en Verilog")
                        .on_hover_text("Écrit ce plan, briques mises à plat, dans un fichier .v : pour un autre simulateur ou un vrai circuit programmable (FPGA).")
                        .clicked()
                    {
                        exporter = true;
                    }
                    ui.label(
                        RichText::new("Les bus se lisent du bit de poids fort au bit 0.")
                            .color(c.discret.c()),
                    );
                });
            });
        if exporter {
            self.exporter = true;
        }
        if !ouverte {
            self.analyse = None;
        }
    }
}

/// `nom`, ou `nom[4]` pour un bus de 4 bits.
/// Un octet lu sur la machine : en décimal et en hexadécimal, « ?? » s'il n'est pas déterminé.
fn octet(v: Option<u8>) -> String {
    v.map_or("??".to_owned(), |v| format!("{v} (0x{v:02X})"))
}

/// L'ordinateur en marche, sous l'éditeur de programme : l'instruction en cours surlignée
/// dans le programme chargé, l'accumulateur et la mémoire vive case par case.
fn vue_machine(ui: &mut egui::Ui, vue: &crate::programme::VueMachine, code: &[u16], a: &Apparence) {
    let c = a.palette();
    let mono = |t: String, couleur: Color32| RichText::new(t).monospace().color(couleur);
    ui.label(
        RichText::new("La machine en marche")
            .font(Police::CorpsDemi.taille(a.typo.corps * a.formes.echelle_texte))
            .color(c.texte.c()),
    );
    ui.columns(2, |colonnes| {
        let ui = &mut colonnes[0];
        ui.label(mono(
            format!("Accumulateur A : {}", octet(vue.a)),
            c.texte.c(),
        ));
        // Le programme chargé, l'instruction en cours surlignée et gardée en vue.
        ScrollArea::vertical()
            .id_salt("instructions")
            .max_height(130.0)
            .show(ui, |ui| {
                for (i, mot) in code.iter().enumerate() {
                    let (op, n) = ce_circuit::ordinateur::decoder(*mot);
                    let texte = if op.a_un_operande() {
                        format!("{i:3}  {} {n}", op.nom())
                    } else {
                        format!("{i:3}  {}", op.nom())
                    };
                    let en_cours = vue.pc == Some(i as u8);
                    let r = ui.label(if en_cours {
                        mono(format!("{texte}   ◀ en cours"), c.accent_clair.c())
                            .background_color(c.enfonce.c())
                    } else {
                        mono(texte, c.doux.c())
                    });
                    if en_cours {
                        r.scroll_to_me(None);
                    }
                }
                if code.is_empty() {
                    ui.label(mono("aucun programme chargé".to_owned(), c.discret.c()));
                }
            });
        let ui = &mut colonnes[1];
        ui.label(mono(
            format!("Mémoire vive ({} octets)", vue.ram.len()),
            c.texte.c(),
        ));
        egui::Grid::new("memoire-vive")
            .spacing([10.0, 2.0])
            .show(ui, |ui| {
                for (k, case) in vue.ram.iter().enumerate() {
                    let texte = case.map_or("··".to_owned(), |v| format!("{v:02X}"));
                    ui.label(mono(format!("{k:X}"), c.discret.c()));
                    ui.label(mono(
                        texte,
                        if case.is_some() {
                            c.un.c()
                        } else {
                            c.discret.c()
                        },
                    ));
                    if k % 4 == 3 {
                        ui.end_row();
                    }
                }
            });
    });
}

fn largeur(nom: &str, l: u32) -> String {
    if l > 1 {
        format!("{nom}[{l}]")
    } else {
        nom.to_owned()
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
        for r in [
            "F1", "Ctrl+P", "Ctrl+G", "Ctrl+Z", "Espace", "Échap", "Ctrl+C", "Ctrl+V", "Ctrl+E",
        ] {
            assert!(t.contains(r), "{r}");
        }
    }

    #[test]
    fn la_recherche_trouve_dans_toutes_les_rubriques_sans_se_soucier_des_accents() {
        let t = chercher("morgan");
        assert!(
            t.contains("# Algèbre de Boole › Les lois de De Morgan"),
            "{t}"
        );
        assert!(t.contains("# Lexique"), "{t}");
        // Sans accent ni majuscule, « Échap » se trouve quand même.
        assert!(chercher("echap").contains("Échap"));
        assert!(chercher("zzzz introuvable").is_empty());
        assert!(chercher("  ").is_empty());
    }

    #[test]
    fn le_lexique_est_dans_l_ordre_alphabetique_et_sans_doublon() {
        let cles: Vec<String> = LEXIQUE.iter().map(|(m, _)| plier(m)).collect();
        let mut triees = cles.clone();
        triees.sort();
        triees.dedup();
        assert_eq!(cles, triees);
        assert!(LEXIQUE.len() >= 40);
    }

    /// Les lois de la rubrique « Algèbre de Boole », démontrées par exhaustion : chaque
    /// égalité est vraie pour toutes les valeurs de a, b et c.
    #[test]
    fn les_lois_de_l_aide_sont_justes() {
        let nand = |x: bool, y: bool| !(x && y);
        for i in 0..8u8 {
            let (a, b, c) = (i & 1 == 1, i & 2 == 2, i & 4 == 4);
            let lois = [
                ("double négation", !!a, a),
                ("neutre ET", a && true, a),
                ("neutre OU", a || false, a),
                ("absorbant ET", a && false, false),
                ("absorbant OU", a || true, true),
                ("idempotence ET", a && a, a),
                ("idempotence OU", a || a, a),
                ("complément ET", a && !a, false),
                ("complément OU", a || !a, true),
                ("distributivité 1", a && (b || c), (a && b) || (a && c)),
                ("distributivité 2", a || (b && c), (a || b) && (a || c)),
                ("absorption 1", a || (a && b), a),
                ("absorption 2", a && (a || b), a),
                ("fusion", (a && b) || (a && !b), a),
                ("De Morgan 1", !(a && b), !a || !b),
                ("De Morgan 2", !(a || b), !a && !b),
                ("NON en NAND", !a, nand(a, a)),
                ("ET en NAND", a && b, nand(nand(a, b), nand(a, b))),
                ("OU en NAND", a || b, nand(nand(a, a), nand(b, b))),
                ("OU exclusif", a ^ b, (a && !b) || (!a && b)),
            ];
            for (nom, gauche, droite) in lois {
                assert_eq!(gauche, droite, "{nom} avec a={a} b={b} c={c}");
            }
        }
        let t = texte_de(Rubrique::Algebre);
        for loi in [
            "¬(a·b) = ¬a + ¬b",
            "¬(a + b) = ¬a·¬b",
            "a + b·c = (a + b)·(a + c)",
            "a·b + a·¬b = a",
            "s = a·¬b + ¬a·b",
        ] {
            assert!(t.contains(loi), "{loi}");
        }
    }
}
