//! Les quêtes : un parcours qui mène de la porte NAND à l'ordinateur complet.
//!
//! Chaque quête demande de construire un composant dont le corrigé existe déjà dans
//! `ce-circuit` : la bibliothèque de référence, vérifiée contre sa définition mathématique,
//! et les composants de l'ordinateur miniature. « Vérifier » fait tourner côte à côte le
//! circuit de l'utilisateur et le corrigé, sur les mêmes essais, et compare leurs sorties.
//! Aucune table de vérité n'est donc écrite à la main ici : le corrigé fait foi.
//!
//! Les petites quêtes se valident aussi cas par cas, en cliquant sur les interrupteurs : la
//! table de la quête se remplit au fur et à mesure.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use ce_circuit::ordinateur::{self, Reglages};
use ce_circuit::{Banc, Bibliotheque, Sens};
use ce_moteur::Valeur;

use crate::brique::Projet;
use crate::document::{Document, Element, Fil, Genre, Id, Point};
use crate::pont::{Briques, Pont};
use crate::simulation::Session;

/// Comment une quête se vérifie.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Epreuve {
    /// Les sorties ne dépendent que des entrées du moment : toutes les combinaisons sont
    /// essayées quand il y en a peu, sinon un large échantillon.
    Combinatoire,
    /// Le circuit a une mémoire : une longue suite d'actions, une entrée à la fois, est jouée
    /// sur les deux circuits depuis la mise sous tension.
    Sequentielle,
    /// Un ordinateur complet : on tape `touches` au clavier, l'écran doit afficher `ecran`.
    Ordinateur {
        /// Le programme de la mémoire morte, en assembleur.
        programme: &'static str,
        /// Ce qu'on tape.
        touches: &'static str,
        /// Ce que l'écran doit afficher.
        ecran: &'static str,
        /// Vrai si seuls la mémoire morte et les appareils sont fournis : le processeur et la
        /// mémoire vive viennent des briques gagnées.
        complet: bool,
    },
    /// Un grand projet, plus grand qu'un ordinateur : l'atelier fournit ses grandes pièces,
    /// on tape `touches` au clavier, l'écran doit afficher `ecran`.
    Systeme {
        /// Ce que l'atelier fournit.
        atelier: Atelier,
        /// Ce qu'on tape.
        touches: &'static str,
        /// Ce que l'écran doit afficher.
        ecran: &'static str,
    },
}

/// L'atelier d'un grand projet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Atelier {
    /// Deux ordinateurs, A et B, chacun avec son programme : A lit le clavier, B tient
    /// l'écran, et c'est au joueur de les relier.
    Deux {
        /// Le programme de l'ordinateur A, en assembleur.
        envoi: &'static str,
        /// Le programme de l'ordinateur B.
        reception: &'static str,
    },
    /// Le processeur et la mémoire morte : la mémoire vive de 256 octets vient des briques.
    GrandeMemoire {
        /// Le programme de la mémoire morte.
        programme: &'static str,
    },
}

impl Epreuve {
    /// Vrai pour un ordinateur complet, ou un grand projet fait d'ordinateurs.
    pub fn ordinateur(self) -> bool {
        matches!(self, Epreuve::Ordinateur { .. } | Epreuve::Systeme { .. })
    }
}

/// Une étape du parcours, telle qu'on la décrit.
#[derive(Clone, Copy, Debug)]
pub struct Fiche {
    /// Clé enregistrée dans le projet pour retrouver la quête.
    pub cle: &'static str,
    /// Le chapitre du parcours.
    pub chapitre: &'static str,
    /// Titre.
    pub titre: &'static str,
    /// Ce qu'il faut construire, avec un indice.
    pub consigne: &'static str,
    /// Texte affiché une fois la quête réussie (vide : un texte tout fait).
    pub felicitations: &'static str,
    /// Le composant corrigé (vide pour l'ordinateur).
    pub corrige: &'static str,
    /// Nom de la brique gagnée en réussissant.
    pub brique: &'static str,
    /// Comment la quête se vérifie.
    pub epreuve: Epreuve,
    /// Broches du corrigé qui portent un autre nom dans le plan.
    pub renommer: &'static [(&'static str, &'static str)],
    /// Si la vitesse compte : nombre maximal d'étapes entre une entrée et une sortie.
    pub profondeur_max: Option<u64>,
}

