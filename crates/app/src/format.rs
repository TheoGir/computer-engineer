//! Les formats de fichier de l'application, numérotés, et leurs migrations.
//!
//! Chaque fichier enregistré porte son format, `computer-engineer/<genre>/<numéro>`. La règle,
//! figée à la version 1.0.0 (`docs/formats.md`) :
//! - un changement de ce qui est écrit (champ ajouté, retiré, renommé, sens changé) prend le
//!   numéro suivant et une migration du numéro d'avant vers lui, testée sur le corpus de
//!   fichiers réels de `tests/donnees/` ; une épreuve compare la forme écrite à son relevé et
//!   échoue tant que ce n'est pas fait ;
//! - un fichier d'un numéro plus ancien est migré à la lecture, pas à pas, et l'original est
//!   gardé à part avant que la version migrée ne l'écrase ;
//! - un fichier d'un numéro plus récent (écrit par une version plus récente de l'application)
//!   est refusé avec un message clair, et n'est jamais réécrit : rien ne s'y perd.

use serde_json::Value;

/// Une migration : transforme sur place un fichier du numéro `n` en fichier du numéro `n + 1`.
pub type Migration = fn(&mut Value) -> Result<(), String>;

/// Un format de fichier numéroté.
#[derive(Clone, Copy, Debug)]
pub struct Format {
    /// Ce que le fichier contient, pour les messages : « projet », « avancement ».
    pub genre: &'static str,
    /// Le début de l'identifiant, jusqu'au numéro : `computer-engineer/projet/`.
    pub prefixe: &'static str,
    /// Le numéro écrit par cette version de l'application.
    pub actuel: u32,
    /// `migrations[k]` mène du numéro `k + 1` au numéro `k + 2` : il y en a `actuel - 1`.
    pub migrations: &'static [Migration],
}

/// Le format des projets.
pub const PROJET: Format = Format {
    genre: "projet",
    prefixe: "computer-engineer/projet/",
    actuel: 5,
    migrations: &[
        |v| programme_des_memoires(v, "modeles"),
        rien_a_changer,
        rien_a_changer,
        rien_a_changer,
    ],
};

/// Le format de l'avancement dans le parcours (`parcours.json`).
pub const PARCOURS: Format = Format {
    genre: "avancement",
    prefixe: "computer-engineer/parcours/",
    actuel: 4,
    migrations: &[
        |v| programme_des_memoires(v, "briques"),
        rien_a_changer,
        rien_a_changer,
    ],
};

/// Migration 2 → 3 (projets et avancement) : les éléments gagnent le champ facultatif
/// `aspect` (bouton poussoir, afficheur 7 segments). Absent, il vaut « normal » : un fichier
/// du format 2 se lit tel quel. Le numéro change pour qu'une version plus ancienne refuse
/// clairement un fichier qui en contient, au lieu d'en perdre l'aspect.
///
/// Migration 3 → 4 (projets seulement) : le projet gagne le champ facultatif `bancs`, les
/// bancs d'essai écrits par l'utilisateur. Absent, il n'y en a aucun ; même raison.
///
/// Migration 4 → 5 des projets, 3 → 4 de l'avancement : les éléments gagnent le champ
/// facultatif `miroir` (élément retourné de gauche à droite). Absent, il vaut « non ».
fn rien_a_changer(_: &mut Value) -> Result<(), String> {
    Ok(())
}

/// Migration 1 → 2 (projets et avancement) : les briques gagnent le champ `programme`. Une
/// mémoire morte livrée avec l'application, reconnue à son circuit, retrouve le programme dont
/// elle est faite ; toutes les autres briques restent sans programme.
fn programme_des_memoires(v: &mut Value, champ: &str) -> Result<(), String> {
    let Some(modeles) = v.get_mut(champ).and_then(Value::as_array_mut) else {
        return Ok(());
    };
    for m in modeles {
        let Some(plan) = m.get("plan") else {
            continue;
        };
        if let Some(texte) = crate::programme::programme_reconnu(plan) {
            if let Some(objet) = m.as_object_mut() {
                objet.insert("programme".to_owned(), Value::String(texte.to_owned()));
            }
        }
    }
    Ok(())
}

/// Ce qu'une lecture a dû faire.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lecture {
    /// Le fichier était déjà au format actuel.
    Actuel,
    /// Le fichier a été migré depuis ce numéro.
    Migre(u32),
}

impl Format {
    /// L'identifiant du format actuel, tel qu'il s'écrit dans les fichiers.
    pub fn identifiant(&self) -> String {
        format!("{}{}", self.prefixe, self.actuel)
    }

    /// Le numéro d'un identifiant de ce genre de fichier.
    pub fn numero(&self, identifiant: &str) -> Result<u32, String> {
        identifiant
            .strip_prefix(self.prefixe)
            .and_then(|n| n.parse::<u32>().ok())
            .filter(|n| *n >= 1)
            .ok_or_else(|| tr!("format de {genre} inconnu : « {identifiant} »", identifiant = identifiant, genre = self.genre))
    }

