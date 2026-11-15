# Mises à jour automatiques

Exigence du projet : l'application doit pouvoir recevoir des mises à jour automatiques.

**État (13/11/2026) : implémenté et vérifié, pas encore publié.** Code : `crates/app/src/mise_a_jour.rs`. Essais : `crates/app/tests/mise_a_jour.rs` (faux serveur de versions, tous les cas ci-dessous) et `outils/essai-mise-a-jour.sh` (un vrai exécutable qui se remplace lui-même pendant qu'il tourne, dans la CI Windows). Publication : [publier.md](publier.md). Reste avant la première version : la clé de signature et le jeton de publication (voir publier.md).

## Ce que voit l'utilisateur

1. Au lancement, l'application regarde en arrière-plan s'il existe une version plus récente. Elle ne bloque jamais le démarrage et fonctionne sans réseau.
2. Si une version existe, un bandeau discret l'annonce avec les nouveautés. Le téléchargement se fait en arrière-plan.
3. La mise à jour s'installe quand l'utilisateur clique sur « Redémarrer et mettre à jour », ou au prochain lancement. Une simulation en cours n'est jamais interrompue.
4. Les circuits sauvegardés ne sont pas touchés. Le format de sauvegarde porte un numéro (`computer-engineer/bibliotheque/1`) et une version plus récente sait toujours relire les précédentes.
5. Un réglage permet de couper la recherche automatique : `"mises_a_jour_automatiques": false` dans `reglages.json`, à côté du dossier des projets (ou la variable d'environnement `CE_SANS_MISE_A_JOUR`). Le même réglage se trouve dans l'écran Paramètres de l'accueil, avec un bouton « Chercher maintenant » qui cherche une nouvelle version à tout moment (le clic sur le nom de l'application, en haut à gauche, ramène désormais à l'accueil).

## Principe : rien de fait maison

Le mécanisme de mise à jour n'est pas écrit à la main. Il repose sur des bibliothèques établies, et le code propre au projet se limite à les configurer et à afficher le bandeau.

| Rôle | Bibliothèque | Contrôlé sur crates.io le 13/11/2026 |
|---|---|---|
| Chercher la dernière version sur GitHub, télécharger, vérifier, remplacer l'exécutable | [`self_update`](https://crates.io/crates/self_update) 1.3.3 | 13,3 millions de téléchargements, dernière version du 27/10/2026 |
| Signature Ed25519 de l'archive, côté CI et côté application | [`zipsign`](https://crates.io/crates/zipsign-api) 0.2.2 (fonction `signatures` de `self_update`) | 10 millions de téléchargements |
| Remplacer un exécutable en cours d'exécution sous Windows | [`self-replace`](https://crates.io/crates/self-replace) 1.5.1 (utilisée par `self_update`) | 15,4 millions de téléchargements |

Présence vérifiée dans le code source de `self_update` 1.3.3 : source GitHub Releases, clés de vérification (`verifying_keys`), empreintes (`checksums`).

**Autre solution éprouvée, écartée pour l'instant : Velopack** (`velopack` 1.2.161, successeur de Squirrel). Elle apporte un vrai installateur Windows (raccourci, désinstallation) et des mises à jour différentielles, mais demande un outil d'empaquetage en plus dans la CI et ne vérifie pas de signature Ed25519 par défaut. À reprendre si un installateur devient souhaitable. Changer de solution reste possible tant qu'aucune version n'est publiée.

## Forme livrée

Un exécutable Windows unique, sans installateur, publié dans une archive `.zip` signée : `computer-engineer-vX.Y.Z-x86_64-pc-windows-msvc.zip`, dans le dépôt public [`TheoGir/computer-engineer-releases`](https://github.com/TheoGir/computer-engineer-releases).

## Circuit d'une version

```
étiquette git vX.Y.Z
  └─ workflow « publication » (GitHub Actions, Windows)
       ├─ construit l'exécutable optimisé et le met dans une archive .zip
       ├─ signe l'archive avec zipsign (clé privée Ed25519 tenue en secret GitHub Actions)
       └─ crée la version dans le dépôt public des versions

application installée (self_update)
  ├─ demande à GitHub les versions publiées et garde le plus grand numéro
  ├─ ne fait rien s'il n'est pas plus récent que le sien
  ├─ copie l'exécutable en cours à côté de lui (computer-engineer.exe.precedent)
  ├─ télécharge exactement cette version et seulement l'archive qui porte son numéro
  ├─ vérifie sa signature avec la clé publique compilée dans l'exécutable
  ├─ remplace l'exécutable (self-replace), effectif au prochain lancement
  └─ si le remplacement échoue après avoir retiré l'exécutable, remet la copie en place
```

## Règles de sécurité

- HTTPS uniquement.
- Aucun fichier n'est remplacé avant que la signature de l'archive soit vérifiée.
- Sans aucune clé publique compilée, l'application refuse toute mise à jour (avec une liste de clés vide, `self_update` ne vérifierait rien).
- La signature est liée au nom de l'archive, et l'application n'accepte que l'archive dont le nom porte le numéro de la version installée : une vieille archive signée ne peut être resservie ni sous le nom d'une nouvelle, ni sous son propre nom dans une version d'étiquette plus récente.
- Chaque requête a une limite de temps (30 s pour la recherche, 15 min pour le téléchargement) : une connexion coupée sans prévenir ne bloque pas la mise à jour jusqu'à la fermeture.
- La clé privée n'est jamais dans le dépôt ni dans le conteneur de travail. Elle vit dans un secret GitHub Actions et dans une sauvegarde hors ligne. **La perdre empêche toute mise à jour automatique** : il faudrait réinstaller à la main une version portant une nouvelle clé.
- L'application embarque deux clés publiques, pour pouvoir changer de clé sans casser les versions installées.

Pièges à tester : dossier avec accent ou espace (`C:\Users\Zoé\...`), dossier non inscriptible (Program Files), antivirus qui verrouille le fichier, téléchargement coupé.

## Vérification

| Cas | Résultat attendu | Où c'est vérifié |
|---|---|---|
| Version plus récente, correctement signée | installée | `tests/mise_a_jour.rs` (Linux et Windows), `outils/essai-mise-a-jour.sh` (CI Windows, exécutable en marche) |
| Signature d'une autre clé, ou absente | refusée, rien n'est modifié | les deux |
| Archive modifiée après signature | refusée, rien n'est modifié | `tests/mise_a_jour.rs` |
| Signature faite pour une autre archive | refusée | `tests/mise_a_jour.rs` |
| Vieille archive signée, sous son nom, dans une version d'étiquette plus récente | refusée, rien n'est modifié | `tests/mise_a_jour.rs` |
| Plusieurs versions plus récentes, publiées dans n'importe quel ordre | la plus grande est annoncée et c'est elle qui s'installe | `tests/mise_a_jour.rs` |
| Remplacement qui échoue après avoir retiré l'exécutable (cas Windows : antivirus, disque plein) | l'ancienne version est remise en place | `tests/mise_a_jour.rs` (simulé : la panne réelle d'un antivirus n'est pas reproduite) |
| Version égale ou inférieure | ignorée | `tests/mise_a_jour.rs` |
| Pas de réseau | erreur propre, l'application continue ; rien n'est dit si la recherche était automatique | `tests/mise_a_jour.rs` |
| Changement de clé (deux emplacements) | acceptée par l'une ou l'autre | `tests/mise_a_jour.rs` |
| Dans l'application : accueil, Paramètres, « Chercher maintenant », « Mettre à jour », « Redémarrer » | message, installation, proposition de redémarrer | `tests/mise_a_jour.rs` (interface sans fenêtre) |
| Dossier avec accent et espace | installée | les deux |

Après chaque publication réelle : le workflow retélécharge l'archive sans s'identifier, revérifie sa signature et contrôle que GitHub annonce la version comme la dernière. Reste à faire à la main : une version précédente installée sur un vrai PC se met à jour (compétence publier-version, étape 6).

## Limites connues

- **`--mettre-a-jour` en ligne de commande** sert aux essais (CI). L'exécutable publié est une application fenêtrée Windows : lancé depuis cmd ou PowerShell, il n'affiche rien et la console ne l'attend pas. Seul le code de sortie le dit, avec `start /wait computer-engineer.exe --mettre-a-jour` puis `echo %errorlevel%`.
- **Avertissement SmartScreen.** Sans certificat de signature de code Windows (payant), le premier téléchargement manuel de l'exécutable affiche un avertissement de Windows. Les mises à jour suivantes, téléchargées par l'application elle-même, ne le déclenchent pas. À confirmer par un essai réel.
- Windows x86_64 seulement pour commencer. `self_update` choisit l'archive d'après la plateforme, d'autres pourront s'ajouter.

## D'où l'application télécharge (tranché le 13/11/2026)

Le projet a choisi un **dépôt public séparé** qui ne contient que les versions : `TheoGir/computer-engineer-releases`. La CI y publie avec un jeton limité à ce dépôt (secret `CE_JETON_VERSIONS`). Un jeton embarqué dans l'application reste refusé : quiconque l'extrairait pourrait écrire dans le dépôt des versions.

## Contrat figé avec les versions installées

Le code d'une version déjà installée ne change plus : c'est lui qui cherche, télécharge et
vérifie la suivante. Toute version publiée doit donc respecter ce qu'attendent **toutes** les
versions installées depuis la 0.1.0. L'épreuve `le_contrat_des_versions_installees_ne_change_pas`
(`crates/app/tests/mise_a_jour.rs`) le vérifie à chaque CI.

| Élément | Valeur figée |
|---|---|
| Dépôt des versions | `TheoGir/computer-engineer-releases`, public, versions ni brouillon ni préversion |
| Étiquette | `vX.Y.Z` (semver ; la plus grande publiée est installée) |
| Archive | `computer-engineer-vX.Y.Z-x86_64-pc-windows-msvc.zip`, seule archive à ce nom |
| Contenu | `computer-engineer.exe` à la racine de l'archive |
| Signature | zipsign Ed25519, le nom de l'archive pour contexte, par la clé privée de `verification-1.pub` (`7eb47c01…d213`) |

Changer de clé : publier d'abord une version qui contient la nouvelle clé dans
l'emplacement 2 (signée par la clé 1), attendre que les installations l'aient reçue, puis
seulement signer avec la nouvelle (`crates/app/cles/LISEZMOI.md`). Changer de dépôt, de nom
d'archive ou de forme d'étiquette rendrait muettes les versions installées : c'est exclu.
