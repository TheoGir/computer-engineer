//! Banc des mises à jour automatiques : un faux serveur de versions (même API que GitHub) sur la
//! machine, des archives signées avec une clé d'essai, et les cas de `docs/mises-a-jour.md`.
//!
//! Tourne sous Linux et sous Windows (CI), dans un dossier dont le nom a un accent et une espace.

use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use ce_app::mise_a_jour::{Etat, MiseAJour, Nouvelle, Source, VerifyingKey};
use zipsign_api::SigningKey;

const BON: [u8; 32] = [7; 32];
const AUTRE: [u8; 32] = [9; 32];

/// Un serveur HTTP minimal qui répond aux chemins qu'on lui donne, une connexion à la fois.
struct Serveur {
    adresse: String,
    routes: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    /// Les pages des versions publiées, la dernière publiée en premier (ordre de GitHub).
    pages: Mutex<Vec<String>>,
}

impl Serveur {
    fn demarrer() -> Serveur {
        let ecoute = TcpListener::bind("127.0.0.1:0").expect("port libre");
        let adresse = format!("http://{}", ecoute.local_addr().unwrap());
        let routes: Arc<Mutex<HashMap<String, Vec<u8>>>> = Arc::default();
        let r = routes.clone();
        std::thread::spawn(move || {
            for flux in ecoute.incoming() {
                let Ok(mut flux) = flux else { continue };
                let mut requete = Vec::new();
                let mut octet = [0u8; 1];
                while !requete.ends_with(b"\r\n\r\n") {
                    match flux.read(&mut octet) {
                        Ok(1) => requete.push(octet[0]),
                        _ => break,
                    }
                }
                let texte = String::from_utf8_lossy(&requete);
                let cible = texte.split_whitespace().nth(1).unwrap_or("/");
                let chemin = cible.split('?').next().unwrap_or(cible).to_owned();
                let corps = r.lock().unwrap().get(&chemin).cloned();
                let reponse = match corps {
                    Some(corps) => {
                        let mut tete = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\n",
                            corps.len()
                        )
                        .into_bytes();
                        tete.extend(corps);
                        tete
                    }
                    None => {
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                            .to_vec()
                    }
                };
                let _ = flux.write_all(&reponse);
            }
        });
        Serveur {
            adresse,
            routes,
            pages: Mutex::default(),
        }
    }

    /// Publie une version : la page de la version et son archive. Elle devient la dernière
    /// publiée ; les précédentes restent dans la liste.
    fn publier(&self, version: &str, nom_archive: &str, archive: Vec<u8>) {
        let url = format!("{}/fichiers/{nom_archive}", self.adresse);
        let page = format!(
            r#"{{"tag_name":"v{version}","name":"v{version}","created_at":"2026-11-13T00:00:00Z","body":"Nouveautés de la {version}","assets":[{{"name":"{nom_archive}","url":"{url}","browser_download_url":"{url}"}}]}}"#
        );
        let mut pages = self.pages.lock().unwrap();
        pages.insert(0, page.clone());
        let mut routes = self.routes.lock().unwrap();
        routes.insert(
            "/repos/essai/versions/releases/latest".into(),
            page.clone().into_bytes(),
        );
        routes.insert(
            format!("/repos/essai/versions/releases/tags/v{version}"),
            page.into_bytes(),
        );
        routes.insert(
            "/repos/essai/versions/releases".into(),
            format!("[{}]", pages.join(",")).into_bytes(),
        );
        routes.insert(format!("/fichiers/{nom_archive}"), archive);
    }
}

fn nom_archive(version: &str) -> String {
    format!(
        "computer-engineer-v{version}-{}.zip",
        self_update::get_target()
    )
}

fn executable() -> String {
    format!("computer-engineer{}", std::env::consts::EXE_SUFFIX)
}

