#!/usr/bin/env python3
"""Écrit les licences des bibliothèques compilées dans l'exécutable Windows.

    python3 outils/licences.py            écrit crates/app/licences/{resume,textes}.txt
    python3 outils/licences.py --verifier  échoue si ces fichiers ne sont plus à jour

Les licences MIT, BSD, Apache, Zlib et autres demandent de joindre leur texte et leur
mention de droits d'auteur à tout programme distribué qui contient le code : l'application
les montre dans « Aide → À propos », et l'archive publiée les contient. La liste vient de
`cargo tree` (dépendances normales de ce-app pour Windows), les textes des sources des
paquets dans le registre local de cargo (`cargo fetch` les télécharge).
"""
import json
import os
import re
import subprocess
import sys
from pathlib import Path

RACINE = Path(__file__).resolve().parent.parent
SORTIE = RACINE / "crates" / "app" / "licences"
CIBLE = "x86_64-pc-windows-msvc"


def paquets():
    meta = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--locked", "--filter-platform", CIBLE],
        cwd=RACINE))
    par_id = {p["id"]: p for p in meta["packages"]}
    arbre = subprocess.check_output(
        ["cargo", "tree", "-p", "ce-app", "-e", "normal", "--target", CIBLE,
         "--prefix", "none", "--locked", "-f", "{p}"], cwd=RACINE, text=True)
    vus = set()
    for ligne in arbre.splitlines():
        m = re.match(r"^(\S+) v(\S+)", ligne.strip())
        if m:
            vus.add((m.group(1), m.group(2)))
    retenus = []
    for p in par_id.values():
        if (p["name"], p["version"]) in vus and p["source"] is not None:
            retenus.append(p)
    retenus.sort(key=lambda p: (p["name"].lower(), p["version"]))
    return retenus


def textes_de(p):
    dossier = Path(p["manifest_path"]).parent
    fichiers = sorted(
        f for f in dossier.iterdir()
        if f.is_file() and re.match(r"(?i)^(licen[cs]e|copying|notice|unlicense)", f.name))
    return [(f.name, f.read_text(encoding="utf-8", errors="replace").strip()) for f in fichiers]


def main():
    tous = paquets()
    resume = []
    textes = {}
    for p in tous:
        licence = p.get("license") or "voir le fichier de licence"
        resume.append(f"- {p['name']} {p['version']} : {licence}")
        trouves = textes_de(p)
        if not trouves and p.get("license_file"):
            f = Path(p["manifest_path"]).parent / p["license_file"]
            if f.exists():
                trouves = [(f.name, f.read_text(encoding="utf-8", errors="replace").strip())]
        if not trouves:
            # Quelques paquets ne joignent pas de texte : le nom de la licence fait foi.
            trouves = [("sans fichier", f"Licence : {licence}. Le paquet ne joint pas de texte : "
                        "le texte standard de cette licence figure ailleurs dans ce document.")]
        for nom, texte in trouves:
            textes.setdefault(texte, []).append(f"{p['name']} {p['version']} ({nom})")
    polices = sorted((RACINE / "crates" / "app" / "polices").glob("OFL-*.txt"))
    for f in polices:
        police = f.stem.removeprefix("OFL-")
        resume.append(f"- police {police} : SIL Open Font License 1.1")
        textes.setdefault(f.read_text(encoding="utf-8").strip(), []).append(f"police {police}")
    resume_txt = "\n".join(resume) + "\n"
    blocs = []
    for texte, qui in textes.items():
        blocs.append("=" * 72 + "\n" + "Pour : " + ", ".join(qui) + "\n" + "=" * 72 + "\n\n" + texte + "\n")
    textes_txt = "\n".join(blocs)
    attendus = {SORTIE / "resume.txt": resume_txt, SORTIE / "textes.txt": textes_txt}
    if "--verifier" in sys.argv:
        perimes = [str(c.relative_to(RACINE)) for c, t in attendus.items()
                   if not c.exists() or c.read_text(encoding="utf-8") != t]
        if perimes:
            print("Licences pas à jour (les dépendances ont changé) : lancer "
                  "python3 outils/licences.py et ajouter " + ", ".join(perimes), file=sys.stderr)
            sys.exit(1)
        print(f"Licences à jour : {len(tous)} bibliothèques, {len(polices)} polices.")
        return
    SORTIE.mkdir(parents=True, exist_ok=True)
    for c, t in attendus.items():
        c.write_text(t, encoding="utf-8")
    print(f"{len(tous)} bibliothèques, {len(textes)} textes de licence distincts, "
          f"{sum(len(t) for t in attendus.values()) // 1024} Ko.")


if __name__ == "__main__":
    main()
