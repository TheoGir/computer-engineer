//! Les licences des bibliothèques et des polices compilées dans l'application, écrites par
//! `outils/licences.py` (la CI vérifie qu'elles sont à jour).

/// Une ligne par bibliothèque ou police : nom, version, licence.
pub const RESUME: &str = include_str!("../licences/resume.txt");

/// Les textes complets des licences, chacun avec la liste de ce qu'il couvre.
pub const TEXTES: &str = include_str!("../licences/textes.txt");

/// Écrit les textes complets dans `dossier/licences.txt` et renvoie le chemin, pour les lire
/// dans un éditeur de texte.
pub fn ecrire(dossier: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    std::fs::create_dir_all(dossier)?;
    let chemin = dossier.join("licences.txt");
    std::fs::write(
        &chemin,
        format!(
            "Computer Engineer {}\n\n{RESUME}\n{TEXTES}",
            env!("CARGO_PKG_VERSION")
        ),
    )?;
    Ok(chemin)
}

#[cfg(test)]
mod tests {
    #[test]
    fn les_licences_couvrent_les_bibliotheques_principales() {
        for nom in [
            "egui ",
            "eframe ",
            "wgpu ",
            "self_update ",
            "wasmtime ",
            "serde ",
        ] {
            assert!(super::RESUME.contains(&format!("- {nom}")), "{nom}");
        }
        assert!(super::RESUME.contains("police Inter"));
        assert!(super::TEXTES.contains("Permission is hereby granted"));
        assert!(super::TEXTES.contains("Apache License"));
    }
}
