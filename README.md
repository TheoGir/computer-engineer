# Computer Engineer

Construire un ordinateur complet à partir d'une seule porte logique, le NAND, dans un simulateur qui reproduit réellement le comportement logique et temporel du circuit.

État : **version 0.8.0** ([télécharger](https://github.com/TheoGir/computer-engineer-releases/releases/latest)), qui se met ensuite à jour toute seule. L'application s'ouvre sur un accueil (reprendre le dernier projet, parcours des quêtes, mes projets, paramètres) et s'anime d'un écran à l'autre. Le moteur de simulation est fait ; l'application graphique affiche, anime et laisse modifier un circuit, regroupe des portes en **briques réutilisables et emboîtables**, et enregistre les projets ; l'**ordinateur complet miniature en NAND** (processeur, mémoires, horloge, écran, clavier) s'y ouvre, s'y visite brique par brique et calcule ce qu'on tape sur son clavier, et **se programme dans l'application** (Ctrl+P : assembleur, mémoire morte refabriquée en NAND) ; un **parcours de 96 quêtes vérifiées** mène de la porte NON à l'ordinateur, avec six branches (carte réseau, montre numérique, contrôleur d'écran, maths et automates, codes et secrets, jeux) et des **grands projets** plus grands que l'ordinateur (deux ordinateurs en réseau, un ordinateur à 256 octets).

## Ce qui existe

| Crate | Rôle |
|---|---|
| `crates/moteur` (`ce-moteur`) | Logique à 4 valeurs `0 1 X Z`, simulateur événementiel à délais, sémantique de référence. Aucune dépendance. |
| `crates/circuit` (`ce-circuit`) | Composants hiérarchiques réutilisables, format JSON, mise à plat, tables de vérité, bibliothèque de référence en NAND, les corrigés des quêtes (`classiques`), export Verilog, et l'ordinateur miniature avec son assembleur. |
| `crates/cli` (`ce`) | Outil en ligne de commande. |
| `crates/app` (`computer-engineer`) | L'application : plan interactif (déplacer, câbler, zoomer), courant animé, briques réutilisables, parcours de quêtes, chronogramme, sauvegarde. Rust natif, egui et wgpu. |

Ce que le moteur garantit, et comment c'est démontré : [docs/semantique.md](docs/semantique.md).

## L'application

```sh
cargo run --release -p ce-app
```

On y glisse des portes NAND depuis la palette, on tire des fils d'un point de connexion à un autre, on clique sur les interrupteurs, et le courant se propage porte après porte sous les yeux, calculé par le moteur.

On sélectionne plusieurs éléments (Maj + glisser un cadre sur le fond, ou Maj + clic), puis `Ctrl+G` : ils deviennent une **brique**, rangée dans la palette, réutilisable et emboîtable dans d'autres briques. Double-clic sur une brique pour l'ouvrir ; le fil d'Ariane en haut du plan ramène au niveau voulu. La liste des projets propose l'**ordinateur miniature** : clique sur son clavier, tape deux chiffres, son écran affiche le produit. Ouvrir une brique posée montre cet exemplaire précis en marche, avec ses valeurs en direct.

Le panneau Quête propose un **parcours de 96 quêtes** : 8 chapitres de la porte NON jusqu'à trois ordinateurs complets, puis six branches (carte réseau, montre numérique, contrôleur d'écran, maths et automates avec les nombres remarquables, codes et secrets, jeux) et les grands projets (mémoires de 64 et 256 octets, ordinateur à 256 octets, messagerie entre deux ordinateurs, messagerie secrète). Chaque quête a une fiche « Comprendre » : à quoi ça sert, comment ça marche, sa place dans le tout, dans la vraie vie, le savais-tu ?, à essayer. Le parcours principal passe par les multiplexeurs, additionneurs, multiplicateur, diviseur, bascules, registres, mémoires, pile, file, compteurs, liaison série et processeur. « Vérifier » compare le circuit construit à un corrigé en NAND (`ce_circuit::classiques`), lui-même prouvé contre sa définition ; chaque quête réussie devient une brique utilisable dans les suivantes.

Les projets s'enregistrent tout seuls dans `%APPDATA%\Computer Engineer\projets` sous Windows (`~/.local/share/computer-engineer/projets` sous Linux, `~/Library/Application Support/Computer Engineer/projets` sous macOS, ou le dossier désigné par la variable `CE_DOSSIER`), en JSON lisible. L'application s'ouvre sur un **accueil** : « Continuer » (ou Entrée) reprend le dernier projet là où il en était ; on y trouve aussi le parcours des quêtes par chapitre, tous les projets (ouvrir, renommer, dupliquer, mettre à la corbeille) et les paramètres (thème, taille du texte, animations, vitesse, ce qui s'ouvre au lancement, mises à jour). Dans l'atelier, un clic sur le nom de l'application, en haut à gauche, ramène à l'accueil.