    /// Lit un fichier de ce format et le mène, migration après migration, au format actuel.
    /// Le champ `format` du résultat est celui d'aujourd'hui.
    pub fn lire(&self, texte: &str) -> Result<(Value, Lecture), String> {
        debug_assert_eq!(self.migrations.len() + 1, self.actuel as usize);
        let texte = texte.trim_start_matches('\u{feff}');
        let mut valeur: Value =
            serde_json::from_str(texte).map_err(|e| tr!("fichier illisible : {e}", e = e))?;
        let identifiant = valeur
            .get("format")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                tr!("fichier illisible : ce n'est pas un fichier de {genre}", genre = self.genre)
            })?
            .to_owned();
        let numero = self.numero(&identifiant)?;
        if numero > self.actuel {
            return Err(tr!("ce fichier vient d'une version plus récente de Computer Engineer (format {numero} ; celle-ci lit jusqu'au format {actuel}). Mets l'application à jour pour l'ouvrir. Il n'a pas été modifié.", numero = numero, actuel = self.actuel));
        }
        for (k, migrer) in self.migrations[numero as usize - 1..].iter().enumerate() {
            migrer(&mut valeur).map_err(|e| {
                tr!("le passage du format {v} au format {v2} a échoué : {e}", e = e, v = numero as usize + k, v2 = numero as usize + k + 1)
            })?;
        }
        if let Some(objet) = valeur.as_object_mut() {
            objet.insert("format".to_owned(), Value::String(self.identifiant()));
        }
        let lecture = if numero == self.actuel {
            Lecture::Actuel
        } else {
            Lecture::Migre(numero)
        };
        Ok((valeur, lecture))
    }
}

/// La forme d'un document JSON : tous les chemins de clés qu'il contient, les éléments d'un
/// tableau confondus (`principal.elements[].genre`). Sert à remarquer qu'un format a changé.
pub fn forme(valeur: &Value) -> std::collections::BTreeSet<String> {
    fn parcourir(v: &Value, chemin: &str, sortie: &mut std::collections::BTreeSet<String>) {
        match v {
            Value::Object(champs) => {
                for (cle, sous) in champs {
                    let ici = if chemin.is_empty() {
                        cle.clone()
                    } else {
                        format!("{chemin}.{cle}")
                    };
                    sortie.insert(ici.clone());
                    parcourir(sous, &ici, sortie);
                }
            }
            Value::Array(elements) => {
                for sous in elements {
                    parcourir(sous, &format!("{chemin}[]"), sortie);
                }
            }
            _ => {}
        }
    }
    let mut sortie = std::collections::BTreeSet::new();
    parcourir(valeur, "", &mut sortie);
    sortie
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ajoute_un_champ(v: &mut Value) -> Result<(), String> {
        v["ajoute"] = Value::from(2);
        Ok(())
    }
    fn renomme(v: &mut Value) -> Result<(), String> {
        let ancien = v
            .as_object_mut()
            .and_then(|o| o.remove("nom"))
            .ok_or("pas de nom")?;
        v["titre"] = ancien;
        Ok(())
    }
    const ESSAI: Format = Format {
        genre: "essai",
        prefixe: "ce/essai/",
        actuel: 3,
        migrations: &[ajoute_un_champ, renomme],
    };

    #[test]
    fn un_fichier_ancien_monte_pas_a_pas() {
        let (v, l) = ESSAI.lire(r#"{"format":"ce/essai/1","nom":"a"}"#).unwrap();
        assert_eq!(l, Lecture::Migre(1));
        assert_eq!(
            v,
            serde_json::json!({"format":"ce/essai/3","titre":"a","ajoute":2})
        );
        let (v, l) = ESSAI.lire(r#"{"format":"ce/essai/2","nom":"b"}"#).unwrap();
        assert_eq!(l, Lecture::Migre(2));
        assert_eq!(v, serde_json::json!({"format":"ce/essai/3","titre":"b"}));
        let (_, l) = ESSAI
            .lire(r#"{"format":"ce/essai/3","titre":"c"}"#)
            .unwrap();
        assert_eq!(l, Lecture::Actuel);
    }

    #[test]
    fn une_migration_qui_echoue_dit_laquelle() {
        let e = ESSAI.lire(r#"{"format":"ce/essai/2"}"#).unwrap_err();
        assert!(e.contains("format 2 au format 3"), "{e}");
    }

    #[test]
    fn un_fichier_plus_recent_est_refuse_clairement() {
        let e = ESSAI.lire(r#"{"format":"ce/essai/4"}"#).unwrap_err();
        assert!(e.contains("plus récente") && e.contains("format 4"), "{e}");
        assert!(e.contains("pas été modifié"), "{e}");
    }

    #[test]
    fn identifiants_etrangers_ou_abimes_refuses() {
        for texte in [
            r#"{"format":"ce/autre/1"}"#,
            r#"{"format":"ce/essai/0"}"#,
            r#"{"format":"ce/essai/x"}"#,
            r#"{"format":"ce/essai/-1"}"#,
            r#"{"format":3}"#,
            r#"{}"#,
            r#"[1,2]"#,
            r#"{"format":"ce/essai/1""#,
            "",
        ] {
            assert!(ESSAI.lire(texte).is_err(), "{texte}");
        }
    }

    #[test]
    fn marque_d_ordre_des_octets_toleree() {
        assert!(ESSAI.lire("\u{feff}{\"format\":\"ce/essai/3\"}").is_ok());
    }

    #[test]
    fn formes_du_projet_et_du_parcours_coherentes() {
        assert_eq!(PROJET.migrations.len() + 1, PROJET.actuel as usize);
        assert_eq!(PARCOURS.migrations.len() + 1, PARCOURS.actuel as usize);
        assert_eq!(PROJET.identifiant(), crate::brique::FORMAT);
        assert_eq!(PARCOURS.identifiant(), crate::parcours::FORMAT);
    }

    #[test]
    fn forme_confond_les_elements_des_tableaux() {
        let v = serde_json::json!({"a":[{"b":1},{"c":2}],"d":{"e":null}});
        let f: Vec<_> = forme(&v).into_iter().collect();
        assert_eq!(f, ["a", "a[].b", "a[].c", "d", "d.e"]);
    }
}
