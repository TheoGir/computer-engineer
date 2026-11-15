# Formats de fichier et compatibilité

Ce que Computer Engineer promet à partir de la version 1.0.0 : **un fichier écrit par une
version 1.x s'ouvre dans toute version 1.y plus récente, sans rien perdre**, et une version
plus ancienne ne détruit jamais un fichier qu'elle ne comprend pas.

## Les fichiers

Tous sont dans le dossier de données (`%APPDATA%\Computer Engineer` sous Windows, ou celui de
la variable `CE_DOSSIER`).

| Fichier | Format | Contenu |
|---|---|---|
| `projets/<nom>.json` | `computer-engineer/projet/<n>` | un projet : plan principal et briques |
| `parcours.json` | `computer-engineer/parcours/<n>` | quêtes réussies et briques gagnées |
| `reglages.json` | sans numéro, lu réglage par réglage | thème, texte, animations, vitesse, lancement, mises à jour |
| `apparence.json` (facultatif) | sans numéro, lu champ par champ | couleurs et formes, voir [apparence.md](apparence.md) |
| `rapports/*.txt` | texte | rapports de plantage ou de démarrage impossible |
| `avant-migration/*.json` | celui d'origine | les originaux des fichiers migrés |

## Les règles

1. **Tout changement de ce qui est écrit change de numéro.** Un champ ajouté, retiré, renommé
   ou dont le sens change : `projet/1` devient `projet/2`, et une migration `1 → 2` s'ajoute
   dans `crates/app/src/format.rs`. Les migrations s'enchaînent : un fichier `projet/1` lu par
   une version qui écrit `projet/4` passe par `1 → 2 → 3 → 4`.
2. **Garde-fou automatique.** L'épreuve `tests/formats.rs` relève la forme exacte (tous les
   chemins de clés) de ce que l'application écrit et la compare à `tests/donnees/forme-*.txt`.
   Elle échoue tant qu'un changement n'a pas son nouveau numéro, sa migration et son corpus.
3. **Corpus de vrais fichiers.** `tests/donnees/format-<n>/` garde des fichiers de chaque
   format. À chaque CI, tous doivent s'ouvrir (migrés si besoin), se relire à l'identique et
   s'ouvrir dans l'atelier. On n'efface jamais un fichier du corpus.
4. **L'original est gardé.** Avant qu'un fichier migré ne soit réécrit, son original est
   copié dans `avant-migration/<nom>.format-<n>.json`.
5. **Un fichier plus récent est refusé, jamais écrasé.** Il apparaît « illisible » avec le
   message « vient d'une version plus récente… mets l'application à jour » ; l'application ne
   l'écrit pas, même si c'était le dernier projet ouvert.
6. **Les clés des quêtes ne disparaissent pas.** `tests/donnees/cles-de-quetes.txt` les liste
   toutes ; une quête peut changer de titre ou de consigne, pas de clé (sinon sa réussite
   serait perdue). Une nouvelle quête s'ajoute à la fin de la liste.
7. **Réglages et apparence** se lisent champ par champ : un champ inconnu ou abîmé garde sa
   valeur par défaut sans emporter les autres.
8. **Fichiers abîmés.** Une épreuve abîme au hasard (de façon reproductible) les fichiers du
   corpus et vérifie que leur lecture ne fait jamais planter l'application.

## Changer un format, pas à pas

1. Augmenter `actuel` du format dans `crates/app/src/format.rs` et ajouter la migration à la
   fin de `migrations` (avec ses épreuves).
2. Mettre à jour la constante `FORMAT` correspondante (`brique.rs` ou `parcours.rs`).
3. Effacer `tests/donnees/forme-<genre>.txt`, puis lancer
   `CE_ECRIRE_DONNEES=1 cargo test -p ce-app --test formats` : le relevé et le corpus du
   nouveau format s'écrivent ; les anciens restent et doivent toujours s'ouvrir.
4. Noter le changement dans le CHANGELOG.
