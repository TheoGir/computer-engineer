#!/usr/bin/env bash
# Mesure la vitesse du moteur sur cette machine (compilation optimisée complète).
set -euo pipefail
cd "$(dirname "$0")/.."
cargo run --release --locked --quiet -p ce-cli -- mesure