Raccourcis : `Espace` lecture ou pause, `Suppr` supprimer, `Ctrl+Z` et `Ctrl+Y` annuler et rétablir, `Ctrl+A` tout sélectionner, `Ctrl+G` regrouper en brique, `Ctrl+D` dupliquer, `Échap` lâcher la sélection puis remonter d'une brique, `F` recadrer, flèches pour déplacer la sélection, molette pour zoomer. Quand un clavier du circuit est sélectionné, ce qu'on tape va au circuit.

F1 ouvre l'aide ; le guide de l'utilisateur est dans [docs/guide.md](docs/guide.md), les formats de fichier et leur compatibilité dans [docs/formats.md](docs/formats.md).

Toute l'esthétique se règle sans recompiler, dans un fichier `apparence.json` relu pendant que l'application tourne : [docs/apparence.md](docs/apparence.md).

Sous Linux, l'application demande les bibliothèques habituelles d'un bureau (`libxkbcommon-x11`, un pilote Vulkan ou OpenGL). Sous Windows, rien à installer.

## Installer et mettre à jour

Les versions publiées sont dans le dépôt public [computer-engineer-releases](https://github.com/TheoGir/computer-engineer-releases/releases) : un **installateur** (`computer-engineer-setup-vX.Y.Z-x86_64.exe`, Inno Setup : installation pour l'utilisateur seul sans droits d'administrateur, raccourcis du menu Démarrer et du bureau, désinstallation depuis les Paramètres de Windows ; script dans [installateur/computer-engineer.iss](installateur/computer-engineer.iss)) et l'archive `.zip` signée qui contient `computer-engineer.exe` seul, celle que téléchargent les mises à jour automatiques. Lancée depuis l'archive extraite, l'application pose elle-même ses raccourcis au premier lancement ; ils suivent les mises à jour, et les Paramètres les retirent ou les remettent. L'application cherche ensuite toute seule une nouvelle version au lancement, la propose dans un message, vérifie sa signature avant de remplacer quoi que ce soit, puis propose de redémarrer. Le bouton « Chercher maintenant » des paramètres cherche une version à tout moment. Conception : [docs/mises-a-jour.md](docs/mises-a-jour.md) ; publier une version : [docs/publier.md](docs/publier.md).

## Essayer le moteur en ligne de commande

```sh
cargo run --release -p ce-cli -- reference > reference.json   # la bibliothèque de référence
cargo run --release -p ce-cli -- valider reference.json
cargo run --release -p ce-cli -- table reference.json ADD_COMPLET
cargo run --release -p ce-cli -- infos reference.json ADD8
cargo run --release -p ce-cli -- mesure                        # vitesse du moteur
```

## L'ordinateur miniature

Le test d'acceptation du projet : 3 401 portes NAND et une horloge, six niveaux de composants emboîtés, et de vrais programmes exécutés porte par porte. Détails et preuves : [docs/ordinateur.md](docs/ordinateur.md).

```sh
cargo run --release -p ce-cli -- ordinateur demo
cargo run --release -p ce-cli -- ordinateur exemple calculatrice > calc.asm
cargo run --release -p ce-cli -- ordinateur executer calc.asm 98        # Écran : 9*8=72
```

## Un composant

```json
{
  "format": "computer-engineer/bibliotheque/1",
  "composants": {
    "NON": {
      "description": "Inverseur",
      "broches": [
        { "nom": "a", "sens": "entree" },
        { "nom": "s", "sens": "sortie" }
      ],
      "instances": [ { "nom": "n", "type": "NAND" } ],
      "fils": [ ["a", "n.a", "n.b"], ["n.s", "s"] ]
    }
  }
}
```

Un composant s'utilise ensuite dans un autre comme une primitive (`"type": "NON"`), sans limite de profondeur. Le format complet est décrit en tête de `crates/circuit/src/modele.rs`.

## Vérifier

```sh
outils/verifier.sh
```

Formatage, lint sans avertissement, tous les tests, documentation. La comparaison avec Icarus Verilog demande `iverilog` (`apt-get install iverilog`).

L'interface est testée sans fenêtre : de vrais événements de souris et de clavier sont envoyés à l'application (`crates/app/tests/interface.rs`). Pour produire des captures d'écran, avec un pilote graphique même logiciel :

```sh
CE_CAPTURES=/tmp/captures cargo test -p ce-app --test captures
```

## Feuille de route

1. **Moteur 4 valeurs, NAND, simulation événementielle, tests** (fait)
2. Éditeur graphique (**plan interactif fait**), sauvegarde et chargement (**faits**), première version publiée avec mise à jour automatique (**faits** : versions 0.1.0 à 0.8.0 publiées, mise à jour automatique signée, procédure dans [docs/publier.md](docs/publier.md))
3. Composants hiérarchiques dans l'éditeur, bibliothèque, bus (**briques, bus, découpeurs et jonctions faits**)
4. Horloge, chronogrammes, bancs de test, tables de vérité
5. RAM et ROM, éditeur de programme, entrées et sorties
6. Quêtes, jusqu'à un ordinateur complet qui sert de base réutilisable (**l'ordinateur miniature tourne dans l'application, s'y ouvre brique par brique, et 96 quêtes vérifiées y mènent**)
7. Performance
