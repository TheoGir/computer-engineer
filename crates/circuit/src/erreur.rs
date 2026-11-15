//! Erreurs de la bibliothèque de composants.

use ce_moteur::ErreurNetlist;
use std::fmt;

/// Tout ce qui peut empêcher de lire, de valider ou de mettre à plat un composant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Erreur {
    /// Le fichier n'est pas du JSON valide ou n'a pas la forme attendue.
    Json(String),
    /// Le champ `format` n'est pas celui que ce programme sait lire.
    FormatInconnu(String),
    /// Aucun composant de ce nom dans la bibliothèque.
    ComposantInconnu { nom: String, dans: Option<String> },
    /// Un composant s'utilise lui-même, directement ou non. Le cycle est donné
    /// dans l'ordre, le premier nom étant répété à la fin.
    DependanceCirculaire(Vec<String>),
    /// Un nom est vide ou contient un caractère interdit (`.`, `[`, `]`, `:`, espace).
    NomInvalide { composant: String, nom: String },
    /// Un composant porte le nom d'une primitive.
    NomReserve(String),
    /// Deux broches ou deux instances du même composant portent le même nom.
    NomEnDouble { composant: String, nom: String },
    /// Une extrémité de fil est mal écrite.
    Syntaxe {
        composant: String,
        point: String,
        raison: String,
    },
    /// Une extrémité de fil désigne une instance qui n'existe pas.
    InstanceInconnue { composant: String, instance: String },
    /// Une extrémité de fil désigne une broche qui n'existe pas.
    BrocheInconnue { composant: String, point: String },
    /// Un indice de bit dépasse la largeur de la broche.
    BitHorsBornes {
        composant: String,
        point: String,
        largeur: u32,
    },
    /// Les extrémités d'un fil n'ont pas toutes le même nombre de bits.
    LargeursDifferentes { composant: String, fil: Vec<String> },
    /// Une broche a une largeur nulle.
    LargeurNulle { composant: String, broche: String },
    /// Un paramètre est manquant, nul ou hors de propos pour ce type d'instance.
    Parametre {
        composant: String,
        instance: String,
        raison: String,
    },
    /// Le circuit mis à plat dépasse la capacité du moteur.
    TropGrand,
    /// Le chemin demandé ne désigne rien dans le circuit.
    Chemin { chemin: String, raison: String },
    /// L'analyse demandée ne s'applique pas à ce circuit.
    Analyse(String),
    /// Erreur du moteur.
    Netlist(ErreurNetlist),
}

impl fmt::Display for Erreur {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Erreur::Json(m) => write!(f, "fichier illisible : {m}"),
            Erreur::FormatInconnu(m) => write!(f, "format de bibliothèque inconnu : « {m} »"),
            Erreur::ComposantInconnu { nom, dans: Some(d) } => {
                write!(f, "le composant « {d} » utilise « {nom} », qui n'existe pas")
            }
            Erreur::ComposantInconnu { nom, dans: None } => {
                write!(f, "le composant « {nom} » n'existe pas")
            }
            Erreur::DependanceCirculaire(cycle) => {
                write!(f, "dépendance circulaire : {}", cycle.join(" → "))
            }
            Erreur::NomInvalide { composant, nom } => {
                write!(f, "dans « {composant} » : le nom « {nom} » est invalide")
            }
            Erreur::NomReserve(nom) => write!(f, "« {nom} » est le nom d'une primitive"),
            Erreur::NomEnDouble { composant, nom } => {
                write!(f, "dans « {composant} » : le nom « {nom} » est utilisé deux fois")
            }
            Erreur::Syntaxe { composant, point, raison } => {
                write!(f, "dans « {composant} » : extrémité « {point} » mal écrite ({raison})")
            }
            Erreur::InstanceInconnue { composant, instance } => {
                write!(f, "dans « {composant} » : l'instance « {instance} » n'existe pas")
            }
            Erreur::BrocheInconnue { composant, point } => {
                write!(f, "dans « {composant} » : la broche « {point} » n'existe pas")
            }
            Erreur::BitHorsBornes { composant, point, largeur } => write!(
                f,
                "dans « {composant} » : « {point} » dépasse la largeur de la broche ({largeur} bits)"
            ),
            Erreur::LargeursDifferentes { composant, fil } => write!(
                f,
                "dans « {composant} » : le fil [{}] relie des extrémités de largeurs différentes",
                fil.join(", ")
            ),
            Erreur::LargeurNulle { composant, broche } => {
                write!(f, "dans « {composant} » : la broche « {broche} » a une largeur nulle")
            }
            Erreur::Parametre { composant, instance, raison } => {
                write!(f, "dans « {composant} », instance « {instance} » : {raison}")
            }
            Erreur::TropGrand => write!(f, "circuit trop grand (plus de 2^31 portes ou 2^32 nets)"),
            Erreur::Chemin { chemin, raison } => write!(f, "chemin « {chemin} » : {raison}"),
            Erreur::Analyse(m) => write!(f, "{m}"),
            Erreur::Netlist(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Erreur {}

impl From<ErreurNetlist> for Erreur {
    fn from(e: ErreurNetlist) -> Self {
        Erreur::Netlist(e)
    }
}