/// Le parcours, dans l'ordre conseillé.
pub const PARCOURS: &[Fiche] = &[
    Fiche {
        cle: "non",
        chapitre: "Portes logiques",
        titre: "La porte NON",
        consigne: "s doit valoir l'inverse de a. Une seule NAND suffit : relie a à ses deux entrées.",
        felicitations: "Ta porte NON marche. Elle rejoint tes briques : tu peux la poser dans les quêtes suivantes.",
        corrige: "NON",
        brique: "NON",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "et",
        chapitre: "Portes logiques",
        titre: "La porte ET",
        consigne: "s vaut 1 seulement quand a et b valent 1. Indice : une NAND, puis un NON.",
        felicitations: "",
        corrige: "ET",
        brique: "ET",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "ou",
        chapitre: "Portes logiques",
        titre: "La porte OU",
        consigne: "s vaut 1 dès que a ou b vaut 1. Indice : inverse a et b, puis une NAND.",
        felicitations: "",
        corrige: "OU",
        brique: "OU",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "non-ou",
        chapitre: "Portes logiques",
        titre: "La porte NON-OU",
        consigne: "s vaut 1 seulement quand a et b valent 0 : c'est l'inverse du OU.",
        felicitations: "",
        corrige: "NON_OU",
        brique: "NON_OU",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "ou-exclusif",
        chapitre: "Portes logiques",
        titre: "Le OU exclusif",
        consigne: "s vaut 1 quand a et b sont différents. Quatre NAND suffisent : la première reçoit a et b, les deux suivantes la combinent avec chaque entrée.",
        felicitations: "Le OU exclusif marche : c'est le cœur de l'addition.",
        corrige: "OU_X",
        brique: "OU_X",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "equivalence",
        chapitre: "Portes logiques",
        titre: "L'équivalence",
        consigne: "s vaut 1 quand a et b sont égaux : l'inverse du OU exclusif.",
        felicitations: "",
        corrige: "EQUIV",
        brique: "EQUIV",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "majorite",
        chapitre: "Portes logiques",
        titre: "Le vote à la majorité",
        consigne: "s vaut 1 quand au moins deux des trois entrées a, b, c valent 1. Indice : une NAND par paire, puis on les combine.",
        felicitations: "",
        corrige: "MAJORITE",
        brique: "MAJORITE",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "multiplexeur",
        chapitre: "Aiguillages et codes",
        titre: "Le multiplexeur",
        consigne: "sel choisit : s recopie a quand sel vaut 0, et b quand sel vaut 1. Indice : un NON sur sel et trois NAND.",
        felicitations: "Le multiplexeur marche : c'est l'aiguillage des signaux.",
        corrige: "MUX",
        brique: "MUX",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "demultiplexeur",
        chapitre: "Aiguillages et codes",
        titre: "Le démultiplexeur",
        consigne: "L'inverse de l'aiguillage : e part sur s0 quand sel vaut 0, sur s1 quand sel vaut 1 ; l'autre sortie reste à 0.",
        felicitations: "",
        corrige: "DEMUX",
        brique: "DEMUX",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "multiplexeur-4",
        chapitre: "Aiguillages et codes",
        titre: "Le multiplexeur à 4 voies",
        consigne: "s recopie le bit de d choisi par sel (de 0 à 3). d (4 bits) et sel (2 bits) sont des bus : branche chacun sur un découpeur (palette, à côté de la jonction) pour avoir un fil par bit. Puis trois multiplexeurs à deux voies : deux choisissent avec sel[0], le dernier avec sel[1].",
        felicitations: "",
        corrige: "MUX4",
        brique: "MUX4",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "decodeur-2",
        chapitre: "Aiguillages et codes",
        titre: "Le décodeur 2 vers 4",
        consigne: "Une seule des quatre sorties s'allume : s[a]. C'est ainsi qu'une adresse choisit une case de mémoire.",
        felicitations: "",
        corrige: "DECODEUR2",
        brique: "DECODEUR2",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "decodeur-3",
        chapitre: "Aiguillages et codes",
        titre: "Le décodeur 3 vers 8",
        consigne: "s[a] vaut 1, les sept autres 0. Indice : un décodeur 2 vers 4 pour les deux bits du bas, et a[2] choisit la moitié.",
        felicitations: "",
        corrige: "DECODEUR3",
        brique: "DECODEUR3",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "encodeur",
        chapitre: "Aiguillages et codes",
        titre: "L'encodeur à priorité",
        consigne: "n donne le rang du plus grand bit de d à 1, et valide dit s'il y en a au moins un. C'est ce qui choisit l'interruption la plus urgente.",
        felicitations: "",
        corrige: "ENCODEUR4",
        brique: "ENCODEUR4",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "sept-segments",
        chapitre: "Aiguillages et codes",
        titre: "L'afficheur 7 segments",
        consigne: "Pour chaque chiffre hexadécimal n (0 à F), allume les bons segments : seg[0] = a en haut, puis b, c, d, e, f dans le sens des aiguilles d'une montre, et seg[6] = g au milieu. 0 allume a b c d e f ; 1 allume b c ; 8 allume tout.",
        felicitations: "Tes chiffres s'affichent comme sur une calculatrice.",
        corrige: "SEPT_SEGMENTS",
        brique: "SEPT_SEGMENTS",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "parite",
        chapitre: "Aiguillages et codes",
        titre: "Le bit de parité",
        consigne: "p vaut 1 quand l'octet a contient un nombre impair de 1. Une chaîne de OU exclusifs ; en arbre, c'est plus rapide.",
        felicitations: "",
        corrige: "PARITE8",
        brique: "PARITE8",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "demi-additionneur",
        chapitre: "Calcul",
        titre: "Le demi-additionneur",
        consigne: "s donne la somme de a et b, c la retenue. Essaie les quatre cas en cliquant sur a et b.",
        felicitations: "Les quatre cas sont justes. Le demi-additionneur rejoint tes briques, prêt pour l'additionneur complet.",
        corrige: "DEMI_ADD",
        brique: "DEMI_ADD",
        epreuve: Epreuve::Combinatoire,
        renommer: &[("r", "c")],
        profondeur_max: None,
    },
    Fiche {
        cle: "additionneur-complet",
        chapitre: "Calcul",
        titre: "L'additionneur complet",
        consigne: "Additionne a, b et la retenue qui entre, re : s est la somme, rs la retenue qui sort. Deux demi-additionneurs et un OU.",
        felicitations: "",
        corrige: "ADD_COMPLET",
        brique: "ADD_COMPLET",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "additionneur-4-bits",
        chapitre: "Calcul",
        titre: "L'additionneur 4 bits",
        consigne: "Additionne deux nombres de 4 bits et la retenue re : s reçoit la somme, rs la retenue finale. Quatre additionneurs complets en chaîne ; les découpeurs séparent les bits, une jonction les regroupe.",
        felicitations: "",
        corrige: "ADD4",
        brique: "ADD4",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "additionneur-8-bits",
        chapitre: "Calcul",
        titre: "L'additionneur 8 bits",
        consigne: "La même chose sur deux octets : s reçoit les 8 bits de la somme, rs la retenue finale.",
        felicitations: "Tu additionnes des octets : tout le calcul d'un processeur part de là.",
        corrige: "ADD8",
        brique: "ADD8",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "additionneur-rapide",
        chapitre: "Calcul",
        titre: "L'additionneur rapide",
        consigne: "Additionne deux octets, mais vite : la retenue ne doit pas traverser les huit étages un par un. Calcule d'avance la moitié haute pour les deux retenues possibles, et laisse la retenue du bas choisir.",
        felicitations: "Ton additionneur répond plus vite que la chaîne : c'est la retenue sélectionnée.",
        corrige: "ADD8_RAPIDE",
        brique: "ADD8_RAPIDE",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: Some(19),
    },
    Fiche {
        cle: "egalite",
        chapitre: "Calcul",
        titre: "Le test d'égalité",
        consigne: "egal vaut 1 quand les nombres a et b de 4 bits sont égaux : bit à bit, puis tous ensemble.",
        felicitations: "",
        corrige: "EGAL4",
        brique: "EGAL4",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "comparateur",
        chapitre: "Calcul",
        titre: "Le comparateur",
        consigne: "inf, egal et sup disent si a est plus petit, égal ou plus grand que b (4 bits, sans signe). Indice : a − b emprunte exactement quand a < b.",
        felicitations: "",
        corrige: "COMPARATEUR4",
        brique: "COMPARATEUR4",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "soustracteur",
        chapitre: "Calcul",
        titre: "Le soustracteur",
        consigne: "d = a − b sur 4 bits (en revenant à 15 après 0), emprunt = 1 quand a < b. Indice : a − b = a + (non b) + 1.",
        felicitations: "",
        corrige: "SOUSTRACTEUR4",
        brique: "SOUSTRACTEUR4",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "incrementeur",
        chapitre: "Calcul",
        titre: "L'incrémenteur",
        consigne: "s = e + 1 sur 8 bits (255 + 1 donne 0). C'est ce qui fait avancer le compteur de programme.",
        felicitations: "",
        corrige: "INCR8",
        brique: "INCR8",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "negatif",
        chapitre: "Calcul",
        titre: "Le complément à deux",
        consigne: "s = −a, codé comme le font les processeurs : on inverse tous les bits, puis on ajoute 1.",
        felicitations: "",
        corrige: "NEGATIF8",
        brique: "NEGATIF8",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "decaleur",
        chapitre: "Calcul",
        titre: "Le décaleur",
        consigne: "s = a décalé de n rangs (0 à 7) : vers les poids forts si droite vaut 0, vers les poids faibles sinon ; des 0 entrent. Indice : trois étages de multiplexeurs, qui décalent de 1, 2 puis 4.",
        felicitations: "",
        corrige: "DECALEUR8",
        brique: "DECALEUR8",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "multiplicateur",
        chapitre: "Calcul",
        titre: "Le multiplicateur",
        consigne: "p = a × b pour deux nombres de 4 bits. Comme à la main : chaque bit de b donne une ligne a ET b[j], décalée, et des additionneurs les cumulent.",
        felicitations: "Ton circuit multiplie : 15 × 15 = 225, en portes NAND.",
        corrige: "MULT4",
        brique: "MULT4",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "diviseur",
        chapitre: "Calcul",
        titre: "Le diviseur",
        consigne: "a = q × b + r avec r < b. Comme une division posée : on abaisse un bit de a à la fois et on soustrait b quand c'est possible. Si b = 0, q vaut 15 et r vaut a.",
        felicitations: "",
        corrige: "DIVISEUR4",
        brique: "DIVISEUR4",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "unite-de-calcul",
        chapitre: "Calcul",
        titre: "L'unité de calcul",
        consigne: "Selon op, r vaut b (op = 0), a + b (op = 1), a − b (op = 2) ou non (a et b) bit à bit (op = 3) ; rs est la retenue de l'addition. Pour soustraire : inverser b et ajouter 1 par la retenue.",
        felicitations: "Ton unité de calcul sait compter et comparer : c'est le cœur du processeur.",
        corrige: "UAL",
        brique: "UAL",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "verrou-sr",
        chapitre: "Mémoire",
        titre: "Le verrou SR",
        consigne: "Un premier bit de mémoire. s_n à 0 met q à 1, r_n à 0 le remet à 0, et les deux à 1 il garde sa valeur. q_n est l'inverse de q. Deux NAND croisées : la sortie de chacune entre dans l'autre.",
        felicitations: "Ton circuit se souvient : c'est le premier bit de mémoire.",
        corrige: "VERROU_SR",
        brique: "VERROU_SR",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "verrou-d",
        chapitre: "Mémoire",
        titre: "Le verrou D",
        consigne: "Quand e vaut 1, q recopie d ; quand e vaut 0, q garde la dernière valeur. Un verrou SR, commandé par deux NAND qui ne laissent passer d que si e vaut 1.",
        felicitations: "",
        corrige: "VERROU_D",
        brique: "VERROU_D",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "bascule-d",
        chapitre: "Mémoire",
        titre: "La bascule D",
        consigne: "q prend la valeur de d au moment précis où clk passe de 0 à 1, et la garde le reste du temps. Deux verrous D à la suite, le premier ouvert quand clk vaut 0, le second quand clk vaut 1.",
        felicitations: "La bascule D marche : c'est la cellule de base des registres et de la mémoire.",
        corrige: "BASCULE_D",
        brique: "BASCULE_D",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "bascule-t",
        chapitre: "Mémoire",
        titre: "La bascule T",
        consigne: "Au front montant de clk, q s'inverse si t vaut 1 et reste sinon ; raz = 1 le remet à 0. Une bascule D dont l'entrée est q OU exclusif t.",
        felicitations: "",
        corrige: "BASCULE_T",
        brique: "BASCULE_T",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "bascule-jk",
        chapitre: "Mémoire",
        titre: "La bascule JK",
        consigne: "Au front montant de clk : j = 1 met q à 1, k = 1 le met à 0, les deux à 1 l'inversent, les deux à 0 le gardent ; raz = 1 le remet à 0.",
        felicitations: "",
        corrige: "BASCULE_JK",
        brique: "BASCULE_JK",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "registre-8-bits",
        chapitre: "Mémoire",
        titre: "Le registre 8 bits",
        consigne: "Au front montant de clk, si charge vaut 1, q prend la valeur de d ; sinon q garde la sienne. Huit bascules D, chacune précédée d'un multiplexeur qui choisit entre q et d.",
        felicitations: "Ton registre garde un octet : le processeur en a plusieurs.",
        corrige: "REGISTRE8",
        brique: "REGISTRE8",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "registre-a-decalage",
        chapitre: "Mémoire",
        titre: "Le registre à décalage",
        consigne: "À chaque front montant de clk, les bits de q avancent d'un rang et e entre en q[0]. Huit bascules D en file indienne : c'est ainsi qu'on envoie un octet sur un seul fil.",
        felicitations: "",
        corrige: "DECALAGE8",
        brique: "DECALAGE8",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "banc-de-registres",
        chapitre: "Mémoire",
        titre: "Le banc de registres",
        consigne: "Quatre registres de 8 bits. a montre le registre ad_a, b le registre ad_b (on lit deux registres à la fois) ; au front montant de clk, si ecrire vaut 1, le registre ad_e prend d.",
        felicitations: "Ton banc de registres est celui d'un vrai processeur RISC, en plus petit.",
        corrige: "BANC_REGISTRES",
        brique: "BANC_REGISTRES",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "memoire-vive",
        chapitre: "Mémoire",
        titre: "La mémoire vive",
        consigne: "Quatre cases d'un octet. q montre toujours la case choisie par adresse ; au front montant de clk, si ecrire vaut 1, cette case prend la valeur de d. Quatre registres, un décodeur pour charger le bon, des multiplexeurs pour lire.",
        felicitations: "Ta mémoire vive retient quatre octets.",
        corrige: "RAM4",
        brique: "RAM4",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "memoire-vive-16",
        chapitre: "Mémoire",
        titre: "La mémoire de 16 octets",
        consigne: "La même chose avec 16 cases et 4 bits d'adresse. Indice : deux mémoires de 8 octets, ou quatre de 4 ; le bit d'adresse du haut choisit laquelle.",
        felicitations: "Ta mémoire de 16 octets est assez grande pour l'ordinateur.",
        corrige: "RAM16",
        brique: "RAM16",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "pile",
        chapitre: "Mémoire",
        titre: "La pile",
        consigne: "Quatre octets empilés. Au front montant de clk : empiler pose d au sommet (tout descend d'un niveau), dépiler retire le sommet (tout remonte, des 0 entrent en bas) ; les deux à la fois ne font rien. vide et plein comptent les niveaux occupés ; raz vide la pile sans effacer les octets.",
        felicitations: "Ta pile est celle qui retient les adresses de retour des sous-programmes.",
        corrige: "PILE",
        brique: "PILE",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "file",
        chapitre: "Mémoire",
        titre: "La file d'attente",
        consigne: "Quatre octets, le premier entré sort le premier. sortie montre le plus ancien. Au front montant de clk : ecrire ajoute d derrière s'il reste de la place, lire retire le plus ancien (les autres avancent) s'il y en a un ; les deux à la fois ne font rien. raz vide la file.",
        felicitations: "",
        corrige: "FILE",
        brique: "FILE",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "compteur-4-bits",
        chapitre: "Compter et cadencer",
        titre: "Le compteur 4 bits",
        consigne: "À chaque front montant de clk, q augmente de 1 (après 15 vient 0). Si raz vaut 1 au front, q revient à 0. Quatre bascules D et de quoi ajouter 1.",
        felicitations: "Ton compteur avance au rythme de l'horloge.",
        corrige: "COMPTEUR4",
        brique: "COMPTEUR4",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "compteur-decimal",
        chapitre: "Compter et cadencer",
        titre: "Le compteur décimal",
        consigne: "q compte de 0 à 9 puis revient à 0, un pas par front montant de clk ; fin vaut 1 quand q vaut 9 ; raz le remet à 0.",
        felicitations: "",
        corrige: "COMPTEUR_BCD",
        brique: "COMPTEUR_BCD",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "compteur-haut-bas",
        chapitre: "Compter et cadencer",
        titre: "Le compteur qui monte et descend",
        consigne: "À chaque front montant de clk, q augmente de 1 si haut vaut 1 et diminue de 1 sinon (sur 4 bits, 0 − 1 donne 15) ; raz le remet à 0. Indice : q − 1 = q + 15.",
        felicitations: "",
        corrige: "COMPTEUR_HB4",
        brique: "COMPTEUR_HB4",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "generateur-aleatoire",
        chapitre: "Compter et cadencer",
        titre: "Le générateur pseudo-aléatoire",
        consigne: "Un registre à décalage qui se nourrit de lui-même : à chaque front, q avance d'un rang et q[0] reçoit q[7] ⊕ q[5] ⊕ q[4] ⊕ q[3]. raz met q à 1. Il parcourt les 255 valeurs non nulles dans un ordre qui a l'air dû au hasard.",
        felicitations: "",
        corrige: "LFSR8",
        brique: "LFSR8",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "minuterie",
        chapitre: "Compter et cadencer",
        titre: "La minuterie",
        consigne: "Au front montant de clk : charger = 1 met q à n, sinon q descend de 1 jusqu'à 0 et y reste. fini vaut 1 quand q vaut 0.",
        felicitations: "",
        corrige: "MINUTERIE",
        brique: "MINUTERIE",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "feu-tricolore",
        chapitre: "Compter et cadencer",
        titre: "Le feu tricolore",
        consigne: "Un compteur de 0 à 7 avance à chaque front de clk : vert de 0 à 2, orange à 3, rouge de 4 à 7 ; raz revient au début du vert. C'est un automate : un compteur et un peu de décodage.",
        felicitations: "Ton feu tricolore tourne tout seul.",
        corrige: "FEU",
        brique: "FEU",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "chronometre",
        chapitre: "Compter et cadencer",
        titre: "Le chronomètre",
        consigne: "Les secondes de 00 à 59 : u (unités, de 0 à 9) avance à chaque front de clk ; d (dizaines, de 0 à 5) avance quand u repasse de 9 à 0 ; après 59 vient 00. raz remet tout à 0. Deux compteurs décimaux reliés.",
        felicitations: "Ton chronomètre compte les secondes comme une montre.",
        corrige: "CHRONOMETRE",
        brique: "CHRONOMETRE",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "liaison-serie",
        chapitre: "Communiquer",
        titre: "La liaison série",
        consigne: "Envoie un octet sur un seul fil, tx, qui vaut 1 au repos. Au front montant de clk, si envoyer vaut 1 et que rien n'est en cours, la trame part, un bit par front : un 0 de départ, d[0] à d[7], un 1 d'arrêt. occupe vaut 1 pendant l'envoi ; raz arrête tout.",
        felicitations: "Ton émetteur parle comme un vrai port série.",
        corrige: "EMETTEUR_SERIE",
        brique: "EMETTEUR_SERIE",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "interruptions",
        chapitre: "Communiquer",
        titre: "Le contrôleur d'interruptions",
        consigne: "Chaque demande irq[i] reste en attente jusqu'à son acquittement. int vaut 1 s'il y en a une, num donne la plus urgente (le plus grand i). Au front montant de clk, ack = 1 acquitte num ; raz efface tout.",
        felicitations: "",
        corrige: "INTERRUPTIONS",
        brique: "INTERRUPTIONS",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "compteur-de-programme",
        chapitre: "Le processeur",
        titre: "Le compteur de programme",
        consigne: "Le pointeur d'instruction. Au front montant de clk : nraz = 0 le remet à 0 ; sinon gel = 1 le garde ; sinon saut = 1 le met à cible ; sinon il avance de 1.",
        felicitations: "",
        corrige: "COMPTEUR_PROGRAMME",
        brique: "COMPTEUR_PROGRAMME",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "decodeur-d-instructions",
        chapitre: "Le processeur",
        titre: "Le décodeur d'instructions",
        consigne: "Traduit le code o de l'instruction (et les drapeaux z et c pour les sauts conditionnels) en commandes pour le reste du processeur. Ouvre la brique DECODEUR de l'ordinateur miniature pour voir le jeu d'instructions.",
        felicitations: "",
        corrige: "DECODEUR",
        brique: "DECODEUR",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "processeur",
        chapitre: "Le processeur",
        titre: "Le processeur",
        consigne: "Le processeur 8 bits complet : instr arrive de la mémoire morte, lu de la mémoire vive, clavier du clavier ; il sort pc (l'instruction suivante), a (l'accumulateur), ecrit_ram, sortie et arret. Assemble le décodeur, l'unité de calcul, l'accumulateur et le compteur de programme ; ouvre le processeur de l'ordinateur miniature pour t'en inspirer.",
        felicitations: "Ton processeur exécute les mêmes instructions que celui de l'ordinateur miniature.",
        corrige: "PROCESSEUR",
        brique: "PROCESSEUR",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "ordinateur",
        chapitre: "Ordinateurs",
        titre: "L'ordinateur calculatrice",
        consigne: "Le processeur, la mémoire morte (qui contient le programme) et la mémoire vive sont posés : branche-les. L'horloge va à cpu.clk et ram.clk ; raz passe par un NON vers cpu.nraz ; cpu.pc va à rom.a ; rom.d va à cpu.instr, et ses 4 bits bas à ram.adresse ; cpu.a va à ram.d et aux données de l'écran ; cpu.ecrit_ram à ram.ecrire ; ram.q à cpu.lu ; le clavier à cpu.clavier ; enfin cpu.sortie ET l'horloge inversée commandent l'écran. Réussi quand, après 6 et 7 au clavier, l'écran affiche 6*7=42.",
        felicitations: "Ton ordinateur calcule : 3 401 portes NAND, de l'horloge à l'écran.",
        corrige: "",
        brique: "ORDINATEUR",
        epreuve: Epreuve::Ordinateur {
            programme: ordinateur::programmes::CALCULATRICE,
            touches: "67",
            ecran: "6*7=42",
            complet: false,
        },
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "ordinateur-majuscules",
        chapitre: "Ordinateurs",
        titre: "L'ordinateur qui écrit en majuscules",
        consigne: "Le même ordinateur, avec un autre programme dans la mémoire morte : il recopie à l'écran ce qu'on tape, les minuscules passées en majuscules. Branche-le comme le précédent. Réussi quand « Nand! » tapé au clavier s'affiche « NAND! ».",
        felicitations: "Même matériel, autre programme : c'est tout l'intérêt d'un ordinateur.",
        corrige: "",
        brique: "ORDINATEUR",
        epreuve: Epreuve::Ordinateur {
            programme: ordinateur::programmes::MAJUSCULES,
            touches: "Nand!",
            ecran: "NAND!",
            complet: false,
        },
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "ordinateur-a-toi",
        chapitre: "Ordinateurs",
        titre: "Ton ordinateur, de A à Z",
        consigne: "Cette fois, seuls la mémoire morte, l'horloge, le clavier et l'écran sont fournis : le processeur et la mémoire vive viennent de tes briques gagnées (fais d'abord les quêtes du processeur et de la mémoire de 16 octets). Branche tout comme pour l'ordinateur calculatrice. Réussi quand 6 puis 7 donnent 6*7=42.",
        felicitations: "Tout cet ordinateur vient de toi, porte après porte. Il est réutilisable comme base.",
        corrige: "",
        brique: "ORDINATEUR",
        epreuve: Epreuve::Ordinateur {
            programme: ordinateur::programmes::CALCULATRICE,
            touches: "67",
            ecran: "6*7=42",
            complet: true,
        },
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "detecteur-de-front",
        chapitre: "Le réseau",
        titre: "Le détecteur de front",
        consigne: "front vaut 1 quand e vaut 1 alors qu'il valait 0 au dernier front montant de clk : un appui long ne compte qu'une fois. Une bascule D garde l'ancien e ; front = e ET NON(ancien).",
        felicitations: "Ton détecteur de front transforme un appui en une seule impulsion.",
        corrige: "DETECTEUR_FRONT",
        brique: "DETECTEUR_FRONT",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "cadence",
        chapitre: "Le réseau",
        titre: "La cadence",
        consigne: "q compte de 0 à 7, un pas par front montant de clk, puis revient à 0 ; tic vaut 1 quand q vaut 7 ; raz remet q à 0. Trois bascules D, de quoi ajouter 1, et un ET des trois bits pour tic.",
        felicitations: "Ta cadence donne un tic tous les huit fronts : c'est le rythme d'une liaison plus lente que l'horloge.",
        corrige: "CADENCE",
        brique: "CADENCE",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "controle-de-parite",
        chapitre: "Le réseau",
        titre: "Le contrôle de parité",
        consigne: "ok vaut 1 quand l'octet d et le bit p contiennent ensemble un nombre pair de 1 : la trame est arrivée sans bit inversé. Ton bit de parité sur d, puis une équivalence avec p.",
        felicitations: "Ton contrôle repère tout octet arrivé avec un seul bit faux.",
        corrige: "CONTROLE_PARITE",
        brique: "CONTROLE_PARITE",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "recepteur-serie",
        chapitre: "Le réseau",
        titre: "Le récepteur série",
        consigne: "L'inverse de la liaison série. Au repos, un 0 sur rx au front montant de clk annonce une trame ; les 8 fronts suivants lisent d[0] à d[7] ; au 9e, le bit d'arrêt : s'il vaut 1, d prend l'octet et pret vaut 1 jusqu'au front suivant ; s'il vaut 0, c'est erreur qui vaut 1. raz arrête tout et met d à 0. Indice : neuf bascules qui se vident d'un rang par front comptent les bits, un registre à décalage les ramasse.",
        felicitations: "Ton récepteur comprend les trames de l'émetteur : la moitié d'une carte réseau.",
        corrige: "RECEPTEUR_SERIE",
        brique: "RECEPTEUR_SERIE",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "liaison-complete",
        chapitre: "Le réseau",
        titre: "Deux machines qui se parlent",
        consigne: "Relie un émetteur série et un récepteur série par un seul fil, sur la même horloge : envoyer = 1 fait partir d, et l'octet arrive dans recu ; pret vaut 1 au front qui suit son bit d'arrêt, occupe pendant l'envoi. raz va aux deux.",
        felicitations: "Un octet a voyagé bit par bit sur un seul fil et il est arrivé intact.",
        corrige: "LIAISON_SERIE",
        brique: "LIAISON_SERIE",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "carte-reseau",
        chapitre: "Le réseau",
        titre: "La carte réseau",
        consigne: "L'élément majeur de cette branche. Un émetteur envoie d sur tx quand envoyer vaut 1 (occupe pendant l'envoi). Un récepteur écoute rx ; chaque octet bien reçu entre dans une file de 4 (ecrire = pret du récepteur). recu montre le plus ancien, dispo = NON vide, plein vient de la file, lire = 1 au front le retire ; erreur vient du récepteur. raz va partout. Comme ta file, elle ne fait rien si lire tombe au front même où un octet arrive : lis entre deux arrivées.",
        felicitations: "Ta carte réseau envoie et reçoit des octets toute seule, et garde ceux qui arrivent pendant que l'ordinateur est occupé. Ton ordinateur peut maintenant parler à un autre.",
        corrige: "CARTE_RESEAU",
        brique: "CARTE_RESEAU",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "compteur-cascade",
        chapitre: "La montre numérique",
        titre: "Le compteur en cascade",
        consigne: "Un compteur décimal qui n'avance que sur ordre : au front montant de clk, si en vaut 1, q avance de 0 à 9 puis revient à 0 ; sinon il ne bouge pas. retenue = en ET (q = 9) : c'est l'ordre d'avancer pour le chiffre suivant. raz remet q à 0.",
        felicitations: "Ton compteur sait passer la main au suivant : on peut maintenant les enchaîner.",
        corrige: "COMPTEUR_CASCADE",
        brique: "COMPTEUR_CASCADE",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "compteur-60",
        chapitre: "La montre numérique",
        titre: "Les secondes et les minutes",
        consigne: "q compte de 00 à 59 : q[0..4] sont les unités (0 à 9), q[4..8] les dizaines (0 à 5), c'est le décimal codé binaire. Au front montant de clk, si en vaut 1, q avance ; après 59 vient 00. retenue = en ET (q = 59). raz remet à 00. Deux compteurs en cascade, le second s'arrêtant à 5.",
        felicitations: "Soixante pas et ton compteur revient à 00 : tu as les secondes, et les minutes avec.",
        corrige: "COMPTEUR_60",
        brique: "COMPTEUR_60",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "compteur-24",
        chapitre: "La montre numérique",
        titre: "Les heures",
        consigne: "Comme les minutes, mais de 00 à 23 : unités dans q[0..4], dizaines dans q[4..8]. Au front où q vaut 23 et en vaut 1, tout revient à 00 ; retenue = en ET (q = 23). raz remet à 00. Indice : reconnais 23 (dizaine 2, unités 3) et force tout à 0.",
        felicitations: "Ton compteur des heures fait le tour du cadran.",
        corrige: "COMPTEUR_24",
        brique: "COMPTEUR_24",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "reveil",
        chapitre: "La montre numérique",
        titre: "Le réveil",
        consigne: "Au front montant de clk, regler = 1 retient h et m comme heure du réveil et l'arme. Ensuite, au premier front où h et m valent l'heure retenue, sonne passe à 1 et y reste ; arreter = 1 l'éteint ; raz éteint et désarme. Deux registres, quatre tests d'égalité, deux bascules (armé, sonne).",
        felicitations: "Ton réveil sonne à l'heure dite. Il ne reste qu'à l'assembler avec la montre.",
        corrige: "REVEIL",
        brique: "REVEIL",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "montre",
        chapitre: "La montre numérique",
        titre: "La montre numérique",
        consigne: "L'élément majeur de cette branche. s, m et h donnent les secondes, minutes et heures en décimal codé binaire. Au front montant de clk, en = 1 ajoute une seconde ; la retenue des secondes fait avancer les minutes, celle des minutes les heures. plus_m avance aussi les minutes, plus_h les heures (pour la régler). raz remet à 00:00:00.",
        felicitations: "Ta montre compte les secondes, les minutes et les heures, et se règle : 86 400 secondes par jour, sans une erreur.",
        corrige: "MONTRE",
        brique: "MONTRE",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "balayage",
        chapitre: "Le contrôleur d'écran",
        titre: "Le balayage de l'écran",
        consigne: "Un écran se dessine pixel par pixel, ligne par ligne. Sur 8 × 8 pixels : à chaque front montant de clk, x avance de 0 à 7 ; après 7, x revient à 0 et y avance (après 7, y revient à 0). fin_ligne = 1 quand x = 7, fin_image quand x = 7 et y = 7. raz revient en (0, 0). Deux compteurs en cascade.",
        felicitations: "Ton balayage parcourt les 64 pixels de l'écran, image après image.",
        corrige: "BALAYAGE",
        brique: "BALAYAGE",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "police",
        chapitre: "Le contrôleur d'écran",
        titre: "La police de caractères",
        consigne: "Chaque chiffre hexadécimal n se dessine en 3 × 5 pixels. motif donne les 3 pixels de la ligne « ligne » (0 en haut), motif[0] à gauche ; les lignes 5 à 7 sont vides. Le 0 : ###, #.#, #.#, #.#, ### ; le 1 : .#., ##., .#., .#., ### ; ouvre la fiche « Comprendre » pour l'idée. Un décodeur pour n, un pour ligne, et des OU.",
        felicitations: "Ta police dessine les seize chiffres hexadécimaux.",
        corrige: "POLICE",
        brique: "POLICE",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "memoire-video",
        chapitre: "Le contrôleur d'écran",
        titre: "La mémoire vidéo",
        consigne: "Huit lignes de 8 pixels. Au front montant de clk, si ecrire vaut 1, la ligne ligne_e prend l'octet d. ligne montre la ligne y, et pixel son bit x. Huit registres, un décodeur pour choisir où écrire, des multiplexeurs pour lire.",
        felicitations: "Ta mémoire vidéo garde l'image : un bit par pixel.",
        corrige: "MEMOIRE_VIDEO",
        brique: "MEMOIRE_VIDEO",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "serialiseur",
        chapitre: "Le contrôleur d'écran",
        titre: "Le sérialiseur de pixels",
        consigne: "Au front montant de clk, charger = 1 prend l'octet d ; sinon les bits avancent d'un rang vers le bit 0 et un 0 entre en haut. pixel montre le bit 0 : les huit pixels d'une ligne sortent un par un. Huit bascules et huit multiplexeurs.",
        felicitations: "Ton sérialiseur envoie une ligne de pixels sur un seul fil, comme le câble d'un écran.",
        corrige: "SERIALISEUR",
        brique: "SERIALISEUR",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "controleur-ecran",
        chapitre: "Le contrôleur d'écran",
        titre: "Le contrôleur d'écran",
        consigne: "L'élément majeur de cette branche. Le balayage donne la position (x, y), et pixel donne le pixel de la mémoire vidéo à cette position : l'écran le reçoit au moment où il le dessine. ecrire = 1 au front range d dans la ligne ligne_e ; raz recommence le balayage en (0, 0) ; fin_image vient du balayage.",
        felicitations: "Ton contrôleur d'écran transforme une mémoire en image : c'est le cœur d'une carte graphique.",
        corrige: "CONTROLEUR_ECRAN",
        brique: "CONTROLEUR_ECRAN",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "premier",
        chapitre: "Nombres remarquables",
        titre: "Le détecteur de nombres premiers",
        consigne: "p vaut 1 quand n (de 0 à 15) est premier : 2, 3, 5, 7, 11 ou 13. Indice : quatre termes suffisent, (non n3 ET non n2 ET n1) OU (non n3 ET n2 ET n0) OU (n2 ET non n1 ET n0) OU (non n2 ET n1 ET n0).",
        felicitations: "Ton circuit reconnaît les nombres premiers d'un coup d'œil, sans faire une seule division.",
        corrige: "PREMIER",
        brique: "PREMIER",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "carre",
        chapitre: "Nombres remarquables",
        titre: "Le carré",
        consigne: "c = n × n, pour n de 0 à 15 (c va jusqu'à 225). Ton multiplicateur fait tout le travail : envoie n sur ses deux entrées.",
        felicitations: "Une brique de plus, presque gratuite : c'est ça, réutiliser.",
        corrige: "CARRE",
        brique: "CARRE",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "binaire-decimal",
        chapitre: "Nombres remarquables",
        titre: "Du binaire au décimal",
        consigne: "n va de 0 à 255 ; sors ses trois chiffres décimaux : n = 100 c + 10 d + u. La méthode du « double dabble » : décale n bit après bit dans les chiffres, et avant chaque décalage ajoute 3 à tout chiffre qui vaut au moins 5. Un petit bloc « ajoute 3 si au moins 5 » (un additionneur de 4 bits) répété sept fois suffit.",
        felicitations: "Ton circuit écrit les nombres comme nous : c'est lui qu'on trouve derrière chaque afficheur de calculatrice.",
        corrige: "BINAIRE_DCB",
        brique: "BINAIRE_DCB",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "fibonacci",
        chapitre: "Nombres remarquables",
        titre: "La suite de Fibonacci",
        consigne: "Au front montant de clk, raz = 1 met a à 0 et b à 1 ; sinon a prend b et b prend a + b (modulo 256). f montre a : 0, 1, 1, 2, 3, 5, 8, 13… Deux registres de 8 bits et un additionneur.",
        felicitations: "Chaque front donne le terme suivant : 0, 1, 1, 2, 3, 5, 8… la suite des lapins de Fibonacci.",
        corrige: "FIBONACCI",
        brique: "FIBONACCI",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "gray",
        chapitre: "Maths et automates",
        titre: "Le code de Gray",
        consigne: "Un code où deux nombres qui se suivent ne diffèrent que d'un bit. g[i] = b[i] ⊕ b[i+1] pour i de 0 à 2, et g[3] = b[3]. Trois OU exclusifs.",
        felicitations: "Ton code de Gray ne change qu'un bit à la fois : idéal pour les roues codeuses.",
        corrige: "GRAY",
        brique: "GRAY",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "motif-101",
        chapitre: "Maths et automates",
        titre: "Le reconnaisseur de motif",
        consigne: "Un automate qui surveille un flot de bits : à chaque front montant de clk, e est lu. vu vaut 1 quand les trois dernières valeurs lues sont 1, 0, 1, dans cet ordre. raz oublie tout. Trois bascules gardent l'histoire, une porte reconnaît le motif.",
        felicitations: "Ton automate repère le motif 1, 0, 1 dans un flot de bits.",
        corrige: "MOTIF_101",
        brique: "MOTIF_101",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "divisible-3",
        chapitre: "Maths et automates",
        titre: "Divisible par 3",
        consigne: "Un automate qui lit un nombre bit par bit, poids fort d'abord : à chaque front montant de clk, e est le bit suivant. div vaut 1 quand le nombre lu jusque-là est un multiple de 3. raz recommence. Indice : il suffit de retenir le reste r (0, 1 ou 2) ; lire un bit e le remplace par le reste de 2r + e.",
        felicitations: "Ton automate sait si un nombre de n'importe quelle longueur est divisible par 3, avec deux bascules seulement.",
        corrige: "DIVISIBLE_3",
        brique: "DIVISIBLE_3",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "cellule-vie",
        chapitre: "Maths et automates",
        titre: "La cellule du jeu de la vie",
        consigne: "Le jeu de la vie de Conway : une cellule vit ou meurt selon ses 8 voisines. s vaut 1 quand exactement 3 voisins n sont vivants, ou quand la cellule est vivante (v = 1) et en a exactement 2. Compte les voisins avec des additionneurs, puis reconnais 2 et 3.",
        felicitations: "Ta cellule obéit aux règles de la vie : un écran rempli de ces cellules fait vivre des planeurs.",
        corrige: "CELLULE_VIE",
        brique: "CELLULE_VIE",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "automate-30",
        chapitre: "Maths et automates",
        titre: "L'automate cellulaire",
        consigne: "Huit cellules en anneau suivent la règle 30. Au front montant de clk, charger = 1 met q à d ; sinon chaque cellule devient gauche ⊕ (elle-même OU droite), où gauche est q[i+1] et droite q[i−1] (q[7] et q[0] sont voisines).",
        felicitations: "Une règle minuscule, un dessin imprévisible : la règle 30 sert même à fabriquer du hasard.",
        corrige: "AUTOMATE_30",
        brique: "AUTOMATE_30",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "pgcd",
        chapitre: "Maths et automates",
        titre: "La machine d'Euclide",
        consigne: "L'élément majeur de cette branche : un algorithme vieux de 2 300 ans, en circuit. Au front montant de clk, charger = 1 met x à a et y à b ; sinon le plus grand des deux devient leur différence. fini vaut 1 quand x = y, ou quand l'un des deux vaut 0 : le PGCD est alors x (ou y si x vaut 0). Un comparateur, deux soustracteurs, des multiplexeurs.",
        felicitations: "Ta machine calcule le plus grand diviseur commun toute seule, pas à pas : un algorithme devenu circuit.",
        corrige: "PGCD",
        brique: "PGCD",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "cesar",
        chapitre: "Codes et secrets",
        titre: "Le chiffre de César",
        consigne: "Jules César décalait chaque lettre de son message. Ici une « lettre » est un nombre de 0 à 15 : c = (d + k) modulo 16. Un additionneur de 4 bits, sa retenue d'entrée à 0, et on oublie la retenue de sortie : c'est elle qui fait revenir au début.",
        felicitations: "Ton chiffreur décale chaque lettre. Pour déchiffrer, il suffit d'avancer de 16 − k.",
        corrige: "CESAR",
        brique: "CESAR",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "masque-xor",
        chapitre: "Codes et secrets",
        titre: "Le masque jetable",
        consigne: "c = d ⊕ k, bit à bit, sur 8 bits. Huit OU exclusifs. Le secret : refaire ⊕ k avec la même clé redonne le message, car x ⊕ k ⊕ k = x.",
        felicitations: "Avec une clé vraiment au hasard, utilisée une seule fois, ce chiffre est incassable : c'est prouvé.",
        corrige: "MASQUE_XOR",
        brique: "MASQUE_XOR",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "somme-controle",
        chapitre: "Codes et secrets",
        titre: "La somme de contrôle",
        consigne: "Pour vérifier qu'un message est arrivé intact, on additionne tous ses octets. Au front montant de clk, raz = 1 remet somme à 0 ; sinon, ajouter = 1 ajoute l'octet d à somme (modulo 256). Un registre de 8 bits, un additionneur et deux multiplexeurs par bit.",
        felicitations: "Ta somme de contrôle accompagne un message : si un octet change en route, la somme ne correspond plus.",
        corrige: "SOMME_CONTROLE",
        brique: "SOMME_CONTROLE",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "hamming-code",
        chapitre: "Codes et secrets",
        titre: "Le code de Hamming",
        consigne: "Ajoute trois bits de parité aux 4 bits de d, pour pouvoir plus tard réparer une erreur. h a 7 places, h[0] étant la place 1 : d0, d1, d2, d3 vont aux places 3, 5, 6, 7 ; la place 1 reçoit d0 ⊕ d1 ⊕ d3, la place 2 d0 ⊕ d2 ⊕ d3, la place 4 d1 ⊕ d2 ⊕ d3.",
        felicitations: "Ton message porte maintenant sa propre assurance : trois bits de parité bien placés.",
        corrige: "HAMMING_CODE",
        brique: "HAMMING_CODE",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "hamming-correcteur",
        chapitre: "Codes et secrets",
        titre: "Le correcteur d'erreurs",
        consigne: "L'élément majeur de cette branche : un circuit qui répare un bit faux. Refais les trois contrôles : c1 = places 1 ⊕ 3 ⊕ 5 ⊕ 7, c2 = places 2 ⊕ 3 ⊕ 6 ⊕ 7, c4 = places 4 ⊕ 5 ⊕ 6 ⊕ 7. Le nombre c1 + 2 c2 + 4 c4 donne la place du bit faux (0 si tout va bien). Un décodeur de 3 bits désigne cette place ; retourne-la avec un OU exclusif, puis relis d aux places 3, 5, 6, 7. erreur vaut 1 quand un bit a été corrigé.",
        felicitations: "Ton correcteur répare n'importe quel bit retourné : c'est ce que font les mémoires des serveurs et les sondes spatiales.",
        corrige: "HAMMING_CORRECTEUR",
        brique: "HAMMING_CORRECTEUR",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "arbitre",
        chapitre: "Jeux",
        titre: "L'arbitre de pierre-feuille-ciseaux",
        consigne: "a et b valent 0 (pierre), 1 (feuille) ou 2 (ciseaux). La feuille bat la pierre, les ciseaux battent la feuille, la pierre bat les ciseaux. Allume gagne_a, gagne_b ou egalite ; le code 3 n'est pas un coup, et tout reste éteint. Indice : un décodeur de 2 bits par joueur, puis un ET par duel.",
        felicitations: "Ton arbitre ne se trompe jamais et ne triche pas.",
        corrige: "ARBITRE",
        brique: "ARBITRE",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "morpion",
        chapitre: "Jeux",
        titre: "Le juge du morpion",
        consigne: "x[i] vaut 1 quand la case i porte une croix ; les cases vont de 0 à 8, ligne par ligne (0, 1, 2 en haut). gagne vaut 1 quand trois croix sont alignées : trois lignes, trois colonnes, deux diagonales. Huit ET à trois entrées, puis un grand OU.",
        felicitations: "Ton juge voit d'un coup les huit alignements : vérifié sur les 512 grilles possibles.",
        corrige: "MORPION",
        brique: "MORPION",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "de",
        chapitre: "Jeux",
        titre: "Le dé électronique",
        consigne: "Au front montant de clk, raz = 1 met face à 1 ; sinon, tant que lancer vaut 1, face avance de 1 à 6, puis revient à 1. Quand on lâche lancer, le dé s'arrête. Indice : un compteur de 3 bits ; 6 est la seule face où les bits 1 et 2 valent 1.",
        felicitations: "L'horloge tourne bien plus vite que ton doigt : là où tu lâches, c'est le hasard.",
        corrige: "DE",
        brique: "DE",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "meilleur-score",
        chapitre: "Jeux",
        titre: "Le meilleur score",
        consigne: "nouveau vaut 1 quand valider = 1 et que score dépasse record. Au front montant de clk, raz = 1 remet record à 0 ; sinon un nouveau record prend la place de l'ancien. Indice : deux comparateurs de 4 bits font un comparateur de 8 bits, la moitié haute décide d'abord.",
        felicitations: "Ta machine retient le meilleur score, comme une borne d'arcade.",
        corrige: "MEILLEUR_SCORE",
        brique: "MEILLEUR_SCORE",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "memoire-64",
        chapitre: "La grande mémoire",
        titre: "La mémoire de 64 octets",
        consigne: "Quatre fois plus grande que ta mémoire de 16 octets, et faite de quatre d'entre elles. Les 2 bits hauts de l'adresse (adresse[4:5]) choisissent la mémoire ; les 4 bits bas vont à toutes. Un décodeur de 2 bits envoie ecrire à la seule mémoire choisie (un ET par mémoire), et des multiplexeurs à 4 voies choisissent son q, bit par bit.",
        felicitations: "Quatre mémoires n'en font qu'une : c'est le même tour qu'on répète pour aller aussi loin qu'on veut.",
        corrige: "RAM64",
        brique: "RAM64",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "memoire-256",
        chapitre: "La grande mémoire",
        titre: "La mémoire de 256 octets",
        consigne: "On refait le même tour : quatre mémoires de 64 octets, les 2 bits hauts (adresse[6:7]) choisissent laquelle. 256 cases de 8 bits : 2 048 bits gardés par plus de 20 000 portes NAND.",
        felicitations: "Ta mémoire est seize fois plus grande que celle de l'ordinateur miniature. Il est temps de lui trouver un ordinateur.",
        corrige: "RAM256",
        brique: "RAM256",
        epreuve: Epreuve::Sequentielle,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "ordinateur-256",
        chapitre: "La grande mémoire",
        titre: "L'ordinateur à 256 octets",
        consigne: "Le processeur et la mémoire morte sont posés ; la mémoire vive, c'est ta brique de 256 octets. Branche-les comme pour l'ordinateur, mais cette fois les 8 bits bas de rom.d vont à l'adresse de la mémoire vive. Le programme range 24 touches à des adresses éloignées les unes des autres (une mémoire plus petite les confondrait), puis les affiche à l'envers.",
        felicitations: "Ton ordinateur a seize fois plus de mémoire, et c'est toi qui l'as construite. Il lit à l'envers comme Léonard de Vinci écrivait.",
        corrige: "",
        brique: "ORDINATEUR",
        epreuve: Epreuve::Systeme {
            atelier: Atelier::GrandeMemoire {
                programme: ordinateur::programmes::MIROIR,
            },
            touches: "BONJOUR DEPUIS LES NAND!",
            ecran: "!DNAN SEL SIUPED RUOJNOB",
        },
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "messagerie",
        chapitre: "Ordinateurs en réseau",
        titre: "La messagerie",
        consigne: "Deux ordinateurs complets : A lit le clavier et passe chaque touche en majuscule, B affiche ce qu'il reçoit. Relie-les par une boîte aux lettres : un registre de 8 bits dont d vient de a.ecran, clk de a.ecran_ecrit (il retient l'octet au moment où A l'écrit) et charge de raz inversé. Sa sortie q va à b.clavier. Le bouton raz démarre les deux, le clavier va à a.clavier, et l'écran à b.ecran et b.ecran_ecrit. Réussi quand « Salut » tapé sur A s'affiche « SALUT » sur B.",
        felicitations: "Deux ordinateurs se parlent : plus de 6 000 portes NAND, et un message qui voyage de l'un à l'autre.",
        corrige: "",
        brique: "ORDINATEUR",
        epreuve: Epreuve::Systeme {
            atelier: Atelier::Deux {
                envoi: ordinateur::programmes::ENVOI_MAJUSCULES,
                reception: ordinateur::programmes::RECEPTION,
            },
            touches: "Salut",
            ecran: "SALUT",
        },
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "messagerie-chiffree",
        chapitre: "Ordinateurs en réseau",
        titre: "La messagerie secrète",
        consigne: "L'élément majeur des grands projets. A chiffre chaque touche avec le code de César (un rang plus loin : H devient I) avant de l'envoyer ; B la déchiffre (un rang en arrière) et l'affiche. Branche-les avec la boîte aux lettres, comme pour la messagerie. Entre les deux, le message passe chiffré : qui l'espionne lit « IBM ». Réussi quand « HAL » tapé sur A s'affiche « HAL » sur B.",
        felicitations: "Ton message a voyagé chiffré d'un ordinateur à l'autre et il est arrivé intact : un vrai petit réseau sécurisé, de la NAND jusqu'au secret.",
        corrige: "",
        brique: "ORDINATEUR",
        epreuve: Epreuve::Systeme {
            atelier: Atelier::Deux {
                envoi: ordinateur::programmes::CHIFFREMENT,
                reception: ordinateur::programmes::DECHIFFREMENT,
            },
            touches: "HAL",
            ecran: "HAL",
        },
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "panne-ou-x",
        chapitre: "Dépannage",
        titre: "Le OU exclusif en panne",
        consigne: "Ce OU exclusif est déjà construit, mais un fil est mal branché. Essaie les quatre cas de a et b pour voir lequel est faux, suis le courant de porte en porte, puis rebranche le bon fil. Il ne faut changer qu'une seule entrée.",
        felicitations: "Panne trouvée et réparée : tu as fait le travail d'un vrai dépanneur, qui lit un circuit qu'il n'a pas dessiné.",
        corrige: "OU_X",
        brique: "OU_X_REPARE",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "panne-demi-additionneur",
        chapitre: "Dépannage",
        titre: "Le demi-additionneur en panne",
        consigne: "s doit donner la somme de a et b, r la retenue, mais une porte reçoit le mauvais fil. Regarde quelle lampe se trompe, et dans quels cas : cela dit de quel côté chercher. Une seule entrée est à rebrancher.",
        felicitations: "Réparé. Regarder quelle sortie se trompe, et quand, réduit la recherche à quelques portes.",
        corrige: "DEMI_ADD",
        brique: "DEMI_ADD_REPARE",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "panne-multiplexeur",
        chapitre: "Dépannage",
        titre: "L'aiguillage en panne",
        consigne: "Ce multiplexeur devrait recopier a quand sel vaut 0 et b quand sel vaut 1. Un fil s'est trompé de place. Fixe sel, change a puis b, et observe : quand la lampe ne suit plus, remonte le courant jusqu'à la porte fautive.",
        felicitations: "L'aiguillage est réparé. Fixer une entrée et faire varier les autres, c'est la méthode des dépanneurs.",
        corrige: "MUX",
        brique: "MUX_REPARE",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
    Fiche {
        cle: "panne-additionneur-complet",
        chapitre: "Dépannage",
        titre: "L'additionneur complet en panne",
        consigne: "Neuf NAND additionnent a, b et la retenue re, mais l'une d'elles est mal branchée. Ctrl+E montre la table de vérité du circuit : compare-la à une addition juste, trouve les lignes fausses, puis la porte. Une seule entrée est à rebrancher.",
        felicitations: "Neuf portes, un seul fil faux, et tu l'as trouvé. Les ingénieurs passent autant de temps à dépanner qu'à construire.",
        corrige: "ADD_COMPLET",
        brique: "ADD_COMPLET_REPARE",
        epreuve: Epreuve::Combinatoire,
        renommer: &[],
        profondeur_max: None,
    },
];

/// Une broche que la quête attend : un interrupteur ou une lampe du plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attendue {
    /// Nom de l'élément dans le plan.
    pub nom: String,
    /// Nom de la broche du corrigé.
    pub corrige: String,
    /// Nombre de bits.
    pub largeur: u32,
}

