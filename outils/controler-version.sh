#!/usr/bin/env bash
# Contrôles d'une version à publier. Le workflow Publication les fait deux fois : au début, en
# quelques secondes, avant d'attendre la CI ; puis juste avant de signer.
#
#   ETIQUETTE=vX.Y.Z ETIQUETTE_A_POSER=true|false outils/controler-version.sh
#
# - l'étiquette porte la version de ce-app (Cargo.toml) ;
# - le CHANGELOG.md a sa section « ## X.Y.Z » ;
# - publiée à la main (ETIQUETTE_A_POSER=true : le workflow posera lui-même l'étiquette),
#   seulement depuis main (GITHUB_REF), et seulement si l'étiquette n'existe pas encore dans ce
#   dépôt. Si l'on ne peut pas le savoir (réseau, droits), on refuse ;
# - publiée par une étiquette poussée (GITHUB_REF = refs/tags/…), seulement si l'étiquette du
#   dépôt désigne toujours le commit publié (GITHUB_SHA).
#
# Demande `cargo`, `git` et `python3` ou `python`.
set -euo pipefail
cd "$(dirname "$0")/.."

: "${ETIQUETTE:?ETIQUETTE manquante (vX.Y.Z)}"
py=python
command -v "$py" >/dev/null 2>&1 || py=python3
version="$(cargo metadata --format-version 1 --no-deps --locked \
  | "$py" -c 'import json,sys; sys.stdout.write(next(p["version"] for p in json.load(sys.stdin.buffer)["packages"] if p["name"]=="ce-app"))')"
# (sys.stdout.write et non print : sous Windows, print écrit « \r\n » et Git Bash garde le « \r ».)
echo "Version du code : $version, étiquette : $ETIQUETTE"
if [ "v$version" != "$ETIQUETTE" ]; then
  echo "L'étiquette $ETIQUETTE ne porte pas la version du code ($version)." >&2
  exit 1
fi
grep -q "^## $version" CHANGELOG.md || { echo "Pas de section « ## $version » dans CHANGELOG.md" >&2; exit 1; }

if [ "${ETIQUETTE_A_POSER:-false}" = true ]; then
  if [ "${GITHUB_REF:-}" != refs/heads/main ]; then
    echo "Une version se publie depuis main (ici : ${GITHUB_REF:-inconnu})." >&2
    exit 1
  fi
  # git ls-remote --exit-code : 0 si l'étiquette existe, 2 si elle n'existe pas, autre chose si
  # la question n'a pas pu être posée.
  code=0
  git ls-remote --exit-code --tags origin "refs/tags/$ETIQUETTE" >/dev/null || code=$?
  case "$code" in
    2) ;;
    0) echo "L'étiquette $ETIQUETTE existe déjà." >&2; exit 1 ;;
    *) echo "Impossible de savoir si l'étiquette $ETIQUETTE existe (git ls-remote : code $code)." >&2; exit 1 ;;
  esac
fi
# Publiée par une étiquette poussée : l'étiquette du dépôt désigne-t-elle toujours ce commit ? Si
# elle a été déplacée pendant que la publication attendait la CI, on ne publie pas l'ancien commit
# sous ce nom. (Étiquette annotée : la ligne « ^{} » donne le commit.)
if [[ "${GITHUB_REF:-}" == refs/tags/* ]]; then
  : "${GITHUB_SHA:?GITHUB_SHA manquant}"
  distante="$(git ls-remote origin "refs/tags/$ETIQUETTE" "refs/tags/$ETIQUETTE^{}")" \
    || { echo "Impossible de lire l'étiquette $ETIQUETTE du dépôt (git ls-remote)." >&2; exit 1; }
  if ! grep -q "^${GITHUB_SHA}[[:space:]]" <<< "$distante"; then
    echo "L'étiquette $ETIQUETTE du dépôt ne désigne plus le commit $GITHUB_SHA : rien n'est publié." >&2
    exit 1
  fi
fi
echo "Étiquette $ETIQUETTE, version et CHANGELOG : bons."
