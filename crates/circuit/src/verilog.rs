//! Export Verilog d'une netlist, pour la comparer à un simulateur établi.
//!
//! Le module généré est structurel : une primitive `nand` par porte NAND, une
//! primitive `bufif1` par tampon trois états, chacune avec son délai. Le banc de
//! test applique un stimulus et affiche la valeur de tous les nets à la fin de
//! chaque unité de temps (`$strobe`).

use ce_moteur::{GenrePorte, Netlist, Valeur};
use std::fmt::Write;

/// Génère un banc de test Verilog autonome.
///
/// `stimulus[t][i]` est la valeur de l'entrée `i` à l'instant `t`. Le programme
/// affiche une ligne par instant : la valeur des nets `0, 1, 2, …` dans cet ordre,
/// en caractères `0`, `1`, `x`, `z`.
pub fn banc_de_test(netlist: &Netlist, stimulus: &[Vec<Valeur>]) -> String {
    let mut v = String::new();
    let _ = writeln!(v, "`timescale 1ns/1ns");
    let _ = writeln!(v, "module banc;");
    for n in 0..netlist.nb_nets {
        let _ = writeln!(v, "  wire n{n};");
    }
    for (i, net) in netlist.entrees.iter().enumerate() {
        let _ = writeln!(v, "  reg e{i};");
        let _ = writeln!(v, "  assign n{net} = e{i};");
    }
    for (i, (net, valeur)) in netlist.constantes.iter().enumerate() {
        let _ = writeln!(v, "  assign n{net} = 1'b{valeur}; // constante {i}");
    }
    for (i, h) in netlist.horloges.iter().enumerate() {
        let _ = writeln!(v, "  reg h{i};");
        let _ = writeln!(v, "  assign n{} = h{i};", h.net);
        let _ = writeln!(v, "  initial h{i} = 1'b0;");
        let _ = writeln!(v, "  always #{} h{i} = ~h{i};", h.demi_periode);
    }
    for (g, p) in netlist.portes.iter().enumerate() {
        match p.genre {
            GenrePorte::Nand => {
                let _ = writeln!(
                    v,
                    "  nand #{} g{g} (n{}, n{}, n{});",
                    p.delai, p.s, p.e0, p.e1
                );
            }
            // bufif1 (sortie, donnée, validation)
            GenrePorte::Tri => {
                let _ = writeln!(
                    v,
                    "  bufif1 #{} g{g} (n{}, n{}, n{});",
                    p.delai, p.s, p.e1, p.e0
                );
            }
        }
    }
    let _ = writeln!(v, "  initial begin");
    for (t, ligne) in stimulus.iter().enumerate() {
        if t > 0 {
            let _ = writeln!(v, "    #1;");
        }
        for (i, valeur) in ligne.iter().enumerate() {
            if t == 0 || stimulus[t - 1][i] != *valeur {
                let _ = writeln!(v, "    e{i} = 1'b{valeur};");
            }
        }
    }
    let _ = writeln!(v, "    #1 $finish;");
    let _ = writeln!(v, "  end");
    if netlist.nb_nets > 0 {
        let format: String = "%b".repeat(netlist.nb_nets as usize);
        let nets: Vec<String> = (0..netlist.nb_nets).map(|n| format!("n{n}")).collect();
        let _ = writeln!(v, "  initial forever begin");
        let _ = writeln!(v, "    $strobe(\"{format}\", {});", nets.join(", "));
        let _ = writeln!(v, "    #1;");
        let _ = writeln!(v, "  end");
    }
    let _ = writeln!(v, "endmodule");
    v
}
