#!/usr/bin/env python3
"""Assemble des captures en une seule planche, pour relire une livraison sans lancer l'appli.

    python3 outils/planche.py planche.png "Titre" "Légende 1=a.png" "Légende 2=b.png" ...

Options : --colonnes N (2 par défaut), --largeur L (largeur d'une vignette, 960 par défaut).
Une légende vide (« =a.png ») laisse la vignette sans texte. Demande Pillow.
"""

import argparse
import sys

from PIL import Image, ImageDraw, ImageFont

FOND = (13, 16, 33)
TEXTE = (226, 230, 245)
DOUX = (150, 158, 190)
MARGE = 40
ECART = 28


def police(taille):
    """Une police lisible si le système en a une, sinon celle de Pillow."""
    for nom in (
        "DejaVuSans.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "Arial.ttf",
        "C:/Windows/Fonts/arial.ttf",
    ):
        try:
            return ImageFont.truetype(nom, taille)
        except OSError:
            continue
    return ImageFont.load_default()


def main():
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("sortie")
    p.add_argument("titre")
    p.add_argument("vignettes", nargs="+", help="« légende=chemin.png »")
    p.add_argument("--colonnes", type=int, default=2)
    p.add_argument("--largeur", type=int, default=960)
    a = p.parse_args()

    vignettes = []
    for v in a.vignettes:
        legende, _, chemin = v.rpartition("=")
        image = Image.open(chemin).convert("RGB")
        h = round(image.height * a.largeur / image.width)
        vignettes.append((legende, image.resize((a.largeur, h), Image.LANCZOS)))

    f_titre, f_legende = police(34), police(22)
    h_legende = 40
    lignes = [vignettes[i : i + a.colonnes] for i in range(0, len(vignettes), a.colonnes)]
    hauteurs = [max(im.height for _, im in ligne) + h_legende for ligne in lignes]
    largeur = 2 * MARGE + a.colonnes * a.largeur + (a.colonnes - 1) * ECART
    haut_titre = 80
    hauteur = haut_titre + 2 * MARGE + sum(hauteurs) + (len(lignes) - 1) * ECART

    planche = Image.new("RGB", (largeur, hauteur), FOND)
    d = ImageDraw.Draw(planche)
    d.text((MARGE, MARGE), a.titre, font=f_titre, fill=TEXTE)
    y = MARGE + haut_titre
    for ligne, h in zip(lignes, hauteurs):
        x = MARGE
        for legende, image in ligne:
            planche.paste(image, (x, y))
            d.rectangle(
                (x - 1, y - 1, x + image.width, y + image.height), outline=(60, 66, 100)
            )
            if legende:
                d.text((x, y + image.height + 10), legende, font=f_legende, fill=DOUX)
            x += a.largeur + ECART
        y += h + ECART
    jpeg = a.sortie.lower().endswith((".jpg", ".jpeg"))
    planche.save(a.sortie, optimize=True, **({"quality": 88} if jpeg else {}))
    print(f"{a.sortie} : {len(vignettes)} vignettes, {largeur}×{hauteur}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