/// Le résultat d'une vérification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verdict {
    /// Vrai si tous les essais sont justes.
    pub reussi: bool,
    /// Ce qu'on en dit.
    pub texte: String,
}

impl Verdict {
    fn echec(texte: impl Into<String>) -> Self {
        Verdict {
            reussi: false,
            texte: texte.into(),
        }
    }
}

/// Une quête en cours.
#[derive(Clone, Debug)]
pub struct Quete {
    /// Clé enregistrée dans le projet pour retrouver la quête.
    pub cle: &'static str,
    /// Titre affiché.
    pub titre: &'static str,
    /// Consigne.
    pub consigne: &'static str,
    /// Texte affiché une fois la quête réussie.
    pub felicitations: String,
    /// Nom de la brique gagnée.
    pub brique: &'static str,
    /// Comment elle se vérifie.
    pub epreuve: Epreuve,
    /// Les interrupteurs, dans l'ordre des colonnes.
    pub entrees: Vec<Attendue>,
    /// Les lampes, dans l'ordre des colonnes.
    pub sorties: Vec<Attendue>,
    /// Pour une petite quête : les sorties justes de chaque cas, dans l'ordre de [`Quete::cas`].
    table: Vec<Vec<bool>>,
    /// Les combinaisons d'entrées déjà validées à la main.
    pub vus: BTreeSet<Vec<bool>>,
    /// Le résultat de la dernière vérification complète.
    pub verdict: Option<Verdict>,
}

