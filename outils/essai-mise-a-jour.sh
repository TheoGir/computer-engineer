#!/usr/bin/env bash
# Essai de bout en bout de la mise à jour automatique, avec un vrai exécutable qui se remplace
# lui-même pendant qu'il tourne, dans un dossier dont le nom a un accent et une espace.
#
#   outils/essai-mise-a-jour.sh <exécutable-d'essai> <exécutable-publié> <dossier-de-travail>
#
# - <exécutable-d'essai> : construit avec `--features banc-mise-a-jour` (jamais publié), il lit
#   l'adresse du faux serveur et la clé d'essai dans CE_BANC_API et CE_BANC_CLE ;
# - <exécutable-publié> : l'exécutable normal, mis dans une archive signée par une clé d'essai
#   et servi comme version 99.0.0 par outils/faux-serveur-de-versions.py.
#
# Cas essayés : signature fausse (refusée, rien ne change), puis bonne signature (installée :
# l'exécutable devient celui publié et démarre).
#
# Demande `zipsign` et `python3` ou `python`.
set -euo pipefail
cd "$(dirname "$0")/.."

banc="$1"
publie="$2"
travail="$3"
py=python
command -v "$py" >/dev/null 2>&1 || py=python3
ext=""
case "$banc" in *.exe) ext=".exe" ;; esac

rm -rf "$travail"
mkdir -p "$travail/cles" "$travail/Zoé essai"
zipsign gen-key "$travail/cles/bonne.key" "$travail/cles/bonne.pub"
zipsign gen-key "$travail/cles/autre.key" "$travail/cles/autre.pub"
hex() { "$py" -c 'import sys; sys.stdout.write(open(sys.argv[1],"rb").read().hex())' "$1"; }

cible="$(rustc -vV | sed -n 's/^host: //p')"
nom="computer-engineer-v99.0.0-${cible}.zip"

# Archive servie : l'exécutable publié, signé par la clé donnée.
archive() {
  mkdir -p "$travail/serveur"
  "$py" - "$publie" "$travail/serveur/$nom" "computer-engineer$ext" <<'PY'
import sys, zipfile
exe, archive, dedans = sys.argv[1:]
with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as z:
    info = zipfile.ZipInfo.from_file(exe, dedans)
    info.external_attr = 0o100755 << 16
    info.compress_type = zipfile.ZIP_DEFLATED
    with open(exe, "rb") as f:
        z.writestr(info, f.read())
PY
  zipsign sign zip "$travail/serveur/$nom" "$1" -f
}

serveur_pid=""
arreter() { [ -n "$serveur_pid" ] && kill "$serveur_pid" 2>/dev/null || true; }
trap arreter EXIT
demarrer() {
  arreter
  rm -f "$travail/adresse"
  "$py" outils/faux-serveur-de-versions.py "$travail/serveur/$nom" 99.0.0 "$travail/adresse" &
  serveur_pid=$!
  for _ in $(seq 100); do [ -s "$travail/adresse" ] && break; sleep 0.1; done
  export CE_BANC_API="$(cat "$travail/adresse")"
}
export CE_BANC_CLE="$(hex "$travail/cles/bonne.pub")"
installe="$travail/Zoé essai/computer-engineer$ext"
empreinte() { "$py" -c 'import hashlib,sys; sys.stdout.write(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$1"; }

echo "== Signature d'une autre clé : refusée"
cp "$banc" "$installe"
archive "$travail/cles/autre.key"
demarrer
avant="$(empreinte "$installe")"
if "$installe" --mettre-a-jour; then
  echo "ÉCHEC : une archive signée par une autre clé a été acceptée" >&2
  exit 1
fi
[ "$(empreinte "$installe")" = "$avant" ] || { echo "ÉCHEC : l'exécutable a changé" >&2; exit 1; }
echo "refusée, exécutable inchangé"

echo "== Bonne signature : installée par l'exécutable en marche"
archive "$travail/cles/bonne.key"
demarrer
"$installe" --mettre-a-jour
[ "$(empreinte "$installe")" = "$(empreinte "$publie")" ] \
  || { echo "ÉCHEC : l'exécutable installé n'est pas celui publié" >&2; exit 1; }
"$installe" --version
echo "installée : l'exécutable est celui publié et il démarre"