/// Une archive `.zip` contenant le nouvel exécutable, signée par ces clés.
fn archive(contenu: &[u8], nom: &str, cles: &[[u8; 32]]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file(
        executable(),
        zip::write::SimpleFileOptions::default().unix_permissions(0o755),
    )
    .unwrap();
    zip.write_all(contenu).unwrap();
    let brute = zip.finish().unwrap().into_inner();
    if cles.is_empty() {
        return brute;
    }
    let cles: Vec<SigningKey> = cles.iter().map(SigningKey::from_bytes).collect();
    let mut signee = Cursor::new(Vec::new());
    zipsign_api::sign::copy_and_sign_zip(
        &mut Cursor::new(brute),
        &mut signee,
        &cles,
        Some(nom.as_bytes()),
    )
    .unwrap();
    signee.into_inner()
}

fn publique(cle: [u8; 32]) -> VerifyingKey {
    SigningKey::from_bytes(&cle).verifying_key().to_bytes()
}

/// Un dossier d'installation neuf, au nom piégeux, avec l'ancien exécutable dedans.
fn installation(nom: &str) -> PathBuf {
    let dossier = std::env::temp_dir()
        .join(format!("ce-maj-{}", std::process::id()))
        .join(format!("Zoé essai {nom}"));
    std::fs::create_dir_all(&dossier).unwrap();
    let exe = dossier.join(executable());
    std::fs::write(&exe, b"ANCIENNE VERSION").unwrap();
    exe
}

fn source(serveur: &Serveur, cible: &Path, version: &str) -> Source {
    Source {
        proprietaire: "essai".into(),
        depot: "versions".into(),
        api: Some(serveur.adresse.clone()),
        cles: vec![publique(BON)],
        version: version.into(),
        cible: Some(cible.to_owned()),
    }
}

#[test]
fn une_version_plus_recente_et_bien_signee_s_installe() {
    let serveur = Serveur::demarrer();
    let nom = nom_archive("0.2.0");
    serveur.publier("0.2.0", &nom, archive(b"NOUVELLE VERSION", &nom, &[BON]));
    let exe = installation("bonne");
    let s = source(&serveur, &exe, "0.1.0");

    let trouvee = s.rechercher().expect("recherche");
    assert_eq!(trouvee.as_ref().map(|n| n.version.as_str()), Some("0.2.0"));
    assert_eq!(trouvee.unwrap().notes, "Nouveautés de la 0.2.0");

    assert_eq!(s.installer(), Ok(Some("0.2.0".to_owned())));
    assert_eq!(std::fs::read(&exe).unwrap(), b"NOUVELLE VERSION");
}

#[test]
fn une_cle_tournee_reste_acceptee_par_l_un_ou_l_autre_emplacement() {
    let serveur = Serveur::demarrer();
    let nom = nom_archive("0.2.0");
    serveur.publier("0.2.0", &nom, archive(b"NOUVELLE VERSION", &nom, &[AUTRE]));
    let exe = installation("rotation");
    let mut s = source(&serveur, &exe, "0.1.0");
    s.cles = vec![publique(BON), publique(AUTRE)];
    assert_eq!(s.installer(), Ok(Some("0.2.0".to_owned())));
}

#[test]
fn une_signature_fausse_ou_absente_est_refusee_sans_rien_toucher() {
    for (cas, cles) in [("autre-cle", vec![AUTRE]), ("sans-signature", vec![])] {
        let serveur = Serveur::demarrer();
        let nom = nom_archive("0.2.0");
        serveur.publier("0.2.0", &nom, archive(b"PIEGE", &nom, &cles));
        let exe = installation(cas);
        let erreur = source(&serveur, &exe, "0.1.0").installer();
        assert!(
            erreur.as_ref().is_err_and(|e| e.contains("Signature")),
            "{cas} : {erreur:?}"
        );
        assert_eq!(
            std::fs::read(&exe).unwrap(),
            b"ANCIENNE VERSION",
            "{cas} : l'exécutable ne doit pas changer"
        );
    }
}