/// Les corrigés : la bibliothèque de référence et les composants de l'ordinateur.
pub(crate) fn corriges() -> &'static Bibliotheque {
    static BIB: OnceLock<Bibliotheque> = OnceLock::new();
    BIB.get_or_init(ce_circuit::classiques::bibliotheque)
}

/// Un générateur pseudo-aléatoire fixe : les essais sont les mêmes à chaque vérification.
struct Hasard(u64);

impl Hasard {
    fn suivant(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn sous(&mut self, n: u64) -> u64 {
        self.suivant() % n.max(1)
    }
}

fn masque(largeur: u32) -> u64 {
    if largeur >= 64 {
        u64::MAX
    } else {
        (1 << largeur) - 1
    }
}

/// Nombre maximal de combinaisons essayées toutes ; au-delà, on tire un échantillon.
const TOUTES_JUSQU_A: u32 = 10;
/// Nombre d'essais tirés au hasard (combinatoire) ou d'actions jouées (séquentielle).
const ESSAIS: usize = 400;
/// Temps laissé à chaque essai pour se stabiliser.
const BUDGET: u64 = 20_000;

impl Quete {
    /// La fiche du parcours qui porte cette clé.
    pub fn fiche(cle: &str) -> Option<&'static Fiche> {
        PARCOURS.iter().find(|f| f.cle == cle)
    }

