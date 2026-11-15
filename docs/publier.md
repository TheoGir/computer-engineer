# Publier une version

Les versions (archive Windows signée et page de nouveautés) sont publiées dans le dépôt public [`TheoGir/computer-engineer-releases`](https://github.com/TheoGir/computer-engineer-releases), où l'application installée vient les chercher.

## Une seule fois : la clé de signature et le jeton

### 1. La clé de signature

Chaque archive publiée est signée avec une clé privée Ed25519. L'application contient la clé publique correspondante et refuse toute archive dont la signature n'est pas valide. **Si la clé privée est perdue, les versions installées ne pourront plus se mettre à jour toutes seules** : il faudrait réinstaller à la main une version qui porte une nouvelle clé. Il faut donc la garder en lieu sûr, par exemple dans un gestionnaire de mots de passe ou sur une clé USB rangée.

```sh
cargo install zipsign --version 0.2.2 --locked
zipsign gen-key cle-privee.key verification-1.pub
```

- `verification-1.pub` est la clé **publique** (32 octets). Elle n'a rien de secret : elle va dans `crates/app/cles/verification-1.pub` du dépôt, par une pull request.
- `cle-privee.key` est la clé **privée**. Elle va dans le secret `ZIPSIGN_CLE_PRIVEE`, encodée en base64, puis dans la sauvegarde hors ligne :
  - sous Windows (PowerShell) : `[Convert]::ToBase64String([IO.File]::ReadAllBytes("cle-privee.key")) | Set-Clipboard` ;
  - sous Linux ou macOS : `base64 -w0 cle-privee.key`.

Le secret se règle dans le dépôt du code : Settings › Secrets and variables › Actions › New repository secret, avec le nom `ZIPSIGN_CLE_PRIVEE`.

### 2. Le jeton qui publie dans le dépôt public

Le workflow tourne dans le dépôt du code et doit écrire dans le dépôt des versions. Son jeton par défaut ne le permet pas, d'où un jeton dédié, limité à ce seul dépôt :

1. Sur GitHub : Settings › Developer settings › Personal access tokens › Fine-grained tokens › Generate new token.
2. Repository access : *Only select repositories*, puis `TheoGir/computer-engineer-releases`.
3. Permissions : *Contents*, en *Read and write*. Rien d'autre.
4. Copier le jeton dans le secret `CE_JETON_VERSIONS` du dépôt du code.

Le jeton expire à la date choisie : il faudra le renouveler, sinon la publication s'arrête avec un message clair.

## À chaque version

1. Monter la version dans `Cargo.toml` (`[workspace.package] version`), puis `cargo check` pour mettre à jour `Cargo.lock`. Chercher l'ancienne version partout pour n'en oublier aucune.
2. Écrire la section `## X.Y.Z` du `CHANGELOG.md`, pour l'utilisateur final : elle devient la page de la version et le message « nouvelle version » de l'application.
3. Mettre à jour le README (état, fonctionnalités).
4. `outils/verifier.sh` doit dire « Tout est vert ».
5. Fusionner dans `main`, puis lancer la publication, de l'une de ces deux façons :
   - **depuis une session Claude** (qui ne peut pas pousser d'étiquette) : lancer le workflow Publication sur `main` avec l'entrée `publier` = `X.Y.Z` (Actions › Publication › Run workflow, ou l'outil MCP `actions_run_trigger`). Le workflow publie, puis pose lui-même l'étiquette `vX.Y.Z` sur le commit publié ; il refuse si l'étiquette existe déjà ou si la branche n'est pas `main` ;
   - **depuis un poste** : poser l'étiquette sur `main` et la pousser :

   ```sh
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```

Le workflow [Publication](../.github/workflows/publication.yml) ne recompile rien et ne relance aucun test : il reprend l'exécutable que la [CI](../.github/workflows/ci.yml) a construit en version finale et testé sur **ce même commit**. La CI note l'empreinte SHA-256 de cet exécutable à la sortie de la compilation et vérifie, avant de le garder, qu'aucun de ses essais ne l'a changé. Le workflow fait :

- sous Linux, il vérifie d'abord, en quelques secondes, que l'étiquette porte la version du code et que le CHANGELOG a sa section ; lancé à la main avec une version, que la branche est `main` et que l'étiquette n'existe pas encore (`outils/controler-version.sh`). Puis il retrouve la CI de ce commit (celle lancée par le push sur `main`, ou lancée à la main) et attend qu'elle soit terminée. Si elle n'est pas entièrement verte (tests Linux et Windows, lint, documentation, comparaison Icarus, essais de l'exécutable, mise à jour de bout en bout), rien n'est publié ;
- sous Windows, pendant ce temps, il compile `zipsign` à neuf depuis crates.io, sans aucun cache, et note son empreinte SHA-256 ;
- sous Windows, dans un job **sans aucun secret**, il télécharge l'exécutable gardé par cette CI et vérifie qu'il a été construit à partir de ce commit, dans le profil `release`, pour le dépôt des versions, et qu'il annonce la version du code (`--version`). Il l'essaie encore dans un dossier dont le nom a un accent et une espace, et note son empreinte SHA-256 ;
- sous Windows, dans un dernier job, le seul qui reçoive la clé de signature, il refait les contrôles de l'étiquette, puis retélécharge ce même exécutable et ce `zipsign`, et refuse si l'une des deux empreintes n'est pas celle notée. Il ne lance jamais l'exécutable, et `zipsign` n'y vient d'aucun cache (un cache peut être écrit par tout job qui lance l'exécutable) : rien de ce qui a côtoyé l'exécutable ne reçoit la clé ;
- il crée l'archive `computer-engineer-vX.Y.Z-x86_64-pc-windows-msvc.zip`, la signe, puis vérifie la signature avec les clés publiques compilées dans l'application (`outils/empaqueter.sh`) ;
- il publie la version dans le dépôt public, avec les notes du CHANGELOG ;
- il retélécharge l'archive comme un inconnu, sans s'identifier, revérifie sa signature et contrôle que GitHub annonce bien cette version comme la dernière.

On peut donc lancer la publication juste après la fusion : elle attend la CI et finit environ une minute après elle (7 min 20 en tout, mesuré le 14/11/2026). Si la CI a déjà fini, la publication prend environ deux minutes. Si la CI de ce commit est rouge, rien n'est publié : corriger, ou la relancer (Actions › CI › Re-run failed jobs) si l'échec ne vient pas du code, puis relancer la publication.

Si l'on a poussé une étiquette et qu'il faut la mettre sur un autre commit : annuler la publication en cours, supprimer l'étiquette (`git push origin :refs/tags/vX.Y.Z`), puis la reposer. Une publication dont l'étiquette ne désigne plus le commit qu'elle publie s'arrête d'elle-même avant de signer.

**Si le run s'arrête après « Publier »** (revérification en échec, étiquette non posée) : la version est en ligne, mais l'étiquette `vX.Y.Z` manque dans ce dépôt. Relancer ne sert à rien, « Publier » refuse une version qui existe déjà.

- Regarder la version dans le dépôt public : l'archive et les notes y sont-elles, et est-elle annoncée comme la dernière ?
- Si elle est bonne : poser l'étiquette sur le commit publié depuis un poste (`git tag vX.Y.Z <commit>`, `git push origin vX.Y.Z`). Le run que cette étiquette déclenche s'arrêtera à « Publier », puisque la version existe : c'est attendu.
- Si elle est mauvaise : la supprimer dans le dépôt public (la version et son étiquette), corriger, puis publier de nouveau.

L'exécutable de la CI est gardé 30 jours, et GitHub ne permet plus de relancer une CI après 30 jours. Si l'exécutable a expiré :

- pour la tête de `main` (le cas courant) : lancer la CI à la main sur `main` (Actions › CI › Run workflow), puis la publication comme d'habitude ; elle attend cette CI et reprend son exécutable ;
- pour un commit plus ancien : créer une branche sur ce commit, y lancer la CI à la main, puis, quand elle est verte, poser l'étiquette `vX.Y.Z` sur ce commit depuis un poste. La publication lancée à la main avec une version ne publie que la tête de `main`.

Lancé à la main sans version (Actions › Publication › Run workflow, entrée `publier` vide), il fait tout sauf publier, sur une branche comme sur une étiquette. L'archive est alors gardée dans les artefacts du run : c'est un essai à blanc. Sur une autre branche que `main`, lancer d'abord la CI à la main sur cette branche (Actions › CI › Run workflow) : c'est elle qui construit la version finale que l'essai reprend.

## Vider les caches

La CI garde en cache les dépendances compilées, et l'exécutable publié est lié à celles que les CI précédentes de `main` y ont laissées : c'est ce qui la rend rapide. Si une dépendance douteuse est passée par `main` (même retirée depuis, même jamais publiée), vider les caches **avant** de publier : Actions › Caches, tout supprimer ; ou, dans `.github/workflows/ci.yml`, passer `prefix-key` de `v1-rust` à `v2-rust` (trois fois) et changer la clé du cache `zipsign-0.2.2-…`. La CI suivante recompile tout (une vingtaine de minutes).

## Après

- Installer la version précédente sur un PC Windows, la lancer, et vérifier qu'elle propose la nouvelle, l'installe et redémarre dessus.
- Mettre à jour `etat/versions` du graphe du projet.

## Changer de clé un jour

1. Créer la nouvelle paire de clés, mettre la nouvelle clé publique dans `crates/app/cles/verification-2.pub`, et publier une version signée **avec l'ancienne** clé : les installations l'acceptent et reçoivent la nouvelle clé publique.
2. Ensuite seulement, remplacer le secret par la nouvelle clé privée.