#[test]
fn une_archive_modifiee_apres_signature_est_refusee() {
    let serveur = Serveur::demarrer();
    let nom = nom_archive("0.2.0");
    let mut signee = archive(b"NOUVELLE VERSION", &nom, &[BON]);
    // Un octet du contenu change après la signature.
    let i = signee
        .windows(4)
        .position(|w| w == b"PK\x03\x04")
        .expect("en-tête zip")
        + 40;
    signee[i] ^= 0x55;
    serveur.publier("0.2.0", &nom, signee);
    let exe = installation("modifiee");
    assert!(source(&serveur, &exe, "0.1.0").installer().is_err());
    assert_eq!(std::fs::read(&exe).unwrap(), b"ANCIENNE VERSION");
}

#[test]
fn une_signature_faite_pour_une_autre_archive_est_refusee() {
    // La signature est liée au nom de l'archive : on ne peut pas resservir celle d'une vieille
    // version sous le nom d'une nouvelle.
    let serveur = Serveur::demarrer();
    let nom = nom_archive("0.2.0");
    serveur.publier(
        "0.2.0",
        &nom,
        archive(b"VIEILLE", &nom_archive("0.0.1"), &[BON]),
    );
    let exe = installation("autre-nom");
    assert!(source(&serveur, &exe, "0.1.0").installer().is_err());
    assert_eq!(std::fs::read(&exe).unwrap(), b"ANCIENNE VERSION");
}

#[test]
fn une_version_egale_ou_plus_ancienne_est_ignoree() {
    for publiee in ["0.1.0", "0.0.9"] {
        let serveur = Serveur::demarrer();
        let nom = nom_archive(publiee);
        serveur.publier(publiee, &nom, archive(b"NON", &nom, &[BON]));
        let exe = installation(&format!("ignoree-{publiee}"));
        let s = source(&serveur, &exe, "0.1.0");
        assert_eq!(s.rechercher(), Ok(None));
        assert_eq!(s.installer(), Ok(None));
        assert_eq!(std::fs::read(&exe).unwrap(), b"ANCIENNE VERSION");
    }
}

#[test]
fn sans_reseau_la_recherche_echoue_proprement() {
    // Un port fermé : rien n'écoute.
    let ferme = TcpListener::bind("127.0.0.1:0").unwrap();
    let adresse = format!("http://{}", ferme.local_addr().unwrap());
    drop(ferme);
    let exe = installation("sans-reseau");
    let mut s = Source {
        proprietaire: "essai".into(),
        depot: "versions".into(),
        api: Some(adresse),
        cles: vec![publique(BON)],
        version: "0.1.0".into(),
        cible: Some(exe.clone()),
    };
    assert!(s.rechercher().is_err());
    assert!(s.installer().is_err());
    assert_eq!(std::fs::read(&exe).unwrap(), b"ANCIENNE VERSION");
    // Sans clé, rien ne part même vers le réseau : sans clé, rien ne serait vérifié.
    s.cles.clear();
    assert!(s
        .installer()
        .unwrap_err()
        .contains("pas encore configurées"));
}

#[test]
fn le_fil_de_l_application_annonce_puis_installe() {
    let serveur = Serveur::demarrer();
    let nom = nom_archive("0.3.0");
    serveur.publier("0.3.0", &nom, archive(b"TROISIEME", &nom, &[BON]));
    let exe = installation("fil");
    let mut maj = MiseAJour::nouvelle(source(&serveur, &exe, "0.1.0"));

    maj.rechercher(false);
    assert_eq!(maj.etat, Etat::Recherche { demandee: false });
    match maj.attendre() {
        Some(Nouvelle::Recherche {
            demandee: false,
            resultat: Ok(Some(n)),
        }) => assert_eq!(n.version, "0.3.0"),
        autre => panic!("{autre:?}"),
    }
    assert!(matches!(maj.etat, Etat::Disponible(_)));

    maj.installer();
    assert_eq!(maj.etat, Etat::Installation);
    assert_eq!(
        maj.attendre(),
        Some(Nouvelle::Installation(Ok(Some("0.3.0".into()))))
    );
    assert_eq!(maj.etat, Etat::Installee("0.3.0".into()));
    assert_eq!(std::fs::read(&exe).unwrap(), b"TROISIEME");
    // Une fois installée, on ne cherche plus rien jusqu'au redémarrage.
    maj.rechercher(true);
    assert!(!maj.occupee());
}