    /// Le demi-additionneur, la quête de l'atelier au premier lancement.
    pub fn demi_additionneur() -> Self {
        Self::de_cle("demi-additionneur").expect("le demi-additionneur est au parcours")
    }

    /// La quête enregistrée sous cette clé.
    pub fn de_cle(cle: &str) -> Option<Self> {
        Self::fiche(cle).map(Self::de_fiche)
    }

    /// La quête décrite par cette fiche.
    pub fn de_fiche(f: &'static Fiche) -> Self {
        let mut q = Quete {
            cle: f.cle,
            titre: f.titre,
            consigne: f.consigne,
            felicitations: if f.felicitations.is_empty() {
                tr!(
                    "Juste ! « {brique} » rejoint tes briques, prête pour les quêtes suivantes.",
                    brique = f.brique
                )
            } else {
                crate::langue::traduire(f.felicitations).to_owned()
            },
            brique: f.brique,
            epreuve: f.epreuve,
            entrees: Vec::new(),
            sorties: Vec::new(),
            table: Vec::new(),
            vus: BTreeSet::new(),
            verdict: None,
        };
        if let Some(c) = corriges().composants.get(f.corrige) {
            for b in &c.broches {
                let nom = f
                    .renommer
                    .iter()
                    .find(|(de, _)| *de == b.nom)
                    .map_or(b.nom.as_str(), |(_, vers)| vers);
                let attendue = Attendue {
                    nom: nom.to_owned(),
                    corrige: b.nom.clone(),
                    largeur: b.largeur,
                };
                match b.sens {
                    Sens::Entree => q.entrees.push(attendue),
                    Sens::Sortie => q.sorties.push(attendue),
                }
            }
        }
        if q.manuelle() {
            q.table = q.calculer_la_table(f.corrige);
        }
        q
    }

