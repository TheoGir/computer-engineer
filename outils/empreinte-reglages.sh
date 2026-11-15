#!/usr/bin/env bash
# Empreinte des réglages du Cargo.toml racine qui changent la façon de compiler les dépendances.
# La CI l'ajoute à la clé de son cache Rust (ci.yml), qui ne lit pas ce fichier.
#
#   outils/empreinte-reglages.sh [Cargo.toml]     → 16 caractères hexadécimaux
#
# Comptent : les profils ([profile.*]), les dépendances communes ([workspace.dependencies]),
# [patch], [replace] et la ligne « resolver ». Sans eux dans la clé, changer un profil laisserait
# le cache « à jour » : tout serait recompilé à chaque CI, sans jamais être enregistré.
#
# Ne comptent pas : la version, la liste des crates, les lints, les commentaires, les lignes
# vides. Ils ne changent rien aux dépendances compilées ; les compter ferait repartir d'un cache
# vide (une vingtaine de minutes) à chaque crate ajoutée.
#
# Demande `awk` et `sha256sum`.
set -euo pipefail

manifeste="${1:-$(dirname "$0")/../Cargo.toml}"
[ -r "$manifeste" ] || { echo "Fichier illisible : $manifeste" >&2; exit 1; }

# Une ligne « [section] » ouvre une section gardée ou non ; ses lignes la suivent. L'en-tête est
# comparé sans espaces ni guillemets : TOML accepte aussi ["profile".release] et [ profile . dev ].
reglages="$(awk '
  /^[ \t]*\[/ {
    entete = $0
    gsub(/[ \t"\047]/, "", entete)
    garder = (entete ~ /^\[(profile|patch|replace|workspace\.dependencies)(\]|\.)/)
  }
  /^[ \t\r]*(#|$)/ { next }
  garder || /^[ \t]*resolver[ \t]*=/ { sub(/[ \t\r]+$/, ""); print }
' "$manifeste")"

# Aucun de ces réglages (fichier écrit autrement) : tout le fichier compte, plutôt qu'une
# empreinte qui ne changerait plus jamais.
[ -n "$reglages" ] || reglages="$(cat "$manifeste")"

printf '%s\n' "$reglages" | sha256sum | cut -c1-16
