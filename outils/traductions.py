#!/usr/bin/env python3
"""Compléter les traductions anglaises de crates/app/locales/en.json.

1. Lister ce qui manque (le test `traductions` écrit la liste) :

       CE_ECRIRE_TRADUCTIONS=$PWD/manquants.json \\
           cargo test -p ce-app --test traductions

2. Numéroter cette liste, un texte par ligne (les retours à la ligne écrits \\n) :

       python3 outils/traductions.py numeroter manquants.json > a_traduire.txt

3. Écrire les traductions dans un fichier « numéro<TAB>traduction », puis les ajouter :

       python3 outils/traductions.py ajouter manquants.json traduit.txt

Le fichier en.json reste trié, une clé par ligne, pour des différences lisibles.
"""

import json
import sys
from pathlib import Path

EN = Path(__file__).resolve().parent.parent / "crates/app/locales/en.json"


def lire_en():
    return json.loads(EN.read_text(encoding="utf-8"))


def ecrire_en(table):
    version = table.pop("_version", 1)
    tri = {"_version": version}
    for cle in sorted(table):
        tri[cle] = table[cle]
    EN.write_text(json.dumps(tri, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def numeroter(liste):
    for i, texte in enumerate(json.loads(Path(liste).read_text(encoding="utf-8"))):
        print(f"{i}\t{texte.replace(chr(10), chr(92) + 'n')}")


def ajouter(liste, traduit):
    textes = json.loads(Path(liste).read_text(encoding="utf-8"))
    table = lire_en()
    n = 0
    for ligne in Path(traduit).read_text(encoding="utf-8").splitlines():
        if not ligne.strip():
            continue
        numero, _, anglais = ligne.partition("\t")
        cle = textes[int(numero)]
        table[cle] = anglais.replace("\\n", "\n")
        n += 1
    ecrire_en(table)
    print(f"{n} traductions ajoutées, {len(table) - 1} en tout")


if __name__ == "__main__":
    if len(sys.argv) >= 3 and sys.argv[1] == "numeroter":
        numeroter(sys.argv[2])
    elif len(sys.argv) >= 4 and sys.argv[1] == "ajouter":
        ajouter(sys.argv[2], sys.argv[3])
    else:
        print(__doc__)
        sys.exit(1)
