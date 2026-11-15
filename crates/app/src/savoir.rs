//! Ce qu'on apprend en chemin : une petite fiche par composant du parcours (à quoi il sert,
//! comment il marche, sa place dans l'ordinateur) et la présentation de chaque chapitre.
//!
//! Les textes sont ici, à part des fiches de [`crate::quete::PARCOURS`], pour qu'on puisse les
//! relire et les reprendre d'un seul coup d'œil. Un test vérifie que chaque quête a sa fiche
//! et chaque chapitre sa présentation.

use crate::quete::PARCOURS;

/// Une branche du parcours : le tronc mène à l'ordinateur, les autres branches à un grand
/// élément de plus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Branche {
    /// Le chemin principal, de la porte NON à l'ordinateur complet.
    Tronc,
    /// La branche de la carte réseau : faire parler deux machines.
    Reseau,
    /// La branche de la montre numérique : compter le temps.
    Montre,
    /// La branche du contrôleur d'écran : faire une image.
    Ecran,
    /// La branche des maths et des automates.
    Maths,
    /// La branche des codes secrets et des codes correcteurs.
    Codes,
    /// La branche des jeux.
    Jeux,
    /// Les grands projets, plus grands que l'ordinateur : plusieurs machines ensemble.
    Grands,
    /// Le dépannage : des circuits tout faits, avec un fil mal branché à trouver.
    Depannage,
}

impl Branche {
    /// Le nom affiché de la branche.
    pub fn nom(self) -> &'static str {
        match self {
            Branche::Tronc => "Vers l'ordinateur",
            Branche::Reseau => "Branche : la carte réseau",
            Branche::Montre => "Branche : la montre numérique",
            Branche::Ecran => "Branche : le contrôleur d'écran",
            Branche::Maths => "Branche : maths et automates",
            Branche::Codes => "Branche : codes et secrets",
            Branche::Jeux => "Branche : les jeux",
            Branche::Grands => "Grands projets",
            Branche::Depannage => "Branche : le dépannage",
        }
    }

    /// Le grand élément au bout de la branche.
    pub fn element(self) -> &'static str {
        match self {
            Branche::Tronc => "l'ordinateur",
            Branche::Reseau => "la carte réseau",
            Branche::Montre => "la montre numérique",
            Branche::Ecran => "le contrôleur d'écran",
            Branche::Maths => "maths et automates",
            Branche::Codes => "le correcteur d'erreurs",
            Branche::Jeux => "les jeux",
            Branche::Grands => "les grands projets",
            Branche::Depannage => "le dépannage",
        }
    }

    /// Une phrase sur ce qu'on y construit.
    pub fn but(self) -> &'static str {
        match self {
            Branche::Tronc => "De la porte NON au processeur, puis à l'ordinateur complet.",
            Branche::Reseau => {
                "Apprendre à ton ordinateur à envoyer et recevoir des octets sur un seul fil."
            }
            Branche::Montre => "Des compteurs en cascade jusqu'à une montre qui se règle et un réveil.",
            Branche::Ecran => "Balayer l'écran, dessiner des chiffres, garder l'image : le cœur d'une carte graphique.",
            Branche::Maths => "Des nombres premiers, Fibonacci, des automates qui lisent des bits, le jeu de la vie, et l'algorithme d'Euclide en circuit.",
            Branche::Codes => "Cacher un message, vérifier qu'il est intact, puis réparer tout seul un bit abîmé.",
            Branche::Jeux => "Un arbitre, un juge de morpion, un dé et un tableau des records : la logique s'amuse.",
            Branche::Grands => "Plusieurs ordinateurs qui travaillent ensemble, et un ordinateur seize fois plus grand.",
            Branche::Depannage => "Des circuits déjà construits, mais avec un fil mal branché : trouver la panne et la réparer.",
        }
    }
}

/// La présentation d'un chapitre.
#[derive(Clone, Copy, Debug)]
pub struct Chapitre {
    /// Le nom, tel qu'il est écrit dans les fiches du parcours.
    pub nom: &'static str,
    /// La branche du parcours où il se trouve.
    pub branche: Branche,
    /// Ce qu'on y apprend, en une ou deux phrases.
    pub apprend: &'static str,
}

/// Les chapitres, dans l'ordre du parcours.
pub const CHAPITRES: &[Chapitre] = &[
    Chapitre {
        nom: "Portes logiques",
        branche: Branche::Tronc,
        apprend: "Avec une seule porte, la NAND, on fabrique toutes les autres : NON, ET, OU, OU exclusif. Tout ce qui suit est fait de ces portes.",
    },
    Chapitre {
        nom: "Aiguillages et codes",
        branche: Branche::Tronc,
        apprend: "Choisir un signal parmi plusieurs, l'envoyer au bon endroit, traduire un nombre en code : c'est ainsi qu'une adresse trouve sa case de mémoire.",
    },
    Chapitre {
        nom: "Calcul",
        branche: Branche::Tronc,
        apprend: "Additionner bit par bit avec des retenues, puis soustraire, comparer, multiplier : à la fin, l'unité de calcul du processeur.",
    },
    Chapitre {
        nom: "Mémoire",
        branche: Branche::Tronc,
        apprend: "Deux portes qui se tiennent l'une l'autre gardent un bit. De là viennent les bascules, les registres et la mémoire vive.",
    },
    Chapitre {
        nom: "Compter et cadencer",
        branche: Branche::Tronc,
        apprend: "Une mémoire plus un calcul, rythmés par l'horloge, donnent des compteurs et des automates : le cœur battant de toute machine.",
    },
    Chapitre {
        nom: "Communiquer",
        branche: Branche::Tronc,
        apprend: "Envoyer un octet sur un seul fil, et laisser les appareils réclamer l'attention du processeur.",
    },
    Chapitre {
        nom: "Le processeur",
        branche: Branche::Tronc,
        apprend: "Un compteur qui pointe l'instruction, un décodeur qui la comprend, une unité de calcul qui l'exécute : le processeur.",
    },
    Chapitre {
        nom: "Ordinateurs",
        branche: Branche::Tronc,
        apprend: "Processeur, mémoires, horloge, clavier et écran branchés ensemble exécutent un vrai programme.",
    },
    Chapitre {
        nom: "Le réseau",
        branche: Branche::Reseau,
        apprend: "Recevoir des trames, vérifier qu'elles sont intactes, les garder en attente : jusqu'à une carte réseau qui fait parler deux machines.",
    },
    Chapitre {
        nom: "La montre numérique",
        branche: Branche::Montre,
        apprend: "Des compteurs qui se passent la main, comme les rouages d'une horloge : secondes, minutes, heures, puis un réveil.",
    },
    Chapitre {
        nom: "Le contrôleur d'écran",
        branche: Branche::Ecran,
        apprend: "Un écran se dessine pixel par pixel : un balayage, une mémoire d'image, une police de caractères, et un contrôleur qui relie tout.",
    },
    Chapitre {
        nom: "Nombres remarquables",
        branche: Branche::Maths,
        apprend: "Les nombres ont des personnalités : premiers, carrés, écrits en décimal, ou enchaînés comme la suite de Fibonacci. Des circuits les reconnaissent et les fabriquent.",
    },
    Chapitre {
        nom: "Maths et automates",
        branche: Branche::Maths,
        apprend: "Un automate est une petite mémoire et une règle. On en fait des reconnaisseurs, des tests de divisibilité, le jeu de la vie et l'algorithme d'Euclide.",
    },
    Chapitre {
        nom: "Codes et secrets",
        branche: Branche::Codes,
        apprend: "Le OU exclusif est l'outil des secrets : il cache un message, il vérifie qu'il est intact, et avec Hamming il répare même une erreur sans rien demander.",
    },
    Chapitre {
        nom: "Jeux",
        branche: Branche::Jeux,
        apprend: "Des règles de jeu deviennent des portes : qui a gagné, trois croix alignées, un dé qui roule, un record à battre.",
    },
    Chapitre {
        nom: "La grande mémoire",
        branche: Branche::Grands,
        apprend: "Une grande mémoire n'est que des petites mémoires et un aiguillage, répétés. On monte à 256 octets, et l'ordinateur grandit avec.",
    },
    Chapitre {
        nom: "Ordinateurs en réseau",
        branche: Branche::Grands,
        apprend: "Deux ordinateurs entiers, reliés par une boîte aux lettres, s'échangent des messages, puis des secrets : c'est le début d'Internet.",
    },
    Chapitre {
        nom: "Dépannage",
        branche: Branche::Depannage,
        apprend: "Lire un circuit qu'on n'a pas dessiné, voir quand il se trompe, suivre le courant jusqu'à la porte fautive et la rebrancher : le métier de dépanneur.",
    },
];

/// La présentation du chapitre qui porte ce nom.
pub fn chapitre(nom: &str) -> Option<&'static Chapitre> {
    CHAPITRES.iter().find(|c| c.nom == nom)
}

/// Les rangs, dans [`PARCOURS`], des quêtes de ce chapitre.
pub fn quetes_du_chapitre(nom: &str) -> Vec<usize> {
    PARCOURS
        .iter()
        .enumerate()
        .filter(|(_, f)| f.chapitre == nom)
        .map(|(i, _)| i)
        .collect()
}

/// Ce qu'annonce la grande fête quand ce chapitre vient d'être terminé : un titre et une
/// phrase.
pub fn fin_de_chapitre(nom: &str) -> (String, String) {
    let Some(i) = CHAPITRES.iter().position(|c| c.nom == nom) else {
        return (format!("Chapitre terminé : {nom}"), String::new());
    };
    let ici = &CHAPITRES[i];
    let suivant = CHAPITRES.get(i + 1);
    let dernier_de_la_branche = suivant.is_none_or(|s| s.branche != ici.branche);
    match (ici.branche, dernier_de_la_branche) {
        (Branche::Tronc, true) => (
            "Parcours principal terminé !".to_owned(),
            "Tu as construit un ordinateur entier. Les branches t'attendent dans le parcours."
                .to_owned(),
        ),
        (b, true) => (
            format!("Branche terminée : {} !", b.element()),
            format!("Grand élément construit. {}", b.but()),
        ),
        (_, false) => (
            format!("Chapitre terminé : {nom}"),
            suivant
                .map(|s| format!("Prochain chapitre : {}", s.nom))
                .unwrap_or_default(),
        ),
    }
}

/// La petite fiche d'un composant du parcours.
#[derive(Clone, Copy, Debug)]
pub struct Savoir {
    /// La clé de la quête.
    pub cle: &'static str,
    /// À quoi il sert.
    pub sert: &'static str,
    /// Comment il marche.
    pub marche: &'static str,
    /// Sa place dans l'ordinateur.
    pub place: &'static str,
    /// Où on le croise dans la vraie vie.
    pub vraie_vie: &'static str,
    /// Une anecdote, un fait d'histoire ou de science : « Le savais-tu ? ».
    pub anecdote: &'static str,
    /// Une petite expérience à faire dans l'atelier : « À essayer ».
    pub essaie: &'static str,
}

impl Savoir {
    /// Les rubriques de la fiche, dans l'ordre où on les lit : un titre et son texte.
    pub fn rubriques(&self) -> [(&'static str, &'static str); 6] {
        [
            ("À quoi ça sert", self.sert),
            ("Comment ça marche", self.marche),
            ("Sa place dans le tout", self.place),
            ("Dans la vraie vie", self.vraie_vie),
            ("Le savais-tu ?", self.anecdote),
            ("À essayer", self.essaie),
        ]
    }
}