#[test]
fn les_cles_officielles_sont_des_cles_valides() {
    // Tant que la clé de signature n'a pas été créée, il n'y en a aucune et les mises à jour
    // sont refusées ; ensuite, chaque clé présente doit être une vraie clé Ed25519.
    for cle in ce_app::mise_a_jour::cles_officielles() {
        assert!(zipsign_api::VerifyingKey::from_bytes(&cle).is_ok());
    }
}

/// Dans l'application : un clic sur son nom mène à l'accueil, « Chercher maintenant » dans les
/// paramètres trouve une version, le message propose de la mettre à jour, un clic l'installe et
/// propose de redémarrer.
#[test]
fn dans_l_application_on_cherche_on_installe_et_on_propose_de_redemarrer() {
    use ce_app::app::{Application, Ecran};
    use ce_app::apparence::{Apparence, Surveillance};
    use ce_app::etat::Proposition;
    use egui::Vec2;

    let serveur = Serveur::demarrer();
    let nom = nom_archive("0.4.0");
    serveur.publier("0.4.0", &nom, archive(b"QUATRIEME", &nom, &[BON]));
    let exe = installation("application");
    let mut app = Application::nouvelle(Apparence::default(), Surveillance::default());
    app.mise_a_jour = Some(MiseAJour::nouvelle(source(&serveur, &exe, "0.1.0")));
    let mut b = egui_kittest::Harness::builder()
        .with_size(Vec2::new(1440.0, 900.0))
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut Application| app.image(ui), app);
    b.run_steps(120);

    // Une commande de l'écran affiché : l'accueil ou les panneaux de l'atelier.
    let repere = |b: &egui_kittest::Harness<'_, Application>, nom: &str, rang: usize| {
        let app = b.state();
        match app.ecran {
            Ecran::Accueil => app.accueil.repere(nom, rang),
            Ecran::Atelier => app.panneaux.repere(nom, rang),
        }
    };
    let cliquer = |b: &mut egui_kittest::Harness<'_, Application>, nom: &str, rang: usize| {
        let p = repere(b, nom, rang)
            .unwrap_or_else(|| panic!("{nom} {rang} n'est pas affiché"))
            .center();
        b.hover_at(p);
        b.step();
        b.drag_at(p);
        b.step();
        b.drop_at(p);
        b.step();
    };
    let attendre = |b: &mut egui_kittest::Harness<'_, Application>, action: Proposition| {
        for _ in 0..500 {
            if b.state()
                .atelier
                .messages
                .iter()
                .any(|m| m.action == Some(action))
            {
                b.run_steps(20);
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
            b.step();
        }
        panic!("pas de message proposant {action:?}");
    };

    cliquer(&mut b, "marque", 0);
    b.run_steps(10);
    assert_eq!(b.state().ecran, Ecran::Accueil);
    cliquer(&mut b, "menu", 3);
    b.run_steps(10);
    cliquer(&mut b, "chercher", 0);
    attendre(&mut b, Proposition::MettreAJour);
    let texte = b
        .state()
        .atelier
        .messages
        .iter()
        .find(|m| m.action == Some(Proposition::MettreAJour))
        .unwrap()
        .texte
        .clone();
    assert!(
        texte.contains("0.4.0") && texte.contains("Nouveautés"),
        "{texte}"
    );

    cliquer(&mut b, "mettre-a-jour", 0);
    attendre(&mut b, Proposition::Redemarrer);
    assert!(repere(&b, "redemarrer", 0).is_some());
    assert_eq!(std::fs::read(&exe).unwrap(), b"QUATRIEME");
    assert_eq!(
        b.state().mise_a_jour.as_ref().unwrap().etat,
        Etat::Installee("0.4.0".into())
    );
    // Le message proposant la mise à jour a disparu une fois choisie.
    assert!(!b
        .state()
        .atelier
        .messages
        .iter()
        .any(|m| m.action == Some(Proposition::MettreAJour)));

    // Le message est fermé (« Plus tard »), puis un clic sur le nom : on repropose de
    // redémarrer, sans chercher.
    b.state_mut()
        .atelier
        .messages
        .retain(|m| m.action != Some(Proposition::Redemarrer));
    b.run_steps(30);
    assert!(repere(&b, "redemarrer", 0).is_none());
    cliquer(&mut b, "chercher", 0);
    attendre(&mut b, Proposition::Redemarrer);
    assert!(!b.state().mise_a_jour.as_ref().unwrap().occupee());
}