    /// Vrai si la quête se valide aussi cas par cas, en cliquant : un circuit sans mémoire,
    /// à au plus trois entrées d'un bit.
    pub fn manuelle(&self) -> bool {
        self.epreuve == Epreuve::Combinatoire
            && !self.entrees.is_empty()
            && self.entrees.len() <= 3
            && self
                .entrees
                .iter()
                .chain(&self.sorties)
                .all(|b| b.largeur == 1)
    }

    fn calculer_la_table(&self, corrige: &str) -> Vec<Vec<bool>> {
        let Ok(mut banc) = Banc::nouveau(corriges(), corrige) else {
            return Vec::new();
        };
        self.cas()
            .iter()
            .map(|cas| {
                for (b, v) in self.entrees.iter().zip(cas) {
                    let _ = banc.ecrire(&b.corrige, u64::from(*v));
                }
                banc.stabiliser(BUDGET);
                self.sorties
                    .iter()
                    .map(|b| banc.lire(&b.corrige).ok().flatten() == Some(1))
                    .collect()
            })
            .collect()
    }

    /// Ce que doivent valoir les lampes pour ces entrées (petites quêtes seulement).
    pub fn attendu(&self, cas: &[bool]) -> Option<&[bool]> {
        let i = self.cas().iter().position(|c| c == cas)?;
        self.table.get(i).map(Vec::as_slice)
    }

    /// Le plan de départ : les interrupteurs à gauche, les lampes à droite, rien entre eux.
    pub fn plan_de_depart(&self) -> Document {
        let mut d = Document::nouveau();
        let ecart = |n: usize| if n > 4 { 100.0 } else { 140.0 };
        let haut = |n: usize| 300.0 - ecart(n) * (n.saturating_sub(1)) as f32 / 2.0;
        for (i, b) in self.entrees.iter().enumerate() {
            let y = haut(self.entrees.len()) + ecart(self.entrees.len()) * i as f32;
            d.poser(Element::entree(&b.nom, b.largeur), Point::nouveau(80.0, y));
        }
        for (i, b) in self.sorties.iter().enumerate() {
            let y = haut(self.sorties.len()) + ecart(self.sorties.len()) * i as f32;
            let mut lampe = Element::sortie(&b.nom, &b.nom, b.largeur);
            // Les segments d'un chiffre se voient sur un afficheur 7 segments.
            if b.nom == "seg" && matches!(b.largeur, 7 | 8) {
                lampe.aspect = crate::document::Aspect::Segments;
            }
            d.poser(lampe, Point::nouveau(900.0, y));
        }
        d
    }

    /// Vrai si l'élément est un interrupteur ou une lampe dont la quête a besoin.
    pub fn tient_a(&self, doc: &Document, id: Id) -> bool {
        doc.element(id).is_some_and(|e| match e.genre {
            Genre::Entree => self.entrees.iter().any(|b| b.nom == e.nom),
            Genre::Sortie => self.sorties.iter().any(|b| b.nom == e.nom),
            _ => false,
        })
    }

    /// Toutes les combinaisons d'entrées des petites quêtes, dans l'ordre de la table de
    /// vérité. Vide pour les autres.
    pub fn cas(&self) -> Vec<Vec<bool>> {
        if !self.manuelle() {
            return Vec::new();
        }
        let n = self.entrees.len();
        (0..1usize << n)
            .map(|k| (0..n).map(|i| k >> (n - 1 - i) & 1 == 1).collect())
            .collect()
    }

    /// Vrai quand la quête est réussie : vérification complète juste, ou tous les cas
    /// validés à la main.
    pub fn reussie(&self) -> bool {
        self.verdict.as_ref().is_some_and(|v| v.reussi)
            || (self.manuelle() && self.vus.len() == 1 << self.entrees.len())
    }

    /// La combinaison d'entrées actuellement réglée sur les interrupteurs, si la quête
    /// retrouve tous ses interrupteurs dans le document.
    pub fn cas_courant(&self, doc: &Document, session: &Session) -> Option<Vec<bool>> {
        self.entrees
            .iter()
            .map(|b| {
                let e = doc
                    .elements
                    .iter()
                    .find(|e| e.genre == Genre::Entree && e.nom == b.nom)?;
                session.entrees.get(&e.id).map(|v| *v == 1)
            })
            .collect()
    }

    /// Regarde le circuit : s'il est au repos et que ses sorties sont justes pour les entrées
    /// réglées, ce cas est validé. Renvoie le cas s'il vient d'être validé pour la première fois.
    pub fn observer(&mut self, doc: &Document, session: &Session) -> Option<Vec<bool>> {
        if !self.manuelle() || !session.stable() {
            return None;
        }
        let cas = self.cas_courant(doc, session)?;
        let attendu = self.attendu(&cas)?.to_vec();
        for (b, voulu) in self.sorties.iter().zip(attendu) {
            let lampe = doc
                .elements
                .iter()
                .find(|e| e.genre == Genre::Sortie && e.nom == b.nom)?;
            if session.valeur_lue(
                doc,
                Fil {
                    puits: lampe.id,
                    entree: 0,
                },
            ) != Valeur::de_bool(voulu)
            {
                return None;
            }
        }
        self.vus.insert(cas.clone()).then_some(cas)
    }

    /// Oublie les cas validés et la dernière vérification : à appeler quand le câblage change.
    pub fn recommencer(&mut self) {
        self.vus.clear();
        self.verdict = None;
    }

    /// Vérifie le plan principal du projet contre le corrigé, et retient le verdict.
    pub fn verifier(&mut self, projet: &Projet) -> &Verdict {
        let v = match self.epreuve {
            Epreuve::Ordinateur { touches, ecran, .. }
            | Epreuve::Systeme { touches, ecran, .. } => {
                verifier_l_ordinateur(projet, touches, ecran)
            }
            _ => self.comparer(projet),
        };
        // Vérifié d'un coup : chaque ligne de la table est juste, on les coche toutes.
        if v.reussi {
            self.vus.extend(self.cas());
        }
        self.verdict.insert(v)
    }

    /// Retrouve dans le plan l'élément de chaque broche attendue.
    fn reperer(&self, plan: &Document) -> Result<(Vec<Id>, Vec<Id>), String> {
        let trouver = |b: &Attendue, genre: Genre| -> Result<Id, String> {
            let quoi = if genre == Genre::Entree {
                tr!("l'interrupteur")
            } else {
                tr!("la lampe")
            };
            let e = plan
                .elements
                .iter()
                .find(|e| e.genre == genre && e.nom == b.nom)
                .ok_or_else(|| {
                    tr!(
                        "Il manque {quoi} « {nom} » sur le plan.",
                        quoi = quoi,
                        nom = b.nom
                    )
                })?;
            let largeur = match genre {
                Genre::Entree => e.broches_s[0].largeur,
                _ => e.broches_e[0].largeur,
            };
            if largeur != b.largeur {
                return Err(tr!(
                    "{v} « {nom} » doit avoir {largeur2} bit{v2}, pas {largeur}.",
                    largeur = largeur,
                    v = if genre == Genre::Entree {
                        tr!("L'interrupteur")
                    } else {
                        tr!("La lampe")
                    },
                    nom = b.nom,
                    largeur2 = b.largeur,
                    v2 = crate::langue::s(b.largeur)
                ));
            }
            Ok(e.id)
        };
        let entrees = self
            .entrees
            .iter()
            .map(|b| trouver(b, Genre::Entree))
            .collect::<Result<_, _>>()?;
        let sorties = self
            .sorties
            .iter()
            .map(|b| trouver(b, Genre::Sortie))
            .collect::<Result<_, _>>()?;
        Ok((entrees, sorties))
    }

    /// Les essais : chacun donne la valeur de toutes les entrées.
    fn essais(&self) -> Vec<Vec<u64>> {
        let largeurs: Vec<u32> = self.entrees.iter().map(|b| b.largeur).collect();
        let total: u32 = largeurs.iter().sum();
        let mut h = Hasard(0x9E37_79B9_7F4A_7C15 ^ self.cle.len() as u64);
        let mut essais = Vec::new();
        match self.epreuve {
            Epreuve::Combinatoire if total <= TOUTES_JUSQU_A => {
                for k in 0..1u64 << total {
                    let mut decalage = 0;
                    essais.push(
                        largeurs
                            .iter()
                            .map(|&l| {
                                let v = k >> decalage & masque(l);
                                decalage += l;
                                v
                            })
                            .collect(),
                    );
                }
            }
            Epreuve::Combinatoire => {
                // Les extrêmes d'abord, là où se cachent les retenues oubliées.
                essais.push(largeurs.iter().map(|_| 0).collect());
                essais.push(largeurs.iter().map(|&l| masque(l)).collect());
                essais.push(
                    largeurs
                        .iter()
                        .enumerate()
                        .map(|(i, &l)| if i == 0 { masque(l) } else { 1 & masque(l) })
                        .collect(),
                );
                while essais.len() < ESSAIS {
                    essais.push(largeurs.iter().map(|&l| h.suivant() & masque(l)).collect());
                }
            }
            Epreuve::Sequentielle => {
                // Depuis la mise sous tension, une entrée change à la fois : jamais deux
                // changements simultanés, dont l'ordre dépendrait du câblage.
                let mut courant: Vec<u64> = largeurs.iter().map(|_| 0).collect();
                essais.push(courant.clone());
                while essais.len() < ESSAIS {
                    let i = h.sous(largeurs.len() as u64) as usize;
                    let l = largeurs[i];
                    courant[i] = if l == 1 {
                        courant[i] ^ 1
                    } else {
                        h.suivant() & masque(l)
                    };
                    essais.push(courant.clone());
                }
            }
            Epreuve::Ordinateur { .. } | Epreuve::Systeme { .. } => {}
        }
        essais
    }

