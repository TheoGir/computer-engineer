#!/usr/bin/env bash
# Contrôles complets, identiques à ceux de la CI. À lancer avant toute livraison.
#
#   outils/verifier.sh          tous les contrôles
#   CE_EXIGER_ICARUS=0 outils/verifier.sh   tolère l'absence d'Icarus Verilog
#
# La comparaison avec Icarus Verilog exige `iverilog` et `vvp`
# (Debian/Ubuntu : apt-get update && apt-get install -y iverilog).
set -euo pipefail
cd "$(dirname "$0")/.."
export CE_EXIGER_ICARUS="${CE_EXIGER_ICARUS:-1}"

etape() { printf '\n== %s (%s)\n' "$1" "$(date +%H:%M:%S)"; }

etape "Formatage"
cargo fmt --all -- --check

etape "Licences des bibliothèques à jour"
python3 outils/licences.py --verifier

etape "Lint sans avertissement"
cargo clippy --workspace --all-targets --locked -- -D warnings

etape "Tests"
cargo test --workspace --locked --profile ci

etape "Documentation sans avertissement"
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked --quiet

printf '\nTout est vert (%s).\n' "$(date +%H:%M:%S)"
