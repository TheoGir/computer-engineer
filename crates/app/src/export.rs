//! Export d'un plan en Verilog structurel : un module par plan, une primitive `nand` par
//! porte NAND, avec son délai. On peut ainsi simuler son circuit dans un autre outil
//! (Icarus Verilog, un simulateur en ligne) ou le charger dans un vrai circuit programmable
//! (FPGA).

use std::collections::BTreeMap;
use std::fmt::Write;

use ce_circuit::Sens;
use ce_moteur::GenrePorte;

use crate::brique::Projet;
use crate::document::{Document, Genre, Id};
use crate::pont::{Briques, Pont};

/// Un nom permis en Verilog : lettres sans accent, chiffres et `_`, sans chiffre au début, ni
/// mot réservé.
pub fn identifiant(nom: &str) -> String {
    let sans_accent = |c: char| -> Option<char> {
        Some(match c {
            'à' | 'â' | 'ä' | 'á' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'À' | 'Â' => 'A',
            'É' | 'È' | 'Ê' => 'E',
            'Ç' => 'C',
            c if c.is_ascii_alphanumeric() || c == '_' => c,
            _ => return None,
        })
    };
    let mut id: String = nom.chars().map(|c| sans_accent(c).unwrap_or('_')).collect();
    if id.is_empty() || id.starts_with(|c: char| c.is_ascii_digit()) {
        id.insert(0, '_');
    }
    const RESERVES: [&str; 24] = [
        "module",
        "endmodule",
        "input",
        "output",
        "inout",
        "wire",
        "reg",
        "assign",
        "always",
        "initial",
        "begin",
        "end",
        "if",
        "else",
        "case",
        "for",
        "nand",
        "and",
        "or",
        "not",
        "nor",
        "xor",
        "xnor",
        "buf",
    ];
    if RESERVES.contains(&id.as_str()) {
        id.push('_');
    }
    id
}

