#!/usr/bin/env python3
"""Recoupe `minimum_nand.py` par une méthode indépendante : l'énumération de TOUS les circuits.

    python3 outils/minimum_nand_enumeration.py resultats.json [--portes 6]

Pour 3 entrées, on énumère tous les circuits d'au plus `--portes` NAND (chaque porte lit deux
signaux plus anciens, éventuellement le même deux fois) et l'on note, pour chacune des 256
fonctions, le plus petit circuit qui la calcule sur l'un de ses signaux. Aucune astuce de
coupe : c'est lent mais évident. Puis on compare au tableau `resultats.json` produit par
`table3` (liste de [fonction, borne basse, meilleur, prouvé, circuit]) : pour chaque fonction
de minimum ≤ `--portes`, les deux méthodes doivent donner le même nombre, et une fonction que
l'énumération n'atteint pas doit avoir un minimum SAT > `--portes`.
"""

import argparse
import json
import sys

ENTREES = [0b10101010, 0b11001100, 0b11110000]  # a0, a1, a2 sur les 8 lignes
MASQUE = 0xFF


def enumerer(max_portes):
    meilleur = {}
    for f in ENTREES:
        meilleur[f] = 0

    def descendre(signaux, k):
        if k == max_portes:
            return
        m = len(signaux)
        for i in range(m):
            for j in range(i, m):
                g = ~(signaux[i] & signaux[j]) & MASQUE
                if meilleur.get(g, 99) > k + 1:
                    meilleur[g] = k + 1
                signaux.append(g)
                descendre(signaux, k + 1)
                signaux.pop()

    descendre(list(ENTREES), 0)
    return meilleur


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("resultats")
    ap.add_argument("--portes", type=int, default=6)
    args = ap.parse_args()
    sat = {r[0]: r for r in json.load(open(args.resultats))}
    enum = enumerer(args.portes)
    ecarts = 0
    for f in range(256):
        _, bas, meilleur, prouve, _ = sat[f]
        e = enum.get(f)
        if e is not None:
            if not (prouve and meilleur == e):
                print(f"écart pour {f:08b} : énumération {e}, SAT {meilleur} (prouvé {prouve})")
                ecarts += 1
        elif bas <= args.portes:
            print(f"écart pour {f:08b} : hors d'atteinte en {args.portes} portes, SAT dit {bas}")
            ecarts += 1
    atteintes = len(enum)
    print(f"{atteintes} fonctions atteintes en ≤ {args.portes} portes ; {ecarts} écart(s)")
    return 1 if ecarts else 0


if __name__ == "__main__":
    sys.exit(main())