/// La fiche de la quête qui porte cette clé.
pub fn savoir(cle: &str) -> Option<&'static Savoir> {
    SAVOIRS.iter().find(|s| s.cle == cle)
}

/// Une fiche par quête du parcours.
pub const SAVOIRS: &[Savoir] = &[
    // Portes logiques.
    Savoir {
        cle: "non",
        sert: "Inverser un signal : 1 devient 0, 0 devient 1.",
        marche: "Une NAND ne donne 0 que si ses deux entrées valent 1. Avec a sur les deux entrées, elle donne donc l'inverse de a.",
        place: "Partout : chaque fois qu'il faut le contraire d'un signal, dans les autres portes, les bascules, l'unité de calcul.",
        vraie_vie: "Un inverseur se cache dans chaque puce : c'est la porte la plus simple et la plus répandue qui soit.",
        anecdote: "En 1913, Henry Sheffer a montré qu'une seule opération, la NAND, suffit à écrire toute la logique. C'est le pari de ce jeu.",
        essaie: "Branche deux NON à la suite : que vaut la sortie ? Et avec trois ?",
    },
    Savoir {
        cle: "et",
        sert: "Dire « oui » seulement quand toutes les conditions sont vraies.",
        marche: "NAND veut dire NON-ET : une NAND suivie d'un NON redonne le ET.",
        place: "Il laisse passer un signal seulement quand une commande le permet : écrire en mémoire si « ecrire » vaut 1, par exemple.",
        vraie_vie: "Un ascenseur ne part que si la porte est fermée ET qu'un étage est demandé : c'est une porte ET.",
        anecdote: "En 1854, George Boole décrivait le raisonnement avec ET, OU et NON. Un siècle plus tard, ses calculs sont devenus des circuits.",
        essaie: "Ajoute une troisième entrée : avec deux portes ET, s ne vaut 1 que si les trois valent 1.",
    },
    Savoir {
        cle: "ou",
        sert: "Dire « oui » dès qu'au moins une condition est vraie.",
        marche: "Loi de De Morgan : a OU b = NON(NON a ET NON b). On inverse les deux entrées, puis une NAND fait le reste.",
        place: "Il réunit des raisons d'agir : une remise à zéro demandée par le bouton OU par le compteur.",
        vraie_vie: "Le plafonnier qui s'allume si l'une des portes de la voiture est ouverte : une porte OU.",
        anecdote: "Claude Shannon a montré en 1937, dans son mémoire de master, que l'algèbre de Boole décrit parfaitement les circuits à relais.",
        essaie: "Compte les NAND de ta porte OU. Peux-tu faire mieux que trois ?",
    },
    Savoir {
        cle: "non-ou",
        sert: "Dire « oui » seulement quand rien n'est vrai.",
        marche: "Un OU suivi d'un NON. Comme la NAND, la NON-OU suffit à elle seule à tout construire.",
        place: "Elle repère un nombre nul : tous les bits à 0. C'est le drapeau Z du processeur.",
        vraie_vie: "Une alarme qui reste muette tant qu'aucun capteur ne se déclenche suit un NON-OU.",
        anecdote: "Le NON-OU est universel lui aussi : l'ordinateur de bord d'Apollo était fait d'environ 5 600 portes NON-OU à trois entrées, toutes identiques.",
        essaie: "Fabrique une porte NON avec un seul NON-OU, comme tu l'as fait avec la NAND.",
    },
    Savoir {
        cle: "ou-exclusif",
        sert: "Dire si deux bits sont différents.",
        marche: "Une NAND de a et b, puis deux NAND qui la combinent avec a et avec b, puis une dernière NAND : quatre portes en tout.",
        place: "C'est la somme d'un bit sans la retenue : le cœur de l'additionneur, et de la parité.",
        vraie_vie: "L'interrupteur à deux commandes d'un escalier : changer l'un ou l'autre inverse la lumière. C'est un OU exclusif.",
        anecdote: "Le OU exclusif est au cœur de la cryptographie : appliquer deux fois la même clé avec un OU exclusif redonne le message.",
        essaie: "Relie a à ses deux entrées : que vaut s ? Et si b vaut toujours 1 ?",
    },
    Savoir {
        cle: "equivalence",
        sert: "Dire si deux bits sont égaux.",
        marche: "L'inverse du OU exclusif.",
        place: "Bit à bit, elle compare deux nombres : le test d'égalité, et le contrôle de parité de la carte réseau.",
        vraie_vie: "Un test d'égalité de deux bits : la base de toute comparaison de mots de passe ou de nombres.",
        anecdote: "L'équivalence est l'inverse du OU exclusif : elle vaut 1 quand les bits sont pareils.",
        essaie: "Mets quatre équivalences côte à côte et un ET au bout : tu tiens un comparateur de 4 bits.",
    },
    Savoir {
        cle: "majorite",
        sert: "Voter : la sortie suit ce que disent au moins deux entrées sur trois.",
        marche: "Une NAND par paire d'entrées repère chaque paire de 1, et une NAND à trois entrées (ou un arbre) les réunit.",
        place: "C'est exactement la retenue d'un additionneur complet : il y a retenue quand au moins deux des trois bits valent 1.",
        vraie_vie: "Les avions et les fusées doublent ou triplent leurs calculateurs : un vote à la majorité décide quand l'un se trompe.",
        anecdote: "Le vote à trois s'appelle la redondance modulaire triple. Elle protège des ordinateurs de vols spatiaux contre les rayons cosmiques.",
        essaie: "Passe une entrée à 1 et une à 0 : la troisième décide seule. Vérifie-le sur la table.",
    },
    // Aiguillages et codes.
    Savoir {
        cle: "multiplexeur",
        sert: "Choisir lequel de deux signaux passe, selon une commande sel.",
        marche: "Une NAND laisse passer a quand sel vaut 0, une autre laisse passer b quand sel vaut 1, une troisième réunit les deux.",
        place: "L'aiguillage universel : il choisit ce qu'un registre garde, ce que l'unité de calcul reçoit, ce qui part vers la mémoire.",
        vraie_vie: "Le sélecteur d'entrée d'un téléviseur : un seul signal passe, celui qu'on choisit.",
        anecdote: "Avec assez de multiplexeurs, on peut fabriquer n'importe quelle fonction logique : les FPGA, des puces reprogrammables, en sont pleins.",
        essaie: "Garde sel à 0 et change b : la sortie bouge-t-elle ?",
    },
    Savoir {
        cle: "demultiplexeur",
        sert: "Envoyer un signal vers une sortie parmi deux.",
        marche: "Deux ET : l'un avec sel, l'autre avec NON sel. Seule la sortie choisie reçoit e.",
        place: "Il distribue un ordre : « écris » ne doit arriver qu'à la case de mémoire choisie.",
        vraie_vie: "Un aiguillage de chemin de fer : un seul train, plusieurs voies possibles.",
        anecdote: "Le démultiplexeur et le multiplexeur sont les deux moitiés d'une ligne partagée : plusieurs conversations sur un seul fil, à tour de rôle.",
        essaie: "Mets e à 1 et change sel : la lumière saute d'une sortie à l'autre.",
    },
    Savoir {
        cle: "multiplexeur-4",
        sert: "Choisir un signal parmi quatre avec un numéro de 2 bits.",
        marche: "Un arbre de multiplexeurs : sel[0] choisit dans chaque paire, sel[1] entre les deux gagnants.",
        place: "En plus large, c'est la lecture de la mémoire : l'adresse choisit la case à montrer.",
        vraie_vie: "Les quatre pistes d'une table de mixage, une seule écoutée à la fois.",
        anecdote: "Un multiplexeur à 4 voies est un arbre de trois multiplexeurs à 2 voies : chaque bit de sel tranche à son étage.",
        essaie: "Change sel de 0 à 3 en comptant en binaire : quelle entrée passe à chaque fois ?",
    },
    Savoir {
        cle: "decodeur-2",
        sert: "Transformer un numéro de 2 bits en une seule ligne allumée parmi quatre.",
        marche: "Chaque sortie est un ET des bits du numéro, inversés ou non : s[2] = a[1] ET NON a[0].",
        place: "C'est lui qui transforme une adresse en « cette case-là » dans la mémoire vive et la mémoire morte.",
        vraie_vie: "La sonnette d'un immeuble : le numéro de l'appartement réveille une seule sonnerie.",
        anecdote: "Les décodeurs choisissent la case de mémoire à lire. Ta mémoire de 16 octets en utilise un.",
        essaie: "Compte de 0 à 3 avec a : la lumière avance d'une sortie à chaque fois.",
    },
    Savoir {
        cle: "decodeur-3",
        sert: "La même chose pour un numéro de 3 bits : une ligne parmi huit.",
        marche: "Un décodeur 2 vers 4 pour le bas du numéro, puis a[2] choisit la moitié haute ou basse.",
        place: "On agrandit ainsi les décodeurs pour adresser de plus grandes mémoires.",
        vraie_vie: "Les huit étages d'un ascenseur, un seul bouton allumé.",
        anecdote: "Un décodeur de n bits a 2^n sorties : avec 20 bits, on désigne plus d'un million de cases.",
        essaie: "Regarde a[2] : il partage les sorties en deux moitiés. Pourquoi ?",
    },
    Savoir {
        cle: "encodeur",
        sert: "L'inverse du décodeur : donner le numéro de la ligne allumée la plus importante.",
        marche: "Chaque bit du numéro réunit les lignes qui le contiennent, en donnant la priorité au plus grand rang.",
        place: "Il choisit l'appareil le plus urgent dans le contrôleur d'interruptions.",
        vraie_vie: "Un clavier traduit la touche enfoncée en son numéro : c'est un encodeur.",
        anecdote: "L'encodeur de priorité choisit le plus important quand plusieurs demandes arrivent : on le retrouve dans les contrôleurs d'interruptions.",
        essaie: "Allume deux bits de d à la fois : lequel gagne ?",
    },
    Savoir {
        cle: "sept-segments",
        sert: "Afficher un chiffre sur un afficheur à sept bâtons lumineux.",
        marche: "Sept petites fonctions logiques du même nombre n : une par segment, qui dit s'il s'allume pour ce chiffre.",
        place: "Les montres, les calculatrices et les fours affichent ainsi leurs chiffres.",
        vraie_vie: "Les chiffres des réveils, des fours et des pompes à essence : sept barreaux suffisent.",
        anecdote: "L'afficheur à sept segments date du début du XXe siècle, bien avant l'électronique, pour des panneaux à ampoules.",
        essaie: "Affiche 8 : tous les segments s'allument. Quel chiffre en allume le moins ?",
    },
    Savoir {
        cle: "parite",
        sert: "Dire si un octet contient un nombre impair de 1.",
        marche: "Le OU exclusif de tous les bits. En arbre, les huit bits sont réunis en trois étages au lieu de sept.",
        place: "Ajouté à un octet envoyé, ce bit permet au destinataire de repérer une erreur de transmission.",
        vraie_vie: "La mémoire des premiers PC ajoutait un bit de parité à chaque octet, pour repérer une erreur.",
        anecdote: "La parité détecte une erreur, mais pas deux : deux bits retournés se compensent. Le code de Hamming va plus loin.",
        essaie: "Change un seul bit de a : p s'inverse à chaque fois, quel que soit le bit.",
    },
    // Calcul.
    Savoir {
        cle: "demi-additionneur",
        sert: "Additionner deux bits : 1 + 1 = 10 en binaire.",
        marche: "La somme est le OU exclusif de a et b, la retenue leur ET.",
        place: "La toute première brique du calcul : l'incrémenteur et les compteurs en sont faits.",
        vraie_vie: "Chaque calculatrice commence par lui : deux bits entrent, une somme et une retenue sortent.",
        anecdote: "1 + 1 = 10 en binaire : la somme vaut 0 et la retenue 1. C'est exactement ta table.",
        essaie: "Mets a et b à 1 : quelle lampe s'allume ? C'est la « retenue » qu'on pose au-dessus, en calcul écrit.",
    },
    Savoir {
        cle: "additionneur-complet",
        sert: "Additionner deux bits plus la retenue qui vient de la colonne d'à côté.",
        marche: "Deux demi-additionneurs à la suite ; il y a retenue si l'un ou l'autre en produit une.",
        place: "Une colonne de l'addition posée : il en faut un par bit dans l'additionneur.",
        vraie_vie: "Un processeur 64 bits en aligne 64, un par bit, dans son unité de calcul.",
        anecdote: "En 1937, George Stibitz a monté sur sa table de cuisine un additionneur à relais : le « modèle K », pour Kitchen.",
        essaie: "Trouve le cas où les trois entrées valent 1 : que valent s et rs ?",
    },
    Savoir {
        cle: "additionneur-4-bits",
        sert: "Additionner deux nombres de 4 bits.",
        marche: "Quatre additionneurs complets en chaîne : la retenue de chaque colonne entre dans la suivante, comme à la main.",
        place: "C'est l'addition de l'unité de calcul, en petit.",
        vraie_vie: "Le premier microprocesseur vendu, l'Intel 4004 de 1971, calculait justement sur 4 bits.",
        anecdote: "On l'appelle additionneur à propagation de retenue : la retenue traverse les étages un par un, comme une rangée de dominos.",
        essaie: "Calcule 15 + 1 : s revient à 0 et rs s'allume. C'est un débordement.",
    },
    Savoir {
        cle: "additionneur-8-bits",
        sert: "Additionner deux octets.",
        marche: "Huit additionneurs complets en chaîne, ou deux additionneurs de 4 bits reliés par la retenue.",
        place: "Les nombres de l'ordinateur miniature font 8 bits : c'est son additionneur.",
        vraie_vie: "Un octet, 8 bits, va de 0 à 255 : la taille d'une couleur primaire d'un pixel d'écran.",
        anecdote: "Les consoles 8 bits des années 1980 calculaient sur des octets, avec un additionneur comme le tien.",
        essaie: "Additionne 200 et 100 : que vaut s ? 300 − 256 = 44, et la retenue est allumée.",
    },
    Savoir {
        cle: "additionneur-rapide",
        sert: "Additionner deux octets sans attendre que la retenue traverse les huit étages.",
        marche: "La moitié haute est calculée deux fois à l'avance, avec et sans retenue ; quand la vraie retenue arrive, un multiplexeur choisit.",
        place: "Les vrais processeurs vont plus vite ainsi : la vitesse d'un circuit dépend de son chemin le plus long.",
        vraie_vie: "Les processeurs modernes calculent la retenue d'avance, sinon une addition de 64 bits serait bien trop lente.",
        anecdote: "Il existe des additionneurs encore plus rapides, dits à anticipation de retenue, dont le temps ne grandit qu'avec le logarithme du nombre de bits.",
        essaie: "Compare la profondeur (le nombre d'étages traversés) de tes deux additionneurs de 8 bits.",
    },
    Savoir {
        cle: "egalite",
        sert: "Dire si deux nombres sont égaux.",
        marche: "Une équivalence par bit, puis un ET de toutes : un seul bit différent suffit à dire non.",
        place: "Il sert aux sauts conditionnels et à reconnaître une adresse.",
        vraie_vie: "Un digicode compare le code tapé au bon code, chiffre par chiffre.",
        anecdote: "Comparer deux nombres de n bits demande n équivalences et un ET : le temps ne dépend presque pas de la taille.",
        essaie: "Rends a et b égaux, puis change un seul bit : egal s'éteint aussitôt.",
    },
    Savoir {
        cle: "comparateur",
        sert: "Dire si un nombre est plus petit, égal ou plus grand qu'un autre.",
        marche: "On calcule a − b : il y a un emprunt exactement quand a < b, et le résultat est nul quand a = b.",
        place: "Le processeur compare ainsi, avec ses drapeaux Z (zéro) et C (retenue).",
        vraie_vie: "Un thermostat compare la température voulue à celle mesurée, et décide de chauffer.",
        anecdote: "Ton comparateur soustrait en secret : a − b sans emprunt veut dire a ≥ b.",
        essaie: "Essaie a = b : seule egal s'allume. Puis a = b + 1 : seule sup.",
    },
    Savoir {
        cle: "soustracteur",
        sert: "Soustraire deux nombres.",
        marche: "a − b = a + (NON b) + 1 : on réutilise l'additionneur, avec b inversé et une retenue d'entrée à 1.",
        place: "L'unité de calcul soustrait avec le même additionneur que pour additionner.",
        vraie_vie: "Le rendu de monnaie d'un distributeur : prix payé moins prix de l'article.",
        anecdote: "Les ordinateurs ne soustraient pas vraiment : ils ajoutent l'opposé, en complément à deux.",
        essaie: "Calcule 3 − 5 : d vaut 14 et emprunt s'allume. 14, c'est −2 sur 4 bits !",
    },
    Savoir {
        cle: "incrementeur",
        sert: "Ajouter 1 à un nombre.",
        marche: "Une chaîne de demi-additionneurs : le 1 entre en bas et la retenue remonte.",
        place: "Il fait avancer le compteur de programme d'une instruction à la suivante.",
        vraie_vie: "Le compteur de visiteurs à l'entrée d'un musée : +1 à chaque passage.",
        anecdote: "Le compteur de programme du processeur est un incrémenteur : il passe d'une instruction à la suivante.",
        essaie: "Mets e à 255 : s revient à 0. Comme un compteur kilométrique qui fait le tour.",
    },
    Savoir {
        cle: "negatif",
        sert: "Écrire un nombre négatif comme le font les processeurs.",
        marche: "Complément à deux : on inverse tous les bits et on ajoute 1. Ainsi a + (−a) donne 0 avec le même additionneur.",
        place: "Grâce à lui, un processeur n'a pas besoin d'un circuit à part pour les nombres négatifs.",
        vraie_vie: "Les températures sous zéro, les dettes, les reculs : un ordinateur a besoin des nombres négatifs.",
        anecdote: "En complément à deux, −1 s'écrit 11111111. Et −128 n'a pas d'opposé sur 8 bits : son opposé est lui-même.",
        essaie: "Prends a = 1 : s vaut 255. Ajoute 1 à 255 sur 8 bits : 0. Donc 255 joue bien le rôle de −1.",
    },
    Savoir {
        cle: "decaleur",
        sert: "Décaler les bits d'un nombre de plusieurs rangs d'un coup.",
        marche: "Trois étages de multiplexeurs : le premier décale de 1 ou non, le deuxième de 2, le troisième de 4.",
        place: "Décaler d'un rang multiplie ou divise par 2 : les processeurs s'en servent pour calculer vite.",
        vraie_vie: "Décaler d'un rang vers la gauche, c'est multiplier par 2 : les processeurs s'en servent tout le temps.",
        anecdote: "En base 10, on multiplie par 10 en ajoutant un zéro à droite. En base 2, le décalage fait la même chose pour 2.",
        essaie: "Prends x = 3 et décale de 1, puis de 2 : 6, puis 12.",
    },
    Savoir {
        cle: "multiplicateur",
        sert: "Multiplier deux nombres.",
        marche: "Comme à la main : chaque bit de b donne une ligne « a ou rien », décalée, et des additionneurs les cumulent.",
        place: "Les processeurs simples n'en ont pas : un programme multiplie par additions répétées, comme la calculatrice.",
        vraie_vie: "Les cartes graphiques font des milliards de multiplications par seconde pour calculer les images.",
        anecdote: "La multiplication posée de l'école, en binaire, n'a que des 0 et des 1 : chaque ligne est un ET, puis on additionne.",
        essaie: "Vérifie que 15 × 15 = 225 : c'est le plus grand produit, il tient juste sur 8 bits.",
    },
    Savoir {
        cle: "diviseur",
        sert: "Diviser deux nombres : quotient et reste.",
        marche: "Comme une division posée : on abaisse un bit, on soustrait b quand c'est possible, et le quotient note si on l'a fait.",
        place: "C'est le calcul le plus lent : beaucoup de processeurs le font en plusieurs cycles.",
        vraie_vie: "La division sert partout : convertir des secondes en minutes, répartir des pixels, calculer une moyenne.",
        anecdote: "La division est l'opération la plus lente des processeurs : souvent dix à quarante fois plus lente qu'une addition.",
        essaie: "Divise par 0 : que donne ton circuit ? Les vrais processeurs déclenchent une erreur.",
    },
    Savoir {
        cle: "unite-de-calcul",
        sert: "Faire l'opération que demande l'instruction : recopier, additionner, soustraire ou NAND.",
        marche: "Toutes les opérations sont calculées en même temps ; un multiplexeur garde celle que choisit op.",
        place: "C'est l'UAL du processeur : chaque calcul de l'ordinateur passe par elle.",
        vraie_vie: "Chaque processeur a au moins une UAL ; les modernes en ont plusieurs qui travaillent en même temps.",
        anecdote: "Le circuit 74181, une UAL de 4 bits sur une seule puce, a été au cœur de nombreux mini-ordinateurs des années 1970.",
        essaie: "Change seulement le code d'opération : les mêmes a et b donnent des résultats différents.",
    },
    // Mémoire.
    Savoir {
        cle: "verrou-sr",
        sert: "Retenir un bit : le premier circuit qui se souvient.",
        marche: "Deux NAND croisées : chacune garde l'autre dans son état. Un 0 sur s_n ou r_n force le basculement.",
        place: "Toute la mémoire de l'ordinateur repose sur cette boucle.",
        vraie_vie: "Le bouton marche-arrêt d'une machine : un appui allume, un autre éteint, et l'état reste entre les deux.",
        anecdote: "Deux NAND qui se regardent l'une l'autre suffisent à retenir un bit : c'est la naissance de la mémoire.",
        essaie: "Mets s_n et r_n à 0 en même temps, puis relâche-les ensemble : l'état est imprévisible. C'est le cas interdit.",
    },
    Savoir {
        cle: "verrou-d",
        sert: "Retenir un bit choisi, au moment où on le décide.",
        marche: "Deux NAND devant le verrou SR ne laissent passer d que lorsque e vaut 1 ; ensuite la boucle le garde.",
        place: "Deux verrous D font une bascule D.",
        vraie_vie: "Un appareil photo : tant que le déclencheur est enfoncé, l'image passe ; relâché, elle reste.",
        anecdote: "Le verrou D laisse passer la donnée tant que e vaut 1 : on dit qu'il est transparent.",
        essaie: "Mets e à 1 et joue avec d : q suit. Mets e à 0 : q ne bouge plus.",
    },
    Savoir {
        cle: "bascule-d",
        sert: "Retenir un bit exactement au front montant de l'horloge.",
        marche: "Deux verrous à la suite : le premier écoute quand clk vaut 0, le second recopie quand clk vaut 1. Rien ne traverse les deux à la fois.",
        place: "La brique de toute mémoire rythmée : registres, compteurs, compteur de programme.",
        vraie_vie: "Toutes les mémoires rapides d'un processeur, ses registres, sont faites de bascules.",
        anecdote: "Une bascule D, ce sont deux verrous tête-bêche : l'un s'ouvre quand l'autre se ferme, et la donnée ne passe qu'au front.",
        essaie: "Change d sans toucher clk : q ne bouge pas. Puis fais monter clk.",
    },
    Savoir {
        cle: "bascule-t",
        sert: "Changer d'état à chaque front, quand on le demande.",
        marche: "Une bascule D dont l'entrée est q OU exclusif t.",
        place: "Une bascule T divise la fréquence de l'horloge par deux : c'est le début d'un compteur.",
        vraie_vie: "Le bouton poussoir d'une lampe de chevet : un appui allume, l'appui suivant éteint.",
        anecdote: "Une bascule T divise la fréquence par deux : en enchaîner n, c'est diviser par 2^n. C'est ainsi qu'une montre tire la seconde de son quartz.",
        essaie: "Laisse t à 1 et fais battre clk : q bat deux fois moins vite.",
    },
    Savoir {
        cle: "bascule-jk",
        sert: "Une bascule à tout faire : mettre à 1, à 0, garder ou inverser.",
        marche: "Une bascule D précédée d'un petit circuit qui calcule le nouvel état à partir de j, k et q.",
        place: "Les anciens circuits intégrés en faisaient leur bascule de base.",
        vraie_vie: "La bascule JK était la star des circuits logiques des années 1970.",
        anecdote: "Selon la légende, JK viendrait des initiales de Jack Kilby, l'inventeur du circuit intégré. Ce n'est pas prouvé.",
        essaie: "Mets j et k à 1 : la bascule se comporte comme une bascule T.",
    },
    Savoir {
        cle: "registre-8-bits",
        sert: "Garder un octet, et ne le changer que sur ordre.",
        marche: "Huit bascules D, chacune précédée d'un multiplexeur qui choisit entre garder q et prendre d.",
        place: "L'accumulateur du processeur est un registre : il garde le nombre sur lequel on calcule.",
        vraie_vie: "L'accumulateur de ton processeur est un registre de 8 bits.",
        anecdote: "Les processeurs actuels ont des registres de 64 bits, et même de 512 bits pour les calculs en série.",
        essaie: "Change d avec charge à 0 et fais un front : rien ne change. Puis avec charge à 1.",
    },
    Savoir {
        cle: "registre-a-decalage",
        sert: "Faire avancer des bits en file indienne, un rang par front.",
        marche: "Huit bascules D reliées en chaîne : chacune prend au front la valeur de sa voisine.",
        place: "C'est ainsi qu'un octet sort ou entre bit par bit sur un fil : le cœur des liaisons série.",
        vraie_vie: "Les guirlandes lumineuses et les grands panneaux de LED sont pilotés par des registres à décalage.",
        anecdote: "Un registre à décalage transforme des bits qui arrivent en file en un mot entier : c'est le cœur d'une liaison série.",
        essaie: "Fais entrer 1, puis 0, puis 0 : regarde le 1 avancer d'un rang à chaque front.",
    },
    Savoir {
        cle: "banc-de-registres",
        sert: "Plusieurs registres, dont on lit deux à la fois et on en écrit un.",
        marche: "Un décodeur choisit le registre à écrire, deux multiplexeurs choisissent les deux à lire.",
        place: "Les processeurs modernes calculent entre registres : c'est leur mémoire la plus rapide.",
        vraie_vie: "Un processeur moderne a des dizaines de registres, lus et écrits à chaque cycle.",
        anecdote: "Les processeurs ARM ont 31 registres généraux de 64 bits : un petit banc très rapide, juste à côté du calcul.",
        essaie: "Écris dans un registre, puis lis-en un autre : chacun garde sa valeur.",
    },
    Savoir {
        cle: "memoire-vive",
        sert: "Garder plusieurs octets, chacun à son adresse.",
        marche: "Des registres côte à côte : un décodeur d'adresse dit lequel écrire, un multiplexeur lequel montrer.",
        place: "La RAM de l'ordinateur : les variables du programme y vivent.",
        vraie_vie: "La mémoire vive d'un ordinateur garde les programmes et les données en cours d'utilisation.",
        anecdote: "La mémoire vive s'efface quand on coupe le courant : c'est pour ça qu'on « enregistre » sur disque.",
        essaie: "Écris une valeur à l'adresse 2, change d'adresse, reviens : elle est toujours là.",
    },
    Savoir {
        cle: "memoire-vive-16",
        sert: "Une mémoire plus grande, faite de mémoires plus petites.",
        marche: "Le bit d'adresse du haut choisit quelle petite mémoire répond ; les autres bits vont à toutes.",
        place: "C'est la mémoire vive de l'ordinateur miniature. Les vraies mémoires grandissent de la même façon.",
        vraie_vie: "Un ordinateur portable actuel a environ un milliard de fois plus de mémoire que celle-ci.",
        anecdote: "L'EDSAC, l'un des premiers ordinateurs (1949), gardait sa mémoire dans des tubes remplis de mercure : à peine plus d'un kilo-octet.",
        essaie: "Écris dans toutes les cases, puis relis-les dans le désordre : chacune répond.",
    },
    Savoir {
        cle: "pile",
        sert: "Empiler et dépiler des octets : le dernier posé sort le premier.",
        marche: "Des registres en colonne : empiler pousse tout d'un cran vers le bas, dépiler fait remonter.",
        place: "Les processeurs rangent dans une pile l'adresse de retour d'un sous-programme.",
        vraie_vie: "Le bouton « annuler » d'un logiciel empile tes actions : la dernière faite est la première défaite.",
        anecdote: "Quand un programme appelle une fonction, l'adresse de retour est rangée sur une pile.",
        essaie: "Empile 1, 2 et 3, puis dépile : ils ressortent 3, 2, 1.",
    },
    Savoir {
        cle: "file",
        sert: "Faire attendre des octets dans leur ordre d'arrivée.",
        marche: "Le plus ancien est toujours en tête ; lire fait avancer tout le monde, écrire remplit la première case libre.",
        place: "Elle garde les touches tapées trop vite, ou les octets reçus par la carte réseau.",
        vraie_vie: "Une imprimante range les documents en attente dans une file : premier arrivé, premier imprimé.",
        anecdote: "Les cartes réseau et les cartes son ont des files, pour que rien ne se perde si le processeur est occupé.",
        essaie: "Remplis la file jusqu'à ce que plein s'allume : que se passe-t-il si tu écris encore ?",
    },
    // Compter et cadencer.
    Savoir {
        cle: "compteur-4-bits",
        sert: "Compter les fronts de l'horloge.",
        marche: "Un registre et un incrémenteur en boucle : à chaque front, le registre prend sa valeur plus 1.",
        place: "Le compteur de programme est un compteur qui sait aussi sauter.",
        vraie_vie: "Le compteur de points d'un tableau d'affichage sportif.",
        anecdote: "Un compteur de 4 bits fait le tour en 16 fronts : de 15, il revient à 0.",
        essaie: "Compte jusqu'à 15 et fais un front de plus : il revient à 0.",
    },
    Savoir {
        cle: "compteur-decimal",
        sert: "Compter de 0 à 9, comme nos chiffres.",
        marche: "Un compteur qui se remet à 0 quand il atteint 9 : un petit circuit reconnaît le 9.",
        place: "Deux compteurs décimaux font un chronomètre ; plusieurs, un compteur de kilomètres.",
        vraie_vie: "Les compteurs d'eau et d'électricité, les afficheurs de pompe à essence.",
        anecdote: "Le codage DCB, un chiffre décimal sur 4 bits, servait dans les calculatrices pour éviter les erreurs d'arrondi du binaire.",
        essaie: "Compte jusqu'à 9 : au front suivant, il revient à 0 au lieu de passer à 10.",
    },
    Savoir {
        cle: "compteur-haut-bas",
        sert: "Compter dans les deux sens.",
        marche: "Le même additionneur ajoute 1 ou 15 : sur 4 bits, ajouter 15 revient à enlever 1.",
        place: "Le pointeur d'une pile monte et descend ainsi.",
        vraie_vie: "Le compteur de places d'un parking : une voiture entre, +1 ; elle sort, −1.",
        anecdote: "Les roues codeuses des boutons de volume comptent dans les deux sens selon le sens de rotation.",
        essaie: "Monte jusqu'à 3, puis redescends : le compteur repasse par les mêmes valeurs.",
    },
    Savoir {
        cle: "generateur-aleatoire",
        sert: "Produire une suite qui a l'air tirée au hasard.",
        marche: "Un registre à décalage qui se nourrit du OU exclusif de quelques-uns de ses bits : il parcourt 255 valeurs avant de revenir.",
        place: "Les jeux, le brouillage des transmissions et les tests de circuits s'en servent.",
        vraie_vie: "Les jeux vidéo des années 1980 tiraient leur hasard de registres comme celui-ci.",
        anecdote: "Le tien passe par 255 valeurs avant de revenir au départ : toutes sauf 0, car un registre à 0 y resterait pour toujours.",
        essaie: "Note les premières valeurs après raz, puis recommence : la même suite revient. C'est un hasard… prévisible.",
    },
    Savoir {
        cle: "minuterie",
        sert: "Compter à rebours et prévenir quand c'est fini.",
        marche: "Un registre qu'on charge, puis qui descend de 1 à chaque front jusqu'à 0.",
        place: "Un ordinateur a des minuteries pour réveiller le processeur à intervalles réguliers.",
        vraie_vie: "Le four, le micro-ondes, le minuteur de cuisine : compter à rebours jusqu'à la sonnerie.",
        anecdote: "Les processeurs ont des minuteries internes qui les interrompent régulièrement, pour partager le temps entre les programmes.",
        essaie: "Charge 3 et laisse battre l'horloge : combien de fronts avant la fin ?",
    },
    Savoir {
        cle: "feu-tricolore",
        sert: "Faire passer une machine par des étapes, dans l'ordre, au rythme de l'horloge.",
        marche: "Un compteur donne l'étape, un petit circuit combinatoire en déduit les lampes : c'est un automate.",
        place: "Le processeur est un automate de ce genre, en beaucoup plus grand.",
        vraie_vie: "Les vrais feux sont des automates : un état par couleur, et une règle pour passer au suivant.",
        anecdote: "Le premier feu électrique à trois couleurs a été installé à Détroit en 1920.",
        essaie: "Suis les couleurs front après front : dans quel ordre reviennent-elles ?",
    },
    Savoir {
        cle: "chronometre",
        sert: "Compter les secondes de 00 à 59.",
        marche: "Deux compteurs décimaux : celui des dizaines avance quand celui des unités repasse à 0, et s'arrête à 5.",
        place: "Des compteurs en cascade : c'est ainsi qu'une horloge compte heures, minutes et secondes.",
        vraie_vie: "Le chronomètre des courses de ski, au centième de seconde.",
        anecdote: "Aux Jeux olympiques, le chronométrage électronique a remplacé les chronomètres à main dans les années 1960.",
        essaie: "Laisse passer 59 fronts, puis un de plus : 59 devient 00.",
    },
    // Communiquer.
    Savoir {
        cle: "liaison-serie",
        sert: "Envoyer un octet sur un seul fil.",
        marche: "Un registre à décalage pousse les bits un à un, encadrés d'un 0 de départ et d'un 1 d'arrêt ; des bascules comptent ce qui reste.",
        place: "Les ports série, l'USB et les réseaux envoient leurs données bit par bit de cette façon. Le récepteur viendra dans la branche de la carte réseau.",
        vraie_vie: "Le port USB est une liaison série : les bits passent l'un après l'autre.",
        anecdote: "Le bit de départ et le bit d'arrêt viennent des téléscripteurs du début du XXe siècle.",
        essaie: "Envoie 0 puis 255 : regarde la différence sur tx.",
    },
    Savoir {
        cle: "interruptions",
        sert: "Laisser un appareil réclamer l'attention du processeur.",
        marche: "Une bascule par demande la garde jusqu'à son acquittement ; un encodeur à priorité désigne la plus urgente.",
        place: "Le clavier ou la carte réseau interrompent le processeur au lieu d'attendre qu'il vienne voir.",
        vraie_vie: "Quand tu appuies sur une touche, le clavier interrompt le processeur pour qu'il la lise.",
        anecdote: "Sans interruptions, le processeur devrait surveiller chaque appareil en boucle : on appelle ça l'attente active.",
        essaie: "Demande deux interruptions à la fois : laquelle est servie la première ?",
    },
    // Le processeur.
    Savoir {
        cle: "compteur-de-programme",
        sert: "Savoir quelle instruction exécuter ensuite.",
        marche: "Un registre qui, à chaque front, avance de 1, saute à une cible, reste en place ou revient à 0.",
        place: "Il donne l'adresse lue dans la mémoire morte : c'est lui qui fait dérouler le programme.",
        vraie_vie: "Il indique toujours où en est le processeur dans son programme : sa page en cours.",
        anecdote: "Un saut, c'est simplement charger une nouvelle valeur dans ce compteur au lieu d'ajouter 1.",
        essaie: "Laisse-le compter, puis mets cible à 0 et saut à 1 : il repart du début, comme un programme qui recommence.",
    },
    Savoir {
        cle: "decodeur-d-instructions",
        sert: "Comprendre une instruction et donner les ordres correspondants.",
        marche: "Un grand circuit combinatoire : du code de l'instruction et des drapeaux, il déduit chaque commande (écrire, sauter, quelle opération).",
        place: "C'est le chef d'orchestre du processeur : il ne calcule rien, il commande tout.",
        vraie_vie: "Chaque instruction d'un processeur passe par un décodeur, des milliards de fois par seconde.",
        anecdote: "Les processeurs x86 ont des milliers d'instructions ; leur décodeur est l'une de leurs parties les plus compliquées.",
        essaie: "Choisis le code 5 (ADD) : quelles commandes s'allument ?",
    },
    Savoir {
        cle: "processeur",
        sert: "Exécuter un programme, instruction après instruction.",
        marche: "Le compteur de programme désigne l'instruction, le décodeur la traduit, l'unité de calcul la fait, l'accumulateur garde le résultat. Un cycle d'horloge par instruction.",
        place: "Le cerveau de l'ordinateur. Tout le reste lui apporte des données ou les emporte.",
        vraie_vie: "Ton téléphone a un processeur fait de milliards de transistors. Le principe est le même que le tien.",
        anecdote: "L'Intel 4004, premier microprocesseur commercial (1971), comptait environ 2 300 transistors.",
        essaie: "Ouvre ton processeur et suis le chemin d'une instruction, de instr jusqu'à a.",
    },
    // Ordinateurs.
    Savoir {
        cle: "ordinateur",
        sert: "Faire tourner un vrai programme, du clavier jusqu'à l'écran.",
        marche: "La mémoire morte donne les instructions au processeur ; la mémoire vive garde ses nombres ; l'horloge rythme le tout.",
        place: "C'est l'ordinateur complet : 3 401 portes NAND.",
        vraie_vie: "Un ordinateur, c'est un processeur, de la mémoire et des appareils d'entrée-sortie : ton ordinateur a tout cela.",
        anecdote: "L'architecture « programme en mémoire » a été décrite par John von Neumann en 1945 et porte souvent son nom.",
        essaie: "Tape d'autres chiffres : le même câblage donne d'autres résultats.",
    },
    Savoir {
        cle: "ordinateur-majuscules",
        sert: "Montrer qu'un même matériel fait tout autre chose avec un autre programme.",
        marche: "Seule la mémoire morte change : le programme lit une touche, la passe en majuscule si besoin, et l'affiche.",
        place: "C'est l'idée qui rend les ordinateurs universels : le matériel est fixe, le programme change tout.",
        vraie_vie: "Changer de programme sans changer de matériel : c'est tout ce qui sépare une calculatrice d'un traitement de texte.",
        anecdote: "En ASCII, une minuscule et sa majuscule ne diffèrent que d'un bit, celui qui vaut 32.",
        essaie: "Tape des chiffres et des signes : ils passent tels quels, seules les lettres changent.",
    },
    Savoir {
        cle: "ordinateur-a-toi",
        sert: "Construire l'ordinateur entier avec tes propres briques.",
        marche: "Ton processeur et ta mémoire vive, gagnés dans les quêtes, remplacent ceux fournis ; le branchement est le même.",
        place: "C'est le but du parcours : un ordinateur dont tu as posé chaque porte, réutilisable comme base.",
        vraie_vie: "Tu as refait, en miniature, le chemin des ingénieurs des années 1940 à 1970.",
        anecdote: "Plus de 3 400 portes NAND, toutes posées par toi ou par tes briques : chaque calcul les traverse.",
        essaie: "Ouvre une de tes briques et descends jusqu'aux NAND : tout vient de là.",
    },
    // Le réseau.
    Savoir {
        cle: "detecteur-de-front",
        sert: "Réagir au moment où un signal passe de 0 à 1, une seule fois, même s'il reste à 1 longtemps.",
        marche: "Une bascule D se souvient de e au front précédent ; front = e ET NON(ancien) ne vaut 1 que juste après la montée.",
        place: "Le récepteur repère ainsi le début d'une trame, et le clavier qu'une touche vient d'être enfoncée.",
        vraie_vie: "Un bouton de souris ne compte qu'un clic, même si on le garde enfoncé : on détecte le moment où il s'enfonce.",
        anecdote: "Un bouton mécanique « rebondit » : il s'ouvre et se ferme plusieurs fois en quelques millisecondes. Les circuits doivent filtrer ces rebonds.",
        essaie: "Garde e à 1 longtemps : l'impulsion ne dure qu'un front.",
    },
    Savoir {
        cle: "cadence",
        sert: "Donner un rythme plus lent que l'horloge : un tic tous les huit fronts.",
        marche: "Un compteur de 0 à 7 ; un ET de ses trois bits repère le 7.",
        place: "Une liaison série tourne bien plus lentement que le processeur : la cadence fixe sa vitesse, comme les bauds d'un vrai port série.",
        vraie_vie: "Une carte réseau tire la vitesse de ses bits d'une horloge bien plus rapide, divisée.",
        anecdote: "La vitesse d'une liaison série se mesure en bauds : 9 600 bauds, c'était la vitesse courante des modems et des terminaux.",
        essaie: "Compte les fronts de clk entre deux tic : c'est le facteur de division, ici 8.",
    },
    Savoir {
        cle: "controle-de-parite",
        sert: "Vérifier qu'un octet est arrivé sans bit inversé.",
        marche: "L'émetteur ajoute un bit pour que le nombre de 1 soit pair ; le récepteur recompte avec ta porte de parité, et une équivalence dit si c'est toujours pair.",
        place: "La mémoire et les réseaux protègent ainsi leurs données. Un seul bit faux se voit ; deux s'annulent, d'où des codes plus forts ailleurs.",
        vraie_vie: "Les anciennes liaisons série envoyaient un bit de parité avec chaque caractère.",
        anecdote: "Avec un bit de parité, une erreur sur deux bits passe inaperçue : la somme des retournements est paire.",
        essaie: "Retourne deux bits de d à la fois : ok reste allumé. Le contrôle ne voit rien !",
    },
    Savoir {
        cle: "recepteur-serie",
        sert: "Rassembler en octet les bits qui arrivent un à un sur un fil.",
        marche: "Au repos, un 0 annonce une trame. Neuf bascules se vident d'un rang par front et comptent les bits ; un registre à décalage les ramasse ; le bit d'arrêt dit si la trame est bonne.",
        place: "La moitié qui écoute d'une carte réseau : sans lui, l'émetteur parlerait dans le vide.",
        vraie_vie: "Le récepteur d'un port série, d'un module Bluetooth ou d'un GPS fait ce travail.",
        anecdote: "Le circuit qui fait émetteur et récepteur s'appelle un UART. Le 8250 de National Semiconductor équipait les premiers PC.",
        essaie: "Envoie une trame sans bit d'arrêt : erreur s'allume au lieu de pret.",
    },
    Savoir {
        cle: "liaison-complete",
        sert: "Faire passer un octet d'une machine à une autre par un seul fil.",
        marche: "L'émetteur et le récepteur partagent l'horloge : chaque front, l'un pose un bit sur le fil, l'autre lit celui d'avant.",
        place: "C'est un réseau en miniature : deux machines, un fil, une règle commune pour découper les octets.",
        vraie_vie: "Deux appareils reliés par un câble série, comme une carte Arduino et un ordinateur.",
        anecdote: "Une trame de 10 bits porte 8 bits utiles : 20 % de la ligne sert à l'emballage.",
        essaie: "Compte les fronts entre envoyer et pret : 10, un par bit de la trame.",
    },
    Savoir {
        cle: "carte-reseau",
        sert: "Envoyer et recevoir des octets pendant que le processeur fait autre chose.",
        marche: "Un émetteur, un récepteur, et une file qui garde jusqu'à quatre octets reçus ; dispo dit qu'il y en a, lire en retire un.",
        place: "Branchée à l'ordinateur, elle lui permet de parler à un autre ordinateur. Avec le contrôleur d'interruptions, elle peut même le prévenir quand un octet arrive.",
        vraie_vie: "Toute carte réseau, Wi-Fi ou Ethernet, combine un émetteur, un récepteur et des files d'attente.",
        anecdote: "Ethernet a été inventé chez Xerox en 1973. Ses trames portent aussi une somme de contrôle, sur 32 bits.",
        essaie: "Envoie plusieurs octets de suite sans les lire : ils attendent dans la file, dans l'ordre.",
    },
    // La montre numérique.
    Savoir {
        cle: "compteur-cascade",
        sert: "Compter de 0 à 9, mais seulement quand on le lui demande, et prévenir le chiffre suivant.",
        marche: "Un compteur décimal dont chaque bascule garde sa valeur quand en vaut 0 (un multiplexeur par bit). La retenue vaut 1 quand il va repasser de 9 à 0.",
        place: "Les compteurs en cascade sont les rouages des montres, des compteurs de kilomètres et des minuteries.",
        vraie_vie: "Les compteurs kilométriques mécaniques : une roue fait le tour et pousse la suivante d'un cran.",
        anecdote: "La Pascaline de Blaise Pascal (1642) propageait déjà les retenues d'une roue à l'autre.",
        essaie: "Laisse en à 1 et compte jusqu'à 9 : retenue s'allume pile au bon moment.",
    },
    Savoir {
        cle: "compteur-60",
        sert: "Compter les secondes ou les minutes, de 00 à 59.",
        marche: "Deux compteurs en cascade : les unités vont de 0 à 9, les dizaines de 0 à 5 et n'avancent que sur la retenue des unités. Chaque chiffre a ses 4 bits : c'est le décimal codé binaire, facile à afficher.",
        place: "Le décimal codé binaire évite une division pour afficher : chaque groupe de 4 bits va droit à un afficheur 7 segments.",
        vraie_vie: "Les secondes et les minutes d'une montre comptent toutes deux jusqu'à 59.",
        anecdote: "Le découpage en 60 vient des Babyloniens, qui comptaient en base 60. On en garde les minutes, les secondes et les degrés.",
        essaie: "Approche de 59 et fais un front de plus : 59 devient 00, et retenue prévient le compteur suivant.",
    },
    Savoir {
        cle: "compteur-24",
        sert: "Compter les heures, de 00 à 23.",
        marche: "Les mêmes deux chiffres, mais un petit circuit reconnaît 23 et remet tout à 0 au lieu de passer à 24.",
        place: "Reconnaître une valeur et forcer un retour est la façon de faire un compteur de n'importe quelle longueur.",
        vraie_vie: "Le compteur des heures d'un affichage à 24 heures, de 00 à 23.",
        anecdote: "Le passage de 23 à 00 est délicat : c'est 2 et 3 qu'il faut reconnaître, pas 24.",
        essaie: "Va jusqu'à 23 : au front suivant, les deux chiffres reviennent à 0 ensemble.",
    },
    Savoir {
        cle: "reveil",
        sert: "Retenir une heure et prévenir quand elle arrive.",
        marche: "Deux registres gardent l'heure du réveil ; des tests d'égalité la comparent à l'heure qui passe ; une bascule « sonne » se met à 1 et s'y tient.",
        place: "Comparer un compteur à une valeur retenue, c'est aussi ce que fait la minuterie d'un ordinateur pour lever une interruption.",
        vraie_vie: "Le réveil du téléphone et de la table de chevet compare l'heure à l'heure réglée, sans cesse.",
        anecdote: "Le comparateur ne demande pas « est-il plus tard ? » mais « est-ce exactement l'heure ? » : le réveil ne sonne qu'une fois par jour.",
        essaie: "Retiens une heure avec regler, puis amène h et m jusqu'à elle : sonne s'allume et reste allumé jusqu'à arreter.",
    },
    Savoir {
        cle: "montre",
        sert: "Donner l'heure, et se régler.",
        marche: "Trois compteurs en cascade : la retenue des secondes fait avancer les minutes, celle des minutes les heures. Les boutons de réglage injectent une retenue à la main.",
        place: "Les ordinateurs ont une horloge de ce genre, qui garde l'heure même éteints grâce à une petite pile.",
        vraie_vie: "La montre numérique, apparue dans les années 1970, comptait déjà comme la tienne.",
        anecdote: "Une montre à quartz : un cristal vibre 32 768 fois par seconde, soit 2^15. Quinze divisions par deux donnent la seconde.",
        essaie: "Amène l'heure à 23:59 avec plus_h et plus_m, puis laisse filer les secondes : à minuit, tout revient à 00:00:00.",
    },
    // Le contrôleur d'écran.
    Savoir {
        cle: "balayage",
        sert: "Parcourir l'écran pixel après pixel, ligne après ligne.",
        marche: "Un compteur pour la colonne x ; quand il repasse à 0, sa retenue fait avancer le compteur de la ligne y.",
        place: "Les écrans reçoivent leurs pixels dans cet ordre, des dizaines de fois par seconde.",
        vraie_vie: "Les écrans se dessinent ligne par ligne, plusieurs dizaines de fois par seconde.",
        anecdote: "Les vieux téléviseurs à tube balayaient l'écran avec un faisceau d'électrons, 50 images par seconde en Europe.",
        essaie: "Compte les fronts d'une image complète : 8 × 8 = 64.",
    },
    Savoir {
        cle: "police",
        sert: "Dessiner les chiffres en petits carrés.",
        marche: "Une mémoire morte : le chiffre et le numéro de ligne désignent une case, qui contient le dessin de cette ligne. En circuit : deux décodeurs et des OU.",
        place: "Les premiers terminaux et ordinateurs affichaient leurs textes avec une police gravée dans une mémoire morte de ce genre.",
        vraie_vie: "Chaque lettre de ton écran est un petit dessin de points, rangé dans une police de caractères.",
        anecdote: "Les premières consoles et les calculatrices affichaient des caractères de 5 × 7 points, dessinés à la main.",
        essaie: "Dessine dans ta tête le chiffre 8 à partir de la table : les cinq lignes forment-elles bien un 8 ?",
    },
    Savoir {
        cle: "memoire-video",
        sert: "Garder l'image à afficher : un bit par pixel.",
        marche: "Une mémoire vive à part : le processeur y écrit des lignes, le contrôleur y lit le pixel qu'il dessine.",
        place: "Les cartes graphiques ont leur propre mémoire vidéo : écrire dedans, c'est dessiner.",
        vraie_vie: "La mémoire d'une carte graphique garde l'image affichée, et l'écran la relit sans cesse.",
        anecdote: "La console Atari 2600 (1977) n'avait pas de mémoire d'image : les jeux redessinaient chaque ligne au passage du faisceau, au cycle près.",
        essaie: "Écris une ligne, puis une autre : la première reste en place.",
    },
    Savoir {
        cle: "serialiseur",
        sert: "Envoyer les pixels d'une ligne un par un.",
        marche: "Un registre qu'on charge d'un coup, puis qui décale ses bits d'un rang à chaque front.",
        place: "Le même principe que la liaison série : parallèle dans la machine, en série sur le câble.",
        vraie_vie: "Le câble HDMI fait passer les pixels un par un sur quelques fils, à très grande vitesse.",
        anecdote: "Sérialiser, c'est échanger de la largeur contre du temps : un fil au lieu de huit, mais huit fois plus de fronts.",
        essaie: "Charge 10000001 : le premier et le dernier bit sortent, le milieu reste noir.",
    },
    Savoir {
        cle: "controleur-ecran",
        sert: "Transformer la mémoire vidéo en image.",
        marche: "Le balayage dit quel pixel l'écran dessine ; la mémoire vidéo donne sa couleur à ce moment-là.",
        place: "Avec lui, l'ordinateur peut dessiner : c'est le cœur d'une carte graphique.",
        vraie_vie: "Chaque ordinateur a un contrôleur d'écran, aujourd'hui caché dans la carte graphique.",
        anecdote: "La carte couleur CGA des premiers PC, en 1981, avait 16 Ko de mémoire vidéo.",
        essaie: "Écris le motif d'une lettre, ligne par ligne, et suis le pixel sortir point par point.",
    },
    // Maths et automates.
    Savoir {
        cle: "gray",
        sert: "Écrire les nombres pour qu'un seul bit change d'un nombre au suivant.",
        marche: "Chaque bit du code est le OU exclusif de deux bits voisins du nombre.",
        place: "Les roues codeuses et les compteurs entre deux horloges l'utilisent : pendant un changement, on ne lit jamais une valeur absurde.",
        vraie_vie: "Les roues codeuses des robots et des boutons rotatifs utilisent le code de Gray.",
        anecdote: "Frank Gray l'a breveté en 1953 pour la télévision. Un seul bit change à la fois : aucune lecture ne tombe entre deux valeurs fausses.",
        essaie: "Compte de 7 à 8 en binaire : 4 bits changent. Regarde g : un seul.",
    },
    Savoir {
        cle: "motif-101",
        sert: "Repérer une suite de bits précise dans un flot.",
        marche: "Un automate : ses bascules résument ce qu'il a vu, une porte décide s'il reconnaît le motif.",
        place: "Les cartes réseau reconnaissent ainsi le début d'une trame ; les compilateurs reconnaissent les mots d'un programme avec des automates.",
        vraie_vie: "Les cartes réseau repèrent le début d'une trame en reconnaissant un motif de bits fixé.",
        anecdote: "Ce genre d'automate s'appelle un automate fini. Les moteurs de recherche de texte en construisent pour trouver des mots.",
        essaie: "Envoie 1, 0, 1, 0, 1 : le motif est reconnu deux fois, car les deux 101 se chevauchent.",
    },
    Savoir {
        cle: "divisible-3",
        sert: "Savoir si un nombre est un multiple de 3, quelle que soit sa longueur.",
        marche: "Lire un bit de plus double le nombre et ajoute e : le reste devient (2r + e) modulo 3. Il suffit de garder le reste, donc trois états et deux bascules.",
        place: "C'est un automate fini : une mémoire bornée qui répond à une question sur un flot infini. La théorie des automates fonde les langages et les compilateurs.",
        vraie_vie: "Les automates qui calculent un reste servent à vérifier des numéros, comme ceux des cartes bancaires.",
        anecdote: "Un nombre est multiple de 3 si la somme de ses chiffres l'est. En binaire, il y a une règle aussi, et ton automate la suit.",
        essaie: "Envoie 1, 1 (le nombre 3) : div s'allume. Puis 0 : 6 est aussi multiple de 3.",
    },
    Savoir {
        cle: "cellule-vie",
        sert: "Décider si une cellule vit à la génération suivante.",
        marche: "On compte les voisins vivants avec des additionneurs ; 3 voisins donnent la vie, 2 la gardent, le reste la retire.",
        place: "Une grille de ces cellules calcule tout ce qu'un ordinateur sait calculer : le jeu de la vie est un ordinateur à lui seul.",
        vraie_vie: "Le jeu de la vie sert à étudier la croissance, les foules et les épidémies.",
        anecdote: "John Conway a inventé ce jeu en 1970. Avec ses règles, on peut même construire un ordinateur complet.",
        essaie: "Mets v à 0 et allume trois voisins : la cellule naît. Avec quatre, elle reste morte.",
    },
    Savoir {
        cle: "automate-30",
        sert: "Faire évoluer une ligne de cellules selon une règle simple.",
        marche: "Chaque cellule regarde sa gauche, elle-même et sa droite ; la règle 30 dit gauche ⊕ (centre OU droite).",
        place: "Une règle de trois bits produit un dessin chaotique : Wolfram l'a utilisée comme générateur de hasard.",
        vraie_vie: "La règle 30 a servi de générateur de hasard dans le logiciel Mathematica.",
        anecdote: "Stephen Wolfram l'a étudiée en 1983 : une règle à trois cellules, et pourtant un dessin chaotique.",
        essaie: "Charge un seul 1 et fais quelques fronts : regarde le motif s'étaler et se désordonner.",
    },
    Savoir {
        cle: "pgcd",
        sert: "Trouver le plus grand diviseur commun de deux nombres.",
        marche: "L'algorithme d'Euclide par soustractions : le PGCD de x et y est aussi celui de y et x − y. À chaque front, le plus grand devient la différence, jusqu'à l'égalité.",
        place: "Un algorithme devenu circuit : un comparateur décide, des soustracteurs calculent, des registres retiennent. C'est ainsi qu'on fabrique un processeur spécialisé.",
        vraie_vie: "Le PGCD sert à simplifier les fractions, et la cryptographie RSA s'appuie sur l'algorithme d'Euclide.",
        anecdote: "Euclide l'a décrit vers 300 avant notre ère : l'un des plus vieux algorithmes encore utilisés.",
        essaie: "Charge 12 et 8 et compte les fronts jusqu'à fini : x vaut 4.",
    },
    Savoir {
        cle: "cesar",
        sert: "Cacher un message en décalant chaque lettre d'un même nombre de rangs.",
        marche: "Une addition modulo 16 : l'additionneur de 4 bits oublie sa retenue de sortie, donc après 15 on revient à 0, comme l'alphabet qui reprend à A après Z.",
        place: "Le premier code secret de l'histoire, et ton additionneur réutilisé tel quel. Il prépare la messagerie chiffrée des grands projets.",
        vraie_vie: "On le retrouve dans les jeux de piste et les énigmes ; le ROT13 des forums est un César de 13 rangs.",
        anecdote: "Suétone raconte que Jules César décalait ses lettres de trois rangs. Avec seulement 26 clés possibles, on le casse en essayant tout.",
        essaie: "Chiffre avec k = 3, puis rechiffre le résultat avec k = 13 : tu as déchiffré, car 3 + 13 = 16.",
    },
    Savoir {
        cle: "masque-xor",
        sert: "Chiffrer un octet avec une clé, et le déchiffrer avec la même clé.",
        marche: "Le OU exclusif retourne les bits de d là où la clé a un 1. Refaire la même opération les retourne une seconde fois : on retrouve le message.",
        place: "Le cœur des chiffrements modernes, qui mélangent les bits avec des OU exclusifs, encore et encore.",
        vraie_vie: "Le « téléphone rouge » entre Washington et Moscou a utilisé des masques jetables.",
        anecdote: "Claude Shannon a prouvé en 1949 que le masque jetable est parfaitement sûr… à condition que la clé soit aussi longue que le message, au hasard, et jamais réutilisée.",
        essaie: "Prends k = 255 : chaque bit de d est retourné. Et avec k = 0 ?",
    },
    Savoir {
        cle: "somme-controle",
        sert: "Résumer un message en un seul octet, pour voir s'il a été abîmé en route.",
        marche: "Un accumulateur : un registre et un additionneur. À chaque octet, la somme grandit ; l'émetteur et le récepteur la calculent chacun et la comparent.",
        place: "Les paquets d'Internet portent une somme de ce genre. Le contrôle de parité de la branche réseau en était la version d'un seul bit.",
        vraie_vie: "Les fichiers téléchargés, les paquets IP et les codes-barres portent une somme de contrôle.",
        anecdote: "Une somme simple ne voit pas deux octets échangés : 1 + 2 = 2 + 1. Les codes CRC, plus malins, s'en aperçoivent.",
        essaie: "Ajoute 200 puis 100 : la somme vaut 44, car on compte modulo 256.",
    },
    Savoir {
        cle: "hamming-code",
        sert: "Protéger 4 bits de données avec 3 bits de parité placés avec astuce.",
        marche: "Chaque bit de parité surveille les places dont le numéro contient une certaine puissance de 2 : 1, 2 ou 4. Une erreur à la place 6 = 2 + 4 trouble donc les contrôles 2 et 4, et eux seuls.",
        place: "Le codeur de la mémoire qui se répare toute seule : il écrit, le correcteur relit.",
        vraie_vie: "Les mémoires ECC des serveurs ajoutent des bits de Hamming à chaque mot.",
        anecdote: "Richard Hamming l'a inventé en 1950, agacé que son ordinateur s'arrête à chaque erreur pendant les week-ends.",
        essaie: "Code d = 0 : h vaut 0. Code d = 15 : h vaut 127. Seuls 16 des 128 mots de 7 bits sont des mots du code.",
    },
    Savoir {
        cle: "hamming-correcteur",
        sert: "Retrouver le message exact même quand un de ses 7 bits a été retourné.",
        marche: "Les trois contrôles de parité, lus comme un nombre en binaire, donnent la place du bit faux. Un décodeur la désigne, un OU exclusif la remet d'aplomb.",
        place: "Le grand élément de la branche. Les mémoires ECC des serveurs et les sondes spatiales font exactement cela, avec des mots plus longs.",
        vraie_vie: "Les sondes spatiales, les QR codes et les disques utilisent des codes correcteurs, cousins de celui-ci.",
        anecdote: "Deux mots du code de Hamming diffèrent toujours d'au moins trois bits : avec une seule erreur, on reste plus près du bon mot que de tous les autres.",
        essaie: "Retourne deux bits à la fois : le correcteur se trompe. Il ne promet de réparer qu'une seule erreur.",
    },
    Savoir {
        cle: "arbitre",
        sert: "Dire qui gagne une manche de pierre-feuille-ciseaux.",
        marche: "Chaque coup est décodé en trois fils (pierre, feuille, ciseaux). Un ET par duel gagnant, puis un OU pour rassembler les trois façons de gagner.",
        place: "Une règle de jeu traduite en table de vérité : c'est le travail de tout circuit de décision.",
        vraie_vie: "Les arbitres électroniques sont partout : la ligne du tennis, la photo d'arrivée, le hors-jeu.",
        anecdote: "Pierre-feuille-ciseaux est un jeu parfaitement équilibré : chaque coup en bat un et perd contre un autre.",
        essaie: "Joue 3 contre n'importe quoi : rien ne s'allume, le coup n'existe pas.",
    },
    Savoir {
        cle: "morpion",
        sert: "Voir d'un coup si trois croix sont alignées.",
        marche: "Les huit alignements sont huit ET à trois entrées ; un OU les réunit. Le circuit regarde toute la grille en même temps.",
        place: "Ce que ferait un programme avec une boucle, le circuit le fait en un instant, en parallèle.",
        vraie_vie: "Les premières intelligences artificielles de jeu ont commencé par le morpion.",
        anecdote: "En 1952, Alexander Douglas a programmé OXO, un morpion sur l'ordinateur EDSAC : l'un des tout premiers jeux vidéo.",
        essaie: "Combien de croix faut-il au minimum pour gagner ? Et au maximum, combien peut-on en poser sans aligner ?",
    },
    Savoir {
        cle: "de",
        sert: "Tirer un nombre de 1 à 6 au hasard.",
        marche: "Un compteur qui tourne de 1 à 6 très vite tant qu'on appuie. Personne ne peut lâcher le bouton au bon cycle : la face est imprévisible.",
        place: "Un compteur à cycle court, comme le feu tricolore, et une façon simple de faire du hasard avec un humain.",
        vraie_vie: "Les dés électroniques des jeux de société et le hasard des jeux vidéo.",
        anecdote: "Ton dé est juste : chaque face dure exactement un front d'horloge, aucune n'est avantagée.",
        essaie: "Garde lancer à 1 et fais six fronts : tu repasses par toutes les faces.",
    },
    Savoir {
        cle: "meilleur-score",
        sert: "Garder le plus haut score et annoncer quand il est battu.",
        marche: "Un comparateur de 8 bits dit si le score dépasse le record ; si on valide, le registre charge le nouveau record.",
        place: "Un registre qui ne charge que sous condition : comme l'accumulateur du processeur, qui n'écrit que si l'instruction le demande.",
        vraie_vie: "Les bornes d'arcade gardaient le meilleur score, avec les initiales du champion.",
        anecdote: "Le premier jeu à enregistrer un meilleur score serait Sea Wolf (1976).",
        essaie: "Valide 100, puis 50 : le record reste 100. Valide 101 : nouveau s'allume.",
    },
    Savoir {
        cle: "premier",
        sert: "Reconnaître les nombres premiers de 0 à 15.",
        marche: "Pas de division : une table de vérité de 16 lignes, simplifiée en quatre termes. Chaque terme couvre deux nombres premiers.",
        place: "La preuve qu'une propriété mathématique, sur des nombres bornés, devient un simple circuit de portes.",
        vraie_vie: "Les grands nombres premiers protègent les paiements en ligne : ils sont au cœur du chiffrement RSA.",
        anecdote: "Il y a une infinité de nombres premiers : Euclide l'a démontré il y a plus de 2 300 ans.",
        essaie: "Cherche le seul nombre premier pair. Puis vérifie que 1 n'est pas premier.",
    },
    Savoir {
        cle: "carre",
        sert: "Calculer n × n.",
        marche: "Le multiplicateur reçoit deux fois le même nombre. Rien d'autre à faire.",
        place: "Réutiliser une brique gagnée, c'est tout l'art de l'électronique : on construit grand avec des pièces déjà prouvées.",
        vraie_vie: "Calculer des distances, des surfaces, l'énergie : les carrés sont partout en physique et dans les jeux.",
        anecdote: "Les carrés se suivent en ajoutant les nombres impairs : 1, 1 + 3 = 4, 4 + 5 = 9, 9 + 7 = 16…",
        essaie: "Vérifie que deux carrés qui se suivent diffèrent d'un nombre impair.",
    },
    Savoir {
        cle: "binaire-decimal",
        sert: "Écrire un octet en chiffres décimaux, pour l'afficher.",
        marche: "Le « double dabble » : décaler un nombre binaire vers la gauche le double. En ajoutant 3 à tout chiffre qui vaut au moins 5 avant le décalage, chaque chiffre déborde au bon moment vers le suivant, comme en décimal.",
        place: "C'est ce qu'il y a entre le processeur et l'afficheur d'une calculatrice ou d'un réveil.",
        vraie_vie: "Chaque afficheur qui montre un nombre calculé en binaire fait cette conversion.",
        anecdote: "Le « double dabble » ne divise jamais par 10 : il décale et ajoute 3, deux opérations très simples en portes.",
        essaie: "Essaie 255 : c = 2, d = 5, u = 5. Puis 100 : 1, 0, 0.",
    },
    Savoir {
        cle: "fibonacci",
        sert: "Produire la suite 0, 1, 1, 2, 3, 5, 8, 13…, où chaque terme est la somme des deux précédents.",
        marche: "Deux registres gardent les deux derniers termes ; à chaque front, le second passe dans le premier et la somme prend sa place.",
        place: "Un petit programme fait en matériel. L'ordinateur miniature sait aussi la calculer en logiciel : compare les deux façons.",
        vraie_vie: "On trouve la suite de Fibonacci dans les spirales des tournesols et des pommes de pin.",
        anecdote: "Fibonacci l'a présentée en 1202 avec un problème de lapins. Le rapport de deux termes voisins s'approche du nombre d'or, environ 1,618.",
        essaie: "Fais 14 fronts après raz : le terme 377 dépasse 255, et la suite repart modulo 256.",
    },
    Savoir {
        cle: "memoire-64",
        sert: "Garder 64 octets, quatre fois plus que la mémoire de l'ordinateur miniature.",
        marche: "Les bits hauts de l'adresse choisissent une des quatre mémoires de 16 octets ; seule celle-là reçoit l'ordre d'écrire, et seul son contenu ressort par les multiplexeurs.",
        place: "C'est ainsi qu'on assemble les barrettes de mémoire : des puces identiques, et un décodeur d'adresse qui les sépare.",
        vraie_vie: "Les barrettes de mémoire assemblent plusieurs puces identiques, comme tes quatre mémoires.",
        anecdote: "Le Commodore 64 (1982), l'ordinateur le plus vendu de son époque, avait 64 Ko : mille fois ta mémoire.",
        essaie: "Écris à l'adresse 5 et à l'adresse 21 : les deux bits hauts diffèrent, les cases aussi.",
    },
    Savoir {
        cle: "memoire-256",
        sert: "Garder 256 octets : toutes les adresses qu'un opérande de 8 bits peut désigner.",
        marche: "Le même montage, un étage plus haut : quatre mémoires de 64 octets et deux bits d'adresse de plus. Chaque étage multiplie la taille par quatre.",
        place: "La plus grande mémoire que le processeur 8 bits sait adresser directement. Au-delà, les vrais ordinateurs ajoutent des bits d'adresse… et encore des étages.",
        vraie_vie: "La console Atari 2600 n'avait que 128 octets de mémoire vive : ta mémoire en a deux fois plus.",
        anecdote: "Plus de 20 000 portes NAND pour 256 octets : une seule barrette moderne en garde des milliards de fois plus.",
        essaie: "Écris aux adresses 0 et 64 : la mémoire 0 et la mémoire 1 reçoivent chacune une valeur.",
    },
    Savoir {
        cle: "ordinateur-256",
        sert: "Faire tourner des programmes qui ont besoin de beaucoup de place pour leurs données.",
        marche: "Le processeur ne change pas : il envoyait déjà 8 bits d'adresse, dont on ne gardait que 4. Brancher les 8 suffit à profiter des 256 cases.",
        place: "Un ordinateur plus grand que l'ordinateur miniature, construit autour de ta propre mémoire.",
        vraie_vie: "Plus de mémoire, ce sont des programmes plus longs et des données plus riches : c'est toute l'histoire de l'informatique.",
        anecdote: "Léonard de Vinci écrivait ses carnets en miroir, de droite à gauche. Ton ordinateur relit les touches à l'envers.",
        essaie: "Branche une mémoire de 16 octets à la place : le texte rendu est faux, des touches se sont écrasées.",
    },
    Savoir {
        cle: "messagerie",
        sert: "Faire passer des messages d'un ordinateur à un autre.",
        marche: "A écrit chaque lettre à sa sortie ; un registre la retient au moment de l'écriture, et B la lit comme si on l'avait tapée à son clavier. Un 0 envoyé juste après vide la boîte, pour que deux lettres pareilles se distinguent.",
        place: "Deux ordinateurs entiers qui travaillent ensemble : un réseau, en tout petit. La branche réseau remplace la boîte aux lettres par un seul fil.",
        vraie_vie: "Deux ordinateurs qui s'envoient des octets : c'est le principe de tout réseau, d'Internet aux consoles en ligne.",
        anecdote: "En 1969, le premier message envoyé sur ARPANET, l'ancêtre d'Internet, devait être « LOGIN ». Le système a planté après « LO ».",
        essaie: "Tape deux fois la même lettre : la boîte se vide entre les deux, B les reçoit bien toutes les deux.",
    },
    Savoir {
        cle: "messagerie-chiffree",
        sert: "Envoyer un message que personne d'autre ne peut lire en route.",
        marche: "Le programme de A chiffre avant d'envoyer, celui de B déchiffre en recevant. Le matériel est le même que la messagerie : seul le logiciel a changé.",
        place: "Le grand élément des grands projets. HTTPS, les messageries chiffrées et les cartes bancaires suivent la même idée, avec des chiffres bien plus solides que César.",
        vraie_vie: "Sur une messagerie chiffrée, ton téléphone fait le travail de A, et celui de ton ami le travail de B : entre les deux, personne ne peut lire.",
        anecdote: "Dans le film « 2001, l'Odyssée de l'espace », l'ordinateur s'appelle HAL. Décale chaque lettre d'un rang : tu obtiens IBM.",
        essaie: "Branche le clavier directement sur B : il déchiffre un message qui n'était pas chiffré, et affiche n'importe quoi.",
    },
    Savoir {
        cle: "panne-ou-x",
        sert: "Apprendre à trouver une erreur dans un circuit qu'on n'a pas construit soi-même.",
        marche: "On essaie chaque cas et on note ceux qui sont faux. Ensuite on suit le courant depuis la lampe : la première porte dont la sortie ne correspond pas à ses entrées, ou qui reçoit le mauvais signal, est la coupable.",
        place: "Avant de livrer une puce, on la teste sur toutes sortes d'entrées. Quand un test échoue, il faut retrouver la porte fautive parmi des millions.",
        vraie_vie: "Un électricien qui cherche pourquoi une prise ne marche plus fait la même chose : il teste, puis remonte le câble jusqu'à la panne.",
        anecdote: "En 1947, une vraie mite coincée dans un relais a arrêté l'ordinateur Mark II. On l'a collée dans le cahier de bord : le premier « bug » trouvé.",
        essaie: "Une fois réparé, débranche exprès un autre fil et regarde quels cas deviennent faux : chaque panne laisse une empreinte différente.",
    },
    Savoir {
        cle: "panne-demi-additionneur",
        sert: "S'entraîner à lire les symptômes : quelle sortie se trompe, et dans quels cas.",
        marche: "Si seule la retenue est fausse, la panne est sur le chemin de la retenue ; si seule la somme se trompe, elle est sur celui de la somme. Les portes partagées par les deux sont suspectes quand les deux se trompent.",
        place: "Les testeurs de puces comparent les sorties à celles attendues, puis déduisent la zone du circuit en cause avant même d'ouvrir le plan.",
        vraie_vie: "Un médecin fait pareil : les symptômes disent quel organe examiner en premier.",
        anecdote: "Les puces modernes contiennent des circuits de test cachés, les « chaînes de scan », juste pour pouvoir lire l'état de chaque bascule pendant le dépannage.",
        essaie: "Avant de réparer, écris la table de vérité fausse : d'après elle seule, peux-tu deviner la porte en cause ?",
    },
    Savoir {
        cle: "panne-multiplexeur",
        sert: "Apprendre la méthode qui consiste à fixer une entrée pour en étudier une autre.",
        marche: "Avec sel fixé, le multiplexeur ne devrait écouter qu'une entrée. Si la lampe bouge quand on change l'autre, ou ne suit plus la bonne, on sait quelle moitié du circuit regarder.",
        place: "Dans un processeur, on isole de la même façon un bloc à la fois : on fixe tout le reste et on observe ce qui change.",
        vraie_vie: "Pour trouver quelle ampoule d'une guirlande est grillée, on teste une moitié, puis la moitié de la moitié.",
        anecdote: "Chercher en coupant en deux à chaque fois s'appelle la dichotomie. Avec 20 essais, on trouve une panne parmi un million de portes.",
        essaie: "Compte combien d'essais il te faut pour trouver la panne, puis cherche une façon d'en faire moins.",
    },
    Savoir {
        cle: "panne-additionneur-complet",
        sert: "Dépanner un circuit plus grand, où chercher au hasard ne suffit plus.",
        marche: "La table de vérité (Ctrl+E) montre d'un coup les lignes fausses. On compare avec une addition juste, puis on suit les deux demi-additionneurs : celui qui donne un résultat faux contient la panne.",
        place: "Les outils de conception de puces comparent automatiquement le circuit à sa description : c'est la « vérification d'équivalence ».",
        vraie_vie: "Les garagistes branchent une valise de diagnostic sur la voiture : elle lit les codes d'erreur et dit quel morceau tester d'abord.",
        anecdote: "En 1994, une petite erreur dans une table du processeur Pentium donnait des divisions fausses dans de rares cas. Intel a dû remplacer les puces : près de 475 millions de dollars.",
        essaie: "Une fois réparé, vérifie que ton additionneur complet donne bien 1 + 1 + 1 = 11 en binaire.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaque_quete_a_sa_fiche_et_chaque_chapitre_sa_presentation() {
        for f in PARCOURS {
            let s = savoir(f.cle).unwrap_or_else(|| panic!("{} : pas de fiche", f.cle));
            for texte in [s.sert, s.marche, s.place, s.vraie_vie, s.anecdote, s.essaie] {
                assert!(!texte.trim().is_empty(), "{} : texte vide", f.cle);
            }
            assert!(
                chapitre(f.chapitre).is_some(),
                "{} : chapitre inconnu",
                f.chapitre
            );
        }
        assert_eq!(SAVOIRS.len(), PARCOURS.len(), "une fiche sans quête");
        // Les chapitres sont dans l'ordre du parcours, et chacun y apparaît.
        let mut ordre: Vec<&str> = Vec::new();
        for f in PARCOURS {
            if ordre.last() != Some(&f.chapitre) {
                ordre.push(f.chapitre);
            }
        }
        let noms: Vec<&str> = CHAPITRES.iter().map(|c| c.nom).collect();
        assert_eq!(ordre, noms);
    }

    #[test]
    fn la_fin_d_un_chapitre_s_annonce_selon_sa_place() {
        assert_eq!(fin_de_chapitre("Calcul").0, "Chapitre terminé : Calcul");
        assert_eq!(fin_de_chapitre("Calcul").1, "Prochain chapitre : Mémoire");
        assert_eq!(
            fin_de_chapitre("Ordinateurs").0,
            "Parcours principal terminé !"
        );
        assert_eq!(
            fin_de_chapitre("Le réseau").0,
            "Branche terminée : la carte réseau !"
        );
        assert_eq!(
            fin_de_chapitre("Nombres remarquables").1,
            "Prochain chapitre : Maths et automates"
        );
        assert_eq!(
            fin_de_chapitre("Ordinateurs en réseau").0,
            "Branche terminée : les grands projets !"
        );
        assert_eq!(
            quetes_du_chapitre("Portes logiques"),
            (0..7).collect::<Vec<_>>()
        );
    }

    #[test]
    fn la_branche_du_reseau_mene_a_la_carte_reseau() {
        let reseau: Vec<&str> = PARCOURS
            .iter()
            .filter(|f| chapitre(f.chapitre).map(|c| c.branche) == Some(Branche::Reseau))
            .map(|f| f.cle)
            .collect();
        assert_eq!(reseau.last(), Some(&"carte-reseau"));
        assert!(reseau.len() >= 5);
        // Chaque branche mène à son grand élément.
        for (branche, fin) in [
            (Branche::Montre, "montre"),
            (Branche::Ecran, "controleur-ecran"),
            (Branche::Maths, "pgcd"),
            (Branche::Codes, "hamming-correcteur"),
            (Branche::Jeux, "meilleur-score"),
            (Branche::Grands, "messagerie-chiffree"),
        ] {
            let derniere = PARCOURS
                .iter()
                .rfind(|f| chapitre(f.chapitre).map(|c| c.branche) == Some(branche))
                .map(|f| f.cle);
            assert_eq!(derniere, Some(fin));
        }
    }
}