/// Le texte Verilog du plan `plan` du projet, en un module nommé d'après `nom`.
pub fn verilog(projet: &Projet, plan: &Document, nom: &str) -> Result<String, String> {
    let mut briques = Briques::default();
    briques.suivre(projet);
    let circuit = Pont::construire(plan, projet, &briques)?.circuit;
    let net = &circuit.netlist;

    // Les ports : le nom de chaque interrupteur, clavier ou lampe, rendu unique.
    let elements: BTreeMap<Id, &crate::document::Element> =
        plan.elements.iter().map(|e| (e.id, e)).collect();
    let mut pris: Vec<String> = Vec::new();
    let mut ports: Vec<(String, Sens, Vec<u32>)> = Vec::new();
    for b in &circuit.broches {
        let element = b
            .nom
            .get(1..)
            .and_then(|n| n.parse::<Id>().ok())
            .and_then(|id| elements.get(&id));
        let base = identifiant(element.map_or(b.nom.as_str(), |e| e.nom.as_str()));
        let mut id = base.clone();
        let mut k = 2;
        while pris.contains(&id) || id.starts_with("n_") || id.starts_with("g_") {
            id = format!("{base}_{k}");
            k += 1;
        }
        pris.push(id.clone());
        ports.push((id, b.sens, b.nets.clone()));
    }

    let module = identifiant(nom);
    let portes_nand = net
        .portes
        .iter()
        .filter(|p| p.genre == GenrePorte::Nand)
        .count();
    let mut v = String::new();
    let _ = writeln!(
        v,
        "// « {nom} », exporté par Computer Engineer {} : {portes_nand} portes NAND.",
        env!("CARGO_PKG_VERSION")
    );
    let _ = writeln!(
        v,
        "// Une primitive nand par porte NAND du plan, briques mises à plat, avec son délai en"
    );
    let _ = writeln!(
        v,
        "// unités de temps (une unité = 1 ns ici). Simulation : iverilog -o sim {module}.v"
    );
    if plan.elements.iter().any(|e| e.genre == Genre::Ecran) {
        let _ = writeln!(
            v,
            "// L'écran du plan n'est pas exporté : ses entrées restent des fils internes."
        );
    }
    let _ = writeln!(v, "`timescale 1ns/1ns");
    let liste: Vec<String> = ports.iter().map(|(n, ..)| n.clone()).collect();
    let _ = writeln!(v, "module {module}({});", liste.join(", "));
    for (n, sens, nets) in &ports {
        let sens = match sens {
            Sens::Entree => "input",
            Sens::Sortie => "output",
        };
        if nets.len() == 1 {
            let _ = writeln!(v, "  {sens} {n};");
        } else {
            let _ = writeln!(v, "  {sens} [{}:0] {n};", nets.len() - 1);
        }
    }
    if net.nb_nets > 0 {
        let fils: Vec<String> = (0..net.nb_nets).map(|i| format!("n_{i}")).collect();
        for morceau in fils.chunks(16) {
            let _ = writeln!(v, "  wire {};", morceau.join(", "));
        }
    }
    for (n, sens, nets) in &ports {
        for (k, net) in nets.iter().enumerate() {
            let bit = if nets.len() == 1 {
                n.clone()
            } else {
                format!("{n}[{k}]")
            };
            match sens {
                Sens::Entree => {
                    let _ = writeln!(v, "  assign n_{net} = {bit};");
                }
                Sens::Sortie => {
                    let _ = writeln!(v, "  assign {bit} = n_{net};");
                }
            }
        }
    }
    for (net, valeur) in &net.constantes {
        let _ = writeln!(v, "  assign n_{net} = 1'b{valeur};");
    }
    for (i, h) in net.horloges.iter().enumerate() {
        let _ = writeln!(
            v,
            "  // Horloge interne (simulation seulement) : pour un FPGA, remplace-la par l'horloge de la carte."
        );
        let _ = writeln!(v, "  reg h_{i};");
        let _ = writeln!(v, "  initial h_{i} = 1'b0;");
        let _ = writeln!(v, "  always #{} h_{i} = ~h_{i};", h.demi_periode);
        let _ = writeln!(v, "  assign n_{} = h_{i};", h.net);
    }
    for (g, p) in net.portes.iter().enumerate() {
        match p.genre {
            GenrePorte::Nand => {
                let _ = writeln!(
                    v,
                    "  nand #{} g_{g} (n_{}, n_{}, n_{});",
                    p.delai, p.s, p.e0, p.e1
                );
            }
            GenrePorte::Tri => {
                let _ = writeln!(
                    v,
                    "  bufif1 #{} g_{g} (n_{}, n_{}, n_{});",
                    p.delai, p.s, p.e1, p.e0
                );
            }
        }
    }
    let _ = writeln!(v, "endmodule");
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_noms_deviennent_des_identifiants_verilog() {
        assert_eq!(identifiant("somme"), "somme");
        assert_eq!(identifiant("retenue entrée"), "retenue_entree");
        assert_eq!(identifiant("2e"), "_2e");
        assert_eq!(identifiant("module"), "module_");
        assert_eq!(identifiant(""), "_");
        assert_eq!(identifiant("Zoé"), "Zoe");
    }

    #[test]
    fn le_demi_additionneur_s_exporte_en_cinq_nand() {
        let doc = Document::demi_additionneur();
        let projet = Projet::nouveau("Premier circuit", doc.clone());
        let v = verilog(&projet, &doc, "Premier circuit").unwrap();
        assert!(v.contains("module Premier_circuit(a, b, s, c);"), "{v}");
        assert!(v.contains("  input a;"));
        assert!(v.contains("  output c;"));
        assert_eq!(v.matches("  nand #").count(), 5, "{v}");
        assert!(v.trim_end().ends_with("endmodule"));
    }

    fn icarus_disponible() -> bool {
        let present = std::process::Command::new("iverilog")
            .arg("-V")
            .output()
            .is_ok();
        if !present {
            assert!(
                std::env::var("CE_EXIGER_ICARUS").as_deref() != Ok("1"),
                "iverilog est exigé (CE_EXIGER_ICARUS=1) mais introuvable"
            );
            eprintln!("iverilog introuvable : export non vérifié par Icarus");
        }
        present
    }

    /// Compile `source` avec Icarus Verilog et, si `executer`, l'exécute ; rend ce qu'il
    /// affiche.
    fn icarus(etiquette: &str, source: &str, executer: bool) -> String {
        let dossier =
            std::env::temp_dir().join(format!("ce-export-{}-{etiquette}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        let fichier = dossier.join("export.v");
        let binaire = dossier.join("export.vvp");
        std::fs::write(&fichier, source).unwrap();
        let compile = std::process::Command::new("iverilog")
            .arg("-o")
            .arg(&binaire)
            .arg(&fichier)
            .output()
            .unwrap();
        assert!(
            compile.status.success(),
            "iverilog refuse l'export : {}",
            String::from_utf8_lossy(&compile.stderr)
        );
        let sortie = if executer {
            let sortie = std::process::Command::new("vvp")
                .arg(&binaire)
                .output()
                .unwrap();
            String::from_utf8_lossy(&sortie.stdout).into_owned()
        } else {
            String::new()
        };
        let _ = std::fs::remove_dir_all(&dossier);
        sortie
    }

    /// Le demi-additionneur exporté, simulé par Icarus Verilog, donne la bonne table.
    #[test]
    fn l_export_du_demi_additionneur_calcule_juste_dans_icarus() {
        if !icarus_disponible() {
            return;
        }
        let doc = Document::demi_additionneur();
        let projet = Projet::nouveau("Premier circuit", doc.clone());
        let mut v = verilog(&projet, &doc, "demi").unwrap();
        v.push_str(
            "module banc;\n  reg a, b;\n  wire s, c;\n  demi d(a, b, s, c);\n  integer i;\n  initial begin\n    for (i = 0; i < 4; i = i + 1) begin\n      a = i[0]; b = i[1];\n      #20 $display(\"%b%b %b%b\", a, b, s, c);\n    end\n  end\nendmodule\n",
        );
        let sortie = icarus("demi", &v, true);
        assert_eq!(sortie, "00 00\n10 10\n01 10\n11 01\n", "{sortie}");
    }

    /// L'ordinateur miniature entier, briques mises à plat, s'exporte en un Verilog
    /// qu'Icarus accepte (compilé, pas exécuté).
    #[test]
    fn l_export_de_l_ordinateur_est_accepte_par_icarus() {
        if !icarus_disponible() {
            return;
        }
        let (_, programme) = crate::programme::PROGRAMMES[0];
        let projet = crate::exemples::ordinateur("Ordinateur", programme).unwrap();
        let v = verilog(&projet, &projet.principal, "Ordinateur").unwrap();
        assert!(v.matches(" nand #").count() > 1000);
        // Compilé seulement : son horloge interne bat sans fin.
        icarus("ordinateur", &v, false);
    }
}