#[test]
fn la_version_annoncee_est_la_plus_recente_et_c_est_celle_installee() {
    // 0.1.1 (correctif) et 0.2.0 sont publiées, dans les deux ordres : depuis la 0.1.0, on
    // annonce et on installe la 0.2.0, la plus grande, pas la dernière publiée ni la « compatible ».
    for ordre in [["0.1.1", "0.2.0"], ["0.2.0", "0.1.1"]] {
        let serveur = Serveur::demarrer();
        for v in ordre {
            let nom = nom_archive(v);
            let contenu = format!("VERSION {v}");
            serveur.publier(v, &nom, archive(contenu.as_bytes(), &nom, &[BON]));
        }
        let exe = installation(&format!("ordre-{}", ordre[0]));
        let s = source(&serveur, &exe, "0.1.0");
        assert_eq!(
            s.rechercher().unwrap().map(|n| n.version),
            Some("0.2.0".to_owned()),
            "{ordre:?}"
        );
        assert_eq!(s.installer(), Ok(Some("0.2.0".to_owned())), "{ordre:?}");
        assert_eq!(std::fs::read(&exe).unwrap(), b"VERSION 0.2.0", "{ordre:?}");
    }
}

#[test]
fn une_vieille_archive_signee_sous_une_etiquette_recente_est_refusee() {
    // Quelqu'un qui peut publier dans le dépôt des versions, mais sans la clé privée, publie
    // l'étiquette v0.2.0 avec une vraie archive signée d'une ancienne version, sous son nom
    // d'origine : elle ne doit pas s'installer.
    let serveur = Serveur::demarrer();
    let vieille = nom_archive("0.0.1");
    serveur.publier("0.2.0", &vieille, archive(b"VIEILLE", &vieille, &[BON]));
    let exe = installation("vieille-archive");
    let erreur = source(&serveur, &exe, "0.1.0").installer();
    assert!(erreur.is_err(), "{erreur:?}");
    assert_eq!(std::fs::read(&exe).unwrap(), b"ANCIENNE VERSION");
    assert!(!ce_app::mise_a_jour::copie_de_secours(&exe).exists());
}