    fn decrire(&self, valeurs: &[u64]) -> String {
        self.entrees
            .iter()
            .zip(valeurs)
            .map(|(b, v)| format!("{} = {v}", crate::langue::nom_affiche(&b.nom)))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Fait tourner le plan et le corrigé côte à côte sur les mêmes essais.
    fn comparer(&self, projet: &Projet) -> Verdict {
        let fiche = Self::fiche(self.cle);
        let Some(corrige) = fiche.map(|f| f.corrige) else {
            return Verdict::echec(tr!("Cette quête n'a pas de corrigé."));
        };
        let plan = &projet.principal;
        let (entrees, sorties) = match self.reperer(plan) {
            Ok(x) => x,
            Err(e) => return Verdict::echec(e),
        };
        let mut briques = Briques::default();
        briques.suivre(projet);
        let circuit = match Pont::construire(plan, projet, &briques) {
            Ok(p) => p.circuit,
            Err(e) => {
                return Verdict::echec(tr!("Le circuit ne peut pas être simulé : {e}.", e = e))
            }
        };
        let profondeur = ce_circuit::analyse::chemin_critique(&circuit.netlist);
        let (Ok(mut toi), Ok(mut lui)) = (
            Banc::de_circuit(circuit),
            Banc::nouveau(corriges(), corrige),
        ) else {
            return Verdict::echec(tr!("Le circuit ne peut pas être simulé."));
        };
        let essais = self.essais();
        let sequentielle = self.epreuve == Epreuve::Sequentielle;
        for (n, valeurs) in essais.iter().enumerate() {
            for (k, v) in valeurs.iter().enumerate() {
                let _ = toi.ecrire(&format!("e{}", entrees[k]), *v);
                let _ = lui.ecrire(&self.entrees[k].corrige, *v);
            }
            lui.stabiliser(BUDGET);
            if !toi.stabiliser(BUDGET).est_stable() {
                return Verdict::echec(tr!(
                    "Avec {v}, ton circuit ne se stabilise pas : il oscille.",
                    v = self.decrire(valeurs)
                ));
            }
            for (k, b) in self.sorties.iter().enumerate() {
                let voulu = lui.lire_valeurs(&b.corrige).unwrap_or_default();
                let vu = toi
                    .lire_valeurs(&format!("s{}", sorties[k]))
                    .unwrap_or_default();
                // Un bit que le corrigé lui-même ne connaît pas encore (une mémoire jamais
                // écrite) n'est pas jugé.
                let faux = voulu
                    .iter()
                    .zip(&vu)
                    .any(|(v, t)| v.est_binaire() && v != t);
                if faux {
                    let lire = |vals: &[Valeur]| {
                        let mut n = 0u64;
                        for (i, v) in vals.iter().enumerate() {
                            match v {
                                Valeur::Un => n |= 1 << i,
                                Valeur::Zero => {}
                                _ => return tr!("indéterminé").to_owned(),
                            }
                        }
                        n.to_string()
                    };
                    let moment = if sequentielle {
                        tr!("Au {v}e essai, avec ", v = n + 1)
                    } else {
                        tr!("Avec ").to_owned()
                    };
                    return Verdict::echec(tr!(
                        "{moment}{v}, la lampe {nom} devrait valoir {v2} ; elle montre {v3}.",
                        moment = moment,
                        v = self.decrire(valeurs),
                        nom = b.nom,
                        v2 = lire(&voulu),
                        v3 = lire(&vu)
                    ));
                }
            }
        }
        if let (Some(max), Some(d)) = (fiche.and_then(|f| f.profondeur_max), profondeur) {
            if d > max {
                return Verdict::echec(tr!("Tous les essais sont justes, mais ton circuit met {d} étapes à répondre dans le pire cas : il en faut au plus {max}.", d = d, max = max));
            }
        }
        Verdict {
            reussi: true,
            texte: tr!(
                "{len} essai{v} sur {v2}, tous justes, comparés au corrigé.",
                len = essais.len(),
                v = crate::langue::s(essais.len()),
                v2 = if sequentielle {
                    tr!("une longue suite d'actions")
                } else if essais.len() == 1 << self.entrees.iter().map(|b| b.largeur).sum::<u32>() {
                    tr!("toutes les combinaisons")
                } else {
                    tr!("des combinaisons tirées au hasard")
                }
            ),
        }
    }
}

/// L'ordinateur est juste s'il fait tourner son programme : `touches` au clavier, `attendu`
/// à l'écran.
fn verifier_l_ordinateur(projet: &Projet, touches: &str, attendu: &str) -> Verdict {
    let plan = &projet.principal;
    let un = |genre: Genre, quoi: &str| -> Result<Id, String> {
        let mut tous = plan.elements.iter().filter(|e| e.genre == genre);
        match (tous.next(), tous.next()) {
            (Some(e), None) => Ok(e.id),
            (None, _) => Err(tr!("Il faut {quoi} sur le plan.", quoi = quoi)),
            _ => Err(tr!("Il faut un seul {quoi} sur le plan.", quoi = quoi)),
        }
    };
    let (clavier, ecran) = match (
        un(Genre::Clavier, "clavier"),
        un(Genre::Ecran, tr!("écran")),
    ) {
        (Ok(c), Ok(e)) => (c, e),
        (Err(e), _) | (_, Err(e)) => return Verdict::echec(e),
    };
    let mut s = Session::nouvelle(plan, projet, &[]);
    if let Some(e) = s.erreur() {
        return Verdict::echec(tr!("Le circuit ne peut pas être simulé : {e}.", e = e));
    }
    for c in touches.bytes() {
        s.taper(clavier, c);
    }
    let periode = 2 * Reglages::default().demi_periode;
    s.avancer_vite(periode * 300 * (touches.len() as u64 + 1));
    let vu = s.ecran(ecran).to_owned();
    let tape = tr!("Après « {touches} » au clavier, l'écran", touches = touches);
    if vu == attendu {
        Verdict {
            reussi: true,
            texte: format!("{tape} affiche {attendu}."),
        }
    } else if vu.is_empty() {
        Verdict::echec(tr!("{tape} n'affiche rien.", tape = tape))
    } else {
        Verdict::echec(tr!(
            "{tape} affiche « {vu} » au lieu de {attendu}.",
            tape = tape,
            vu = vu,
            attendu = attendu
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Source;

    #[test]
    fn la_quete_se_valide_cas_par_cas_et_seulement_au_repos() {
        let doc = Document::demi_additionneur();
        let id = |nom: &str| doc.elements.iter().find(|e| e.nom == nom).unwrap().id;
        let mut s = Session::du_plan(&doc, &[(id("a"), 1)]);
        let mut q = Quete::demi_additionneur();
        assert_eq!(
            q.cas(),
            [[false, false], [false, true], [true, false], [true, true]]
        );
        assert_eq!(q.attendu(&[true, true]), Some(&[false, true][..]));
        assert_eq!(
            q.observer(&doc, &s),
            None,
            "le circuit n'est pas encore au repos"
        );
        for _ in 0..3 {
            s.pas(&doc);
        }
        assert_eq!(q.observer(&doc, &s), Some(vec![true, false]));
        assert_eq!(
            q.observer(&doc, &s),
            None,
            "un cas ne se valide qu'une fois"
        );
        for entree in ["b", "a", "b"] {
            s.basculer(&doc, id(entree));
            assert_eq!(q.observer(&doc, &s), None);
            for _ in 0..3 {
                s.pas(&doc);
            }
            assert!(q.observer(&doc, &s).is_some());
        }
        assert!(q.reussie());
    }

    #[test]
    fn un_circuit_faux_ne_valide_rien() {
        let mut doc = Document::demi_additionneur();
        let id = |d: &Document, nom: &str| d.elements.iter().find(|e| e.nom == nom).unwrap().id;
        // On branche la lampe c sur n2 au lieu de la retenue.
        let (c, g2) = (id(&doc, "c"), id(&doc, "g2"));
        doc.relier(
            Source::de(g2),
            Fil {
                puits: c,
                entree: 0,
            },
        );
        // Avec a = b = 0, n2 vaut 1 alors que la retenue attendue est 0.
        let mut s = Session::du_plan(&doc, &[]);
        let mut q = Quete::demi_additionneur();
        for _ in 0..5 {
            s.pas(&doc);
        }
        assert!(s.stable());
        assert_eq!(q.observer(&doc, &s), None);
        let v = q.verifier(&Projet::nouveau("", doc.clone())).clone();
        assert!(!v.reussi);
        assert!(
            v.texte.contains("la lampe c devrait valoir 0"),
            "{}",
            v.texte
        );
        // Une lampe débranchée non plus.
        doc.debrancher(Fil {
            puits: c,
            entree: 0,
        });
        s.suivre(&doc, &Projet::nouveau("", Document::nouveau()));
        for _ in 0..5 {
            s.pas(&doc);
        }
        assert_eq!(q.observer(&doc, &s), None);
    }

    #[test]
    fn chaque_quete_a_ses_broches_et_un_plan_de_depart() {
        let mut cles = BTreeSet::new();
        for f in PARCOURS {
            assert!(cles.insert(f.cle), "clé en double : {}", f.cle);
            let q = Quete::de_fiche(f);
            if f.epreuve.ordinateur() {
                continue;
            }
            assert!(!q.entrees.is_empty(), "{} a des entrées", f.cle);
            assert!(!q.sorties.is_empty(), "{} a des sorties", f.cle);
            let plan = q.plan_de_depart();
            assert!(
                q.reperer(&plan).is_ok(),
                "{} : plan de départ complet",
                f.cle
            );
            // Un plan vide ne réussit jamais.
            let mut q = q;
            assert!(
                !q.verifier(&Projet::nouveau("", plan)).reussi,
                "{} : rien de branché ne passe",
                f.cle
            );
        }
    }

    #[test]
    fn le_demi_additionneur_de_l_atelier_passe_la_verification() {
        let mut q = Quete::demi_additionneur();
        let v = q
            .verifier(&Projet::nouveau("", Document::demi_additionneur()))
            .clone();
        assert!(v.reussi, "{}", v.texte);
        assert!(v.texte.contains("toutes les combinaisons"), "{}", v.texte);
        assert!(q.reussie());
    }

    /// La solution d'une quête faite avec le corrigé lui-même, importé en briques : chaque
    /// interrupteur branché sur la broche du même nom, chaque broche de sortie sur sa lampe.
    fn resolue_avec(q: &Quete, corrige: &str, renommer: &[(&str, &str)]) -> Projet {
        let mut projet = Projet::nouveau("", q.plan_de_depart());
        let mut bib = corriges().clone();
        bib.ajouter("COMPTEUR_NAND", compteur_tout_nand());
        for m in crate::importer::importer(&bib, corrige).unwrap() {
            projet.ajouter(m);
        }
        let plan = &mut projet.principal;
        let mut brique = projet
            .modeles
            .iter()
            .find(|m| m.nom == corrige)
            .unwrap()
            .instance();
        brique.nom = "solution".to_owned();
        let b = plan.poser(brique, Point::nouveau(500.0, 300.0));
        let interieur = |nom: &str| {
            renommer
                .iter()
                .find(|(de, _)| *de == nom)
                .map_or(nom.to_owned(), |(_, vers)| (*vers).to_owned())
        };
        for a in &q.entrees {
            let e = plan.nomme(&a.nom).unwrap().id;
            let i = plan
                .element(b)
                .unwrap()
                .broches_e
                .iter()
                .position(|p| p.nom == interieur(&a.corrige))
                .unwrap();
            assert!(plan.relier(
                Source::de(e),
                Fil {
                    puits: b,
                    entree: i
                }
            ));
        }
        for a in &q.sorties {
            let l = plan.nomme(&a.nom).unwrap().id;
            let j = plan
                .element(b)
                .unwrap()
                .broches_s
                .iter()
                .position(|p| p.nom == interieur(&a.corrige))
                .unwrap();
            assert!(plan.relier(
                Source::nouvelle(b, j),
                Fil {
                    puits: l,
                    entree: 0
                }
            ));
        }
        projet
    }

    /// Un compteur 4 bits sans la primitive constante du corrigé, que la palette n'a pas :
    /// le 1 à ajouter vient d'une NAND qui reçoit raz et son inverse.
    fn compteur_tout_nand() -> ce_circuit::Composant {
        let mut c = ce_circuit::Composant::nouveau("compteur fait de briques posables")
            .entree("raz", 1)
            .entree("clk", 1)
            .sortie("q", 4)
            .inst("nraz", "NON")
            .inst("un", "NAND")
            .fil(&["raz", "nraz.a", "un.a"])
            .fil(&["nraz.s", "un.b"]);
        for i in 0..4 {
            c = c
                .inst(&format!("h{i}"), "DEMI_ADD")
                .inst(&format!("e{i}"), "ET")
                .inst(&format!("b{i}"), "BASCULE_D")
                .fil(&[format!("b{i}.q"), format!("h{i}.a"), format!("q[{i}]")])
                .fil(&[format!("h{i}.s"), format!("e{i}.a")])
                .fil(&[format!("e{i}.s"), format!("b{i}.d")]);
            let retenue = if i == 0 {
                "un.s".to_owned()
            } else {
                format!("h{}.r", i - 1)
            };
            c = c.fil(&[retenue, format!("h{i}.b")]);
        }
        c.fil(&["nraz.s", "e0.b", "e1.b", "e2.b", "e3.b"])
            .fil(&["clk", "b0.clk", "b1.clk", "b2.clk", "b3.clk"])
    }

    #[test]
    fn le_corrige_de_chaque_quete_passe_sa_verification() {
        for f in PARCOURS.iter().filter(|f| !f.epreuve.ordinateur()) {
            let mut q = Quete::de_fiche(f);
            // Le corrigé du compteur utilise une constante que la palette n'a pas : on le
            // construit avec ce qu'un joueur peut poser.
            let corrige = if f.corrige == "COMPTEUR4" {
                "COMPTEUR_NAND"
            } else {
                f.corrige
            };
            let projet = resolue_avec(&q, corrige, &[]);
            let v = q.verifier(&projet).clone();
            assert!(v.reussi, "{} : {}", f.cle, v.texte);
            assert!(q.reussie());
        }
        for cle in ["ordinateur", "ordinateur-majuscules"] {
            let mut q = Quete::de_cle(cle).unwrap();
            let Epreuve::Ordinateur { programme, .. } = q.epreuve else {
                unreachable!()
            };
            let mut projet =
                crate::exemples::atelier_de_l_ordinateur("essai", programme, true).unwrap();
            assert!(!q.verifier(&projet).reussi, "{cle} : rien n'est branché");
            crate::exemples::tests::brancher_l_atelier(&mut projet);
            let v = q.verifier(&projet).clone();
            assert!(v.reussi, "{cle} : {}", v.texte);
        }
    }

    #[test]
    fn un_composant_qui_ressemble_ne_suffit_pas() {
        // Un verrou D à la place de la bascule D : transparent tant que clk vaut 1, au lieu
        // de ne copier d qu'au front montant.
        let mut q = Quete::de_cle("bascule-d").unwrap();
        let projet = resolue_avec(&q, "VERROU_D", &[("clk", "e")]);
        let v = q.verifier(&projet).clone();
        assert!(!v.reussi);
        assert!(v.texte.contains("essai"), "{}", v.texte);
        // Un OU à la place du OU exclusif : faux seulement quand a = b = 1.
        let mut q = Quete::de_cle("ou-exclusif").unwrap();
        let v = q.verifier(&resolue_avec(&q, "OU", &[])).clone();
        assert_eq!(
            v.texte,
            "Avec a = 1, b = 1, la lampe s devrait valoir 0 ; elle montre 1."
        );
        // Le demi-additionneur à la place de l'additionneur complet : la retenue qui entre
        // est oubliée. Les broches manquent : la vérification le dit.
        let mut q = Quete::de_cle("additionneur-complet").unwrap();
        let mut plan = q.plan_de_depart();
        let re = plan.nomme("re").unwrap().id;
        plan.retirer(re);
        let v = q.verifier(&Projet::nouveau("", plan)).clone();
        assert_eq!(v.texte, "Il manque l'interrupteur « re » sur le plan.");
    }

    #[test]
    fn une_quete_reussie_donne_sa_brique_aux_suivantes() {
        use crate::parcours::Parcours;
        let mut parcours = Parcours::default();
        // La porte NON faite d'une NAND aux deux entrées reliées.
        let fiche = Quete::fiche("non").unwrap();
        let mut projet = parcours.projet(fiche, "non").unwrap();
        let plan = &mut projet.principal;
        let (a, s) = (plan.nomme("a").unwrap().id, plan.nomme("s").unwrap().id);
        let g = plan.ajouter_porte(Point::nouveau(400.0, 300.0));
        for entree in 0..2 {
            plan.relier(Source::de(a), Fil { puits: g, entree });
        }
        plan.relier(
            Source::de(g),
            Fil {
                puits: s,
                entree: 0,
            },
        );
        let mut q = Quete::de_fiche(fiche);
        assert!(q.verifier(&projet).reussi);
        parcours.gagner(&q, &projet);
        assert!(parcours.faite("non"));
        assert_eq!(parcours.briques.len(), 1);
        assert_eq!(parcours.briques[0].nom, "NON");
        assert_eq!(parcours.briques[0].plan, projet.principal);
        assert_eq!(parcours.prochaine().unwrap().cle, "et");
        // La quête suivante a la porte NON dans sa palette.
        let et = parcours
            .projet(Parcours::suivante("non").unwrap(), "et")
            .unwrap();
        assert!(et.modele("NON").is_some());
        assert_eq!(et.quete.as_deref(), Some("et"));
        // Rejouer la porte NON avec la brique NON elle-même ne casse rien.
        let mut rejoue = parcours.projet(fiche, "non 2").unwrap();
        let mut brique = rejoue.modele("NON").unwrap().instance();
        brique.nom = "n".to_owned();
        let plan = &mut rejoue.principal;
        let (a, s) = (plan.nomme("a").unwrap().id, plan.nomme("s").unwrap().id);
        let b = plan.poser(brique, Point::nouveau(400.0, 300.0));
        plan.relier(
            Source::de(a),
            Fil {
                puits: b,
                entree: 0,
            },
        );
        plan.relier(
            Source::de(b),
            Fil {
                puits: s,
                entree: 0,
            },
        );
        assert!(q.verifier(&rejoue).reussi);
        parcours.gagner(&q, &rejoue);
        assert_eq!(parcours.briques.len(), 1);
        assert_eq!(
            parcours.briques[0].plan, projet.principal,
            "la brique d'avant reste"
        );
        // Le parcours se relit à l'identique.
        let relu = Parcours::de_json(&parcours.en_json()).unwrap();
        assert_eq!(relu.faites, parcours.faites);
        assert_eq!(relu.briques, parcours.briques);
    }

    /// Ajoute au projet le corrigé `nom` et tout ce qu'il contient.
    fn importer_le_corrige(p: &mut Projet, nom: &str) {
        for m in crate::importer::importer(corriges(), nom).unwrap() {
            if p.modele(&m.nom).is_none() {
                p.ajouter(m);
            }
        }
    }

    /// Branche deux ordinateurs par une boîte aux lettres, comme le dit la consigne.
    fn brancher_les_deux(p: &mut Projet) {
        crate::solution::brancher_les_deux(p).expect("les deux ordinateurs se branchent");
    }

    #[test]
    fn les_grands_projets_marchent_une_fois_branches() {
        let grands: Vec<&Fiche> = PARCOURS
            .iter()
            .filter(|f| matches!(f.epreuve, Epreuve::Systeme { .. }))
            .collect();
        assert_eq!(grands.len(), 3);
        for f in grands {
            let Epreuve::Systeme { atelier, .. } = f.epreuve else {
                unreachable!()
            };
            let mut q = Quete::de_fiche(f);
            let mut p = crate::exemples::atelier_du_grand_projet("essai", atelier).unwrap();
            assert!(!q.verifier(&p).reussi, "{} : rien n'est branché", f.cle);
            match atelier {
                Atelier::Deux { .. } => brancher_les_deux(&mut p),
                Atelier::GrandeMemoire { .. } => {
                    importer_le_corrige(&mut p, "RAM256");
                    let mut ram = p.modele("RAM256").unwrap().instance();
                    ram.nom = "ram".to_owned();
                    p.principal.poser(ram, Point::nouveau(820.0, 520.0));
                    crate::exemples::tests::brancher_l_atelier(&mut p);
                }
            }
            let v = q.verifier(&p).clone();
            assert!(v.reussi, "{} : {}", f.cle, v.texte);
        }
    }

    /// Sans la boîte aux lettres, le clavier branché droit sur B : le message n'est pas
    /// chiffré puis déchiffré, l'écran se trompe.
    #[test]
    fn la_messagerie_secrete_ne_se_contourne_pas() {
        let f = Quete::fiche("messagerie-chiffree").unwrap();
        let Epreuve::Systeme { atelier, .. } = f.epreuve else {
            unreachable!()
        };
        let mut q = Quete::de_fiche(f);
        let mut p = crate::exemples::atelier_du_grand_projet("essai", atelier).unwrap();
        let plan = &mut p.principal;
        let id = |plan: &Document, nom: &str| plan.nomme(nom).unwrap().id;
        let (b, raz, clavier, ecran) = (
            id(plan, "b"),
            id(plan, "raz"),
            id(plan, "clavier"),
            id(plan, "ecran"),
        );
        let broche = |plan: &Document, nom: &str| {
            plan.element(b)
                .unwrap()
                .broches_e
                .iter()
                .position(|x| x.nom == nom)
                .unwrap()
        };
        let (er, ec) = (broche(plan, "raz"), broche(plan, "clavier"));
        assert!(plan.relier(
            Source::de(raz),
            Fil {
                puits: b,
                entree: er
            }
        ));
        assert!(plan.relier(
            Source::de(clavier),
            Fil {
                puits: b,
                entree: ec
            }
        ));
        assert!(plan.relier(
            Source::nouvelle(b, 0),
            Fil {
                puits: ecran,
                entree: 0
            }
        ));
        assert!(plan.relier(
            Source::nouvelle(b, 1),
            Fil {
                puits: ecran,
                entree: 1
            }
        ));
        let v = q.verifier(&p).clone();
        assert!(!v.reussi);
        assert!(v.texte.contains("G@K"), "{}", v.texte);
    }

    #[test]
    fn l_ordinateur_de_a_a_z_marche_avec_les_briques_gagnees() {
        use crate::parcours::Parcours;
        let mut parcours = Parcours::default();
        for cle in ["processeur", "memoire-vive-16"] {
            let mut q = Quete::de_cle(cle).unwrap();
            let projet = resolue_avec(&q, Quete::fiche(cle).unwrap().corrige, &[]);
            assert!(q.verifier(&projet).reussi, "{cle}");
            parcours.gagner(&q, &projet);
        }
        let fiche = Quete::fiche("ordinateur-a-toi").unwrap();
        let mut projet = parcours.projet(fiche, "a-toi").unwrap();
        let plan = &projet.principal;
        assert!(plan.nomme("rom").is_some());
        assert!(
            plan.nomme("cpu").is_none(),
            "le processeur n'est pas fourni"
        );
        let mut q = Quete::de_fiche(fiche);
        assert!(!q.verifier(&projet).reussi);
        for (modele, nom) in [("PROCESSEUR", "cpu"), ("RAM16", "ram")] {
            let mut b = projet.modele(modele).unwrap().instance();
            b.nom = nom.to_owned();
            projet.principal.poser(b, Point::nouveau(600.0, 300.0));
        }
        crate::exemples::tests::brancher_l_atelier(&mut projet);
        let v = q.verifier(&projet).clone();
        assert!(v.reussi, "{}", v.texte);
    }
}
