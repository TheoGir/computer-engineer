#!/usr/bin/env bash
# Met l'exécutable de l'application dans l'archive publiée, la signe et vérifie la signature.
#
#   outils/empaqueter.sh <exécutable> <clé-privée> <dossier-de-sortie>
#
# - l'archive s'appelle computer-engineer-vX.Y.Z-<cible>.zip, avec la version de ce-app et la
#   cible de compilation de Rust : c'est le nom que cherche la mise à jour automatique ;
# - elle ne contient que computer-engineer(.exe), à la racine ;
# - elle est signée avec zipsign (Ed25519), le nom de l'archive servant de contexte, comme le
#   vérifie l'application ;
# - la signature est vérifiée avec les clés publiques de crates/app/cles/ : si la clé privée
#   donnée ne correspond à aucune, l'archive est refusée (l'application la refuserait aussi).
#
# Demande `zipsign` (cargo install zipsign --version 0.2.2 --locked) et `python3` ou `python`.
# ZIPSIGN donne le chemin absolu d'un zipsign précis (la publication y met celui qu'elle a vérifié) ;
# sinon, celui du PATH.
set -euo pipefail
cd "$(dirname "$0")/.."

zipsign="${ZIPSIGN:-zipsign}"

exe="$1"
cle="$2"
sortie="$3"

cible="$(rustc -vV | sed -n 's/^host: //p')"
case "$exe" in
  *.exe) dedans="computer-engineer.exe" ;;
  *) dedans="computer-engineer" ;;
esac
py=python
command -v "$py" >/dev/null 2>&1 || py=python3
version="$(cargo metadata --format-version 1 --no-deps --locked \
  | "$py" -c 'import json,sys; sys.stdout.write(next(p["version"] for p in json.load(sys.stdin.buffer)["packages"] if p["name"]=="ce-app"))')"
# (sys.stdout.write et non print : sous Windows, print écrit « \r\n » et Git Bash garde le « \r ».)
case "$version" in
  '' | *[!0-9A-Za-z.+-]*) echo "Version illisible : « $version »" >&2; exit 1 ;;
esac
nom="computer-engineer-v${version}-${cible}.zip"

mkdir -p "$sortie"
archive="$sortie/$nom"
rm -f "$archive"
"$py" - "$exe" "$archive" "$dedans" <<'PY'
import sys, zipfile
exe, archive, dedans = sys.argv[1:]
with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as z:
    info = zipfile.ZipInfo.from_file(exe, dedans)
    info.external_attr = 0o100755 << 16
    info.compress_type = zipfile.ZIP_DEFLATED
    with open(exe, "rb") as f:
        z.writestr(info, f.read())
PY

"$zipsign" sign zip "$archive" "$cle" -f

cles=()
for c in crates/app/cles/verification-*.pub; do
  [ -s "$c" ] && cles+=("$c")
done
if [ "${#cles[@]}" -eq 0 ]; then
  echo "Aucune clé publique dans crates/app/cles/ : l'application refuserait toute mise à jour." >&2
  exit 1
fi
"$zipsign" verify zip "$archive" "${cles[@]}"
echo "$archive"