#[test]
fn une_installation_qui_echoue_apres_avoir_retire_l_executable_le_remet_en_place() {
    use ce_app::mise_a_jour::{avec_secours, copie_de_secours};
    let exe = installation("secours");
    let copie = copie_de_secours(&exe);
    // Comme self-replace sous Windows quand l'antivirus bloque la dernière étape : l'exécutable a
    // été déplacé, une copie temporaire traîne, et l'erreur remonte.
    let reste = exe.with_file_name(".computer-engineer.abcdef.__temp__.exe");
    let r: Result<(), String> = avec_secours(&exe, || {
        assert_eq!(std::fs::read(&copie).unwrap(), b"ANCIENNE VERSION");
        std::fs::rename(&exe, exe.with_file_name("ailleurs")).unwrap();
        std::fs::write(&reste, b"MOITIE").unwrap();
        Err("accès refusé (os error 32)".into())
    });
    let e = r.unwrap_err();
    assert!(e.contains("remise en place"), "{e}");
    assert_eq!(std::fs::read(&exe).unwrap(), b"ANCIENNE VERSION");
    assert!(!copie.exists());
    assert!(!reste.exists());

    // Échec sans rien toucher : le message le dit, la copie disparaît.
    let r: Result<(), String> = avec_secours(&exe, || Err("réseau coupé".into()));
    let e = r.unwrap_err();
    assert!(e.contains("n'a pas été modifiée"), "{e}");
    assert_eq!(std::fs::read(&exe).unwrap(), b"ANCIENNE VERSION");
    assert!(!copie.exists());

    // Réussite : la copie disparaît aussi.
    assert_eq!(avec_secours(&exe, || Ok(3)), Ok(3));
    assert!(!copie.exists());

    // Sans exécutable à copier, rien n'est tenté.
    let absent = exe.with_file_name("absent.exe");
    let mut appele = false;
    let r: Result<(), String> = avec_secours(&absent, || {
        appele = true;
        Ok(())
    });
    assert!(r.is_err() && !appele);
}

#[test]
fn une_recherche_demandee_pendant_la_recherche_automatique_recoit_sa_reponse() {
    let serveur = Serveur::demarrer();
    let exe = installation("demandee");
    // Rien de publié : la réponse est « rien de neuf » (ou une erreur), qu'il faut dire puisque
    // l'utilisateur a demandé.
    let mut maj = MiseAJour::nouvelle(source(&serveur, &exe, "0.1.0"));
    maj.rechercher(false);
    maj.rechercher(true);
    assert_eq!(maj.etat, Etat::Recherche { demandee: true });
    match maj.attendre() {
        Some(Nouvelle::Recherche { demandee, .. }) => assert!(demandee),
        autre => panic!("{autre:?}"),
    }
}

/// Le contrat que les versions déjà installées attendent de toute version publiée après elles
/// (`docs/mises-a-jour.md`, « Contrat figé ») : le code d'une version installée ne change plus,
/// donc ces valeurs ne changent jamais, sinon elle ne trouverait plus ses mises à jour.
#[test]
fn le_contrat_des_versions_installees_ne_change_pas() {
    use ce_app::mise_a_jour::{nom_d_archive, DEPOT_DES_VERSIONS, EXECUTABLE};
    // Le dépôt public où les versions 0.1.0 et suivantes cherchent.
    if option_env!("CE_DEPOT_DES_VERSIONS").is_none() {
        assert_eq!(DEPOT_DES_VERSIONS, "TheoGir/computer-engineer-releases");
    }
    // Le nom de l'exécutable dans l'archive, et le nom de l'archive.
    assert_eq!(EXECUTABLE, "computer-engineer");
    assert_eq!(
        nom_d_archive("1.0.0"),
        format!("computer-engineer-v1.0.0-{}.zip", self_update::get_target())
    );
    if cfg!(all(windows, target_arch = "x86_64")) {
        assert_eq!(
            nom_d_archive("1.0.0"),
            "computer-engineer-v1.0.0-x86_64-pc-windows-msvc.zip"
        );
    }
    // La clé publique 1, compilée dans toutes les versions depuis la 0.1.0 : la clé privée qui
    // lui répond doit signer toutes les versions publiées (ou la clé 2 la relayer, voir le
    // LISEZMOI des clés).
    let cle_1 = include_bytes!("../cles/verification-1.pub");
    let hex: String = cle_1.iter().map(|o| format!("{o:02x}")).collect();
    assert_eq!(
        hex,
        "7eb47c01905dea3a4e06bdb87642a8a89785ed935e0e8b825f524eeeed61d213"
    );
}
