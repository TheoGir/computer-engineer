#!/usr/bin/env python3
"""Dessine l'icône de Computer Engineer : une porte NAND néon sur un carré arrondi.

    python3 outils/icone.py

Écrit dans crates/app/icone/ :
- computer-engineer.png (512 × 512) : l'image de référence ;
- computer-engineer.ico (16 à 256) : l'icône de l'exécutable Windows ;
- fenetre.rgba (128 × 128, octets RVBA bruts) : l'icône de la fenêtre, lue sans décodeur.

Les couleurs sont celles de la palette sombre (crates/app/src/apparence.rs).
"""
import math
import os
from PIL import Image, ImageDraw, ImageFilter

ICI = os.path.join(os.path.dirname(__file__), "..", "crates", "app", "icone")
N = 1024  # dessin en grand, réduit à la fin

FOND_HAUT = (0x14, 0x1D, 0x3D)
FOND_BAS = (0x0A, 0x0F, 0x1F)
GRILLE = (0x22, 0x2C, 0x4A)
UN = (0x4D, 0xFF, 0xB5)
PERLE = (0xF2, 0xFF, 0xF9)
ACCENT = (0x65, 0x50, 0xEE)
ZERO = (0x35, 0x41, 0x64)


def carre_arrondi(taille, rayon):
    m = Image.new("L", (taille, taille), 0)
    ImageDraw.Draw(m).rounded_rectangle((0, 0, taille - 1, taille - 1), rayon, fill=255)
    return m


def dessiner():
    # Fond : dégradé vertical et halo violet derrière la porte.
    fond = Image.new("RGBA", (N, N))
    px = fond.load()
    for y in range(N):
        t = y / (N - 1)
        c = tuple(round(FOND_HAUT[i] * (1 - t) + FOND_BAS[i] * t) for i in range(3))
        for x in range(N):
            px[x, y] = c + (255,)
    halo = Image.new("RGBA", (N, N), (0, 0, 0, 0))
    ImageDraw.Draw(halo).ellipse((170, 190, 860, 830), fill=ACCENT + (120,))
    fond = Image.alpha_composite(fond, halo.filter(ImageFilter.GaussianBlur(120)))
    # Grille discrète.
    grille = Image.new("RGBA", (N, N), (0, 0, 0, 0))
    g = ImageDraw.Draw(grille)
    for k in range(1, 8):
        p = k * N // 8
        g.line((p, 0, p, N), fill=GRILLE + (110,), width=3)
        g.line((0, p, N, p), fill=GRILLE + (110,), width=3)
    fond = Image.alpha_composite(fond, grille)

    # La porte NAND : corps en D, rond de sortie, deux entrées, une sortie.
    trait = Image.new("RGBA", (N, N), (0, 0, 0, 0))
    d = ImageDraw.Draw(trait)
    ep = 46
    x0, y0, y1 = 300, 300, 724
    r = (y1 - y0) // 2
    xc = 480  # centre de l'arc
    corps = [(xc, y0), (x0, y0), (x0, y1), (xc, y1)]
    d.line(corps, fill=UN, width=ep, joint="curve")
    # PIL trace l'épaisseur d'un arc vers l'intérieur : on agrandit sa boîte d'une demi-épaisseur.
    h = ep // 2
    d.arc((xc - r - h, y0 - h, xc + r + h, y1 + h), -90, 90, fill=UN, width=ep)
    bulle = 50
    bx = xc + r + h + bulle + 4
    d.ellipse((bx - bulle, 512 - bulle, bx + bulle, 512 + bulle), outline=UN, width=ep - 10)
    # Fils : a = 1 (vert), b = 0 (éteint), sortie = 1.
    d.line((90, 410, x0, 410), fill=UN, width=ep - 6)
    d.line((90, 614, x0, 614), fill=ZERO, width=ep - 6)
    d.line((bx + bulle - 4, 512, 940, 512), fill=UN, width=ep - 6)
    for x, y in ((90, 410), (90, 614)):
        d.ellipse((x - 30, y - 30, x + 30, y + 30), fill=UN if y == 410 else ZERO)

    lueur = trait.filter(ImageFilter.GaussianBlur(34))
    image = Image.alpha_composite(fond, lueur)
    image = Image.alpha_composite(image, lueur)
    image = Image.alpha_composite(image, trait)
    # Perles de courant sur les fils à 1.
    p = ImageDraw.Draw(image)
    for x, y in ((200, 410), (840, 512)):
        p.ellipse((x - 17, y - 17, x + 17, y + 17), fill=PERLE)

    masque = carre_arrondi(N, int(N * 0.225))
    sortie = Image.new("RGBA", (N, N), (0, 0, 0, 0))
    sortie.paste(image, (0, 0), masque)
    return sortie


def main():
    image = dessiner()
    os.makedirs(ICI, exist_ok=True)
    image.resize((512, 512), Image.LANCZOS).save(os.path.join(ICI, "computer-engineer.png"))
    image.resize((256, 256), Image.LANCZOS).save(
        os.path.join(ICI, "computer-engineer.ico"),
        sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )
    with open(os.path.join(ICI, "fenetre.rgba"), "wb") as f:
        f.write(image.resize((128, 128), Image.LANCZOS).tobytes())
    print("Icône écrite dans", os.path.normpath(ICI))


if __name__ == "__main__":
    main()
