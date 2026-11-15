//! Sous Windows, l'icône et la fiche de l'exécutable (nom, version) sont gravées dans
//! `computer-engineer.exe`, avec `winresource`. Ailleurs, rien à faire : la fenêtre reçoit son
//! icône au lancement (`src/main.rs`).

fn main() {
    println!("cargo:rerun-if-changed=icone/computer-engineer.ico");
    // Les traductions sont intégrées à la compilation (`rust_i18n::i18n!`, `src/lib.rs`).
    println!("cargo:rerun-if-changed=locales");
    println!("cargo:rerun-if-env-changed=CI");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    // La description est le nom que le gestionnaire des tâches affiche : le nom de l'application,
    // en ASCII pour ne dépendre d'aucune page de code du compilateur de ressources.
    res.set_icon("icone/computer-engineer.ico")
        .set("ProductName", "Computer Engineer")
        .set("FileDescription", "Computer Engineer");
    if let Err(e) = res.compile() {
        // En CI, l'exécutable publié doit avoir son icône : c'est une erreur. Ailleurs (compilation
        // croisée de contrôle sans compilateur de ressources), l'exécutable reste utilisable.
        if std::env::var_os("CI").is_some() {
            panic!("icône Windows non gravée : {e}");
        }
        println!("cargo:warning=icône Windows non gravée : {e}");
    }
}
