#!/usr/bin/env python3
"""Cherche le plus petit nombre de portes NAND (à deux entrées) qui calcule une fonction donnée.

    python3 outils/minimum_nand.py tables.json [--temps 60] [--noms NON,ET,...] [--sortie res.json]

`tables.json` associe à chaque composant `{"n": bits d'entrée, "sorties": ["0110", ...]}` :
une chaîne par bit de sortie, le caractère i étant la valeur quand les bits d'entrée valent
ceux de l'entier i (bit 0 = première entrée). Un `x` = valeur libre.

Méthode (« circuit minimal par SAT », Kojevnikov, Kulikov et Yaroslavtsev, 2009) : pour k
portes, une formule dit « il existe k NAND, chacune lisant deux signaux plus anciens (entrées
ou portes), dont certaines donnent exactement les sorties voulues sur toutes les lignes ».
La formule est satisfaisable si et seulement si un tel circuit existe. On essaie k = 0, 1, 2…
- insatisfaisable pour tous les k < m et satisfaisable pour m : le minimum est m (prouvé) ;
- au-delà du temps donné, on garde la borne basse prouvée et le meilleur circuit trouvé.

Seule contrainte ajoutée pour aller vite : chaque porte sert (à une autre porte ou à une
sortie). Elle ne change pas la réponse : un circuit minimal n'a pas de porte inutile, et l'on
essaie tous les k depuis 0. Le circuit trouvé est rendu en texte (`g3=a0,g1` …) et revérifié
ici par simulation ; le test Rust `crates/circuit/tests/minimums.rs` le revérifie encore.

Demande python-sat (`pip install python-sat`).
"""

import argparse
import itertools
import json
import multiprocessing as mp
import sys
import time

from pysat.solvers import Cadical195


def construire(n, sorties, k):
    """Formule CNF pour « k portes suffisent ». Rend (clauses, décodeur)."""
    rangs = 1 << n
    compteur = itertools.count(1)
    x = [[next(compteur) for _ in range(rangs)] for _ in range(k)]
    paires = []  # paires[j] = liste de ((a, b), variable)
    for j in range(k):
        p = []
        for a in range(n + j):
            for b in range(a, n + j):
                p.append(((a, b), next(compteur)))
        paires.append(p)
    choix = [[next(compteur) for _ in range(n + k)] for _ in sorties]
    clauses = []

    def valeur(s, r):
        """(constante, None) pour une entrée, (None, variable) pour une porte."""
        if s < n:
            return ((r >> s) & 1, None)
        return (None, x[s - n][r])

    for j in range(k):
        clauses.append([v for _, v in paires[j]])
        for (a, b), sel in paires[j]:
            for r in range(rangs):
                xa, va = valeur(a, r)
                xb, vb = valeur(b, r)
                g = x[j][r]
                # g = 1 dès qu'une entrée vaut 0.
                for c, v in ((xa, va), (xb, vb)):
                    if c == 1:
                        continue
                    if c == 0:
                        clauses.append([-sel, g])
                    else:
                        clauses.append([-sel, g, v])
                # g = 0 si les deux entrées valent 1.
                cl = [-sel, -g]
                vraie = False
                for c, v in ((xa, va), (xb, vb)):
                    if c == 0:
                        vraie = True
                    elif c is None:
                        cl.append(-v)
                if not vraie:
                    clauses.append(cl)
    for o, table in enumerate(sorties):
        clauses.append(list(choix[o]))
        for s in range(n + k):
            for r in range(rangs):
                t = table[r]
                if t == "x":
                    continue
                c, v = valeur(s, r)
                if c is not None:
                    if c != int(t):
                        clauses.append([-choix[o][s]])
                        break
                else:
                    clauses.append([-choix[o][s], v if t == "1" else -v])
    # Chaque porte sert.
    for j in range(k):
        s = n + j
        cl = [choix[o][s] for o in range(len(sorties))]
        for jj in range(j + 1, k):
            cl += [v for (a, b), v in paires[jj] if a == s or b == s]
        clauses.append(cl)

    def decoder(modele):
        vrai = set(v for v in modele if v > 0)
        portes = []
        for j in range(k):
            for (a, b), v in paires[j]:
                if v in vrai:
                    portes.append((a, b))
                    break
        sorts = []
        for o in range(len(sorties)):
            for s in range(n + k):
                if choix[o][s] in vrai:
                    sorts.append(s)
                    break
        return portes, sorts

    return clauses, decoder


def nom_signal(s, n):
    return f"a{s}" if s < n else f"g{s - n}"


def en_texte(n, portes, sorts):
    morceaux = [f"g{j}={nom_signal(a, n)},{nom_signal(b, n)}" for j, (a, b) in enumerate(portes)]
    morceaux.append("s=" + ",".join(nom_signal(s, n) for s in sorts))
    return ";".join(morceaux)


def simuler(n, portes, sorts, sorties):
    for r in range(1 << n):
        v = [(r >> i) & 1 for i in range(n)]
        for a, b in portes:
            v.append(1 - (v[a] & v[b]))
        for o, s in enumerate(sorts):
            t = sorties[o][r]
            if t != "x" and v[s] != int(t):
                return False
    return True


def essayer(n, sorties, k, file):
    clauses, decoder = construire(n, sorties, k)
    with Cadical195(bootstrap_with=clauses) as sat:
        if sat.solve():
            file.put(("sat", decoder(sat.get_model())))
        else:
            file.put(("unsat", None))


def resoudre(n, sorties, k, temps):
    """'sat' (avec le circuit), 'unsat', ou 'temps' si l'on n'a pas su dans le temps donné."""
    file = mp.Queue()
    p = mp.Process(target=essayer, args=(n, sorties, k, file))
    p.start()
    p.join(temps)
    if p.is_alive():
        p.kill()
        p.join()
        return "temps", None
    return file.get()


def chercher(nom, n, sorties, temps, haut=None, montee=12):
    """Monte depuis 0 tant que c'est insatisfaisable ; si le temps manque, descend depuis `haut`."""
    bas = 0
    journal = []
    meilleur = None
    k = 0
    while True:
        if haut is not None and k >= haut:
            break
        t0 = time.time()
        etat, res = resoudre(n, sorties, k, temps)
        journal.append((k, etat, round(time.time() - t0, 2)))
        if etat == "unsat":
            bas = k + 1
            k += 1
            continue
        if etat == "sat":
            portes, sorts = res
            assert simuler(n, portes, sorts, sorties), nom
            meilleur = (k, en_texte(n, portes, sorts))
            break
        # Le temps a manqué : on continue de monter pour trouver un circuit (plus facile quand
        # k grandit), sans plus rien conclure sur la borne basse.
        k += 1
        while (haut is None or k < haut) and k < bas + montee:
            t0 = time.time()
            etat, res = resoudre(n, sorties, k, temps)
            journal.append((k, etat, round(time.time() - t0, 2)))
            if etat == "sat":
                portes, sorts = res
                assert simuler(n, portes, sorts, sorties), nom
                meilleur = (k, en_texte(n, portes, sorts))
                break
            k += 1
        break
    if meilleur is not None and meilleur[0] > bas:
        # Un circuit trouvé au-dessus de la borne basse : on essaie d'en trouver de plus petits.
        k = meilleur[0] - 1
        while k >= bas:
            t0 = time.time()
            etat, res = resoudre(n, sorties, k, temps)
            journal.append((k, etat, round(time.time() - t0, 2)))
            if etat == "sat":
                portes, sorts = res
                assert simuler(n, portes, sorts, sorties), nom
                meilleur = (k, en_texte(n, portes, sorts))
                k -= 1
            else:
                if etat == "unsat":
                    bas = k + 1
                break
    if meilleur is None and haut is not None:
        # Descendre depuis le corrigé : chaque circuit trouvé abaisse la borne haute.
        k = haut - 1
        while k >= bas:
            t0 = time.time()
            etat, res = resoudre(n, sorties, k, temps)
            journal.append((k, etat, round(time.time() - t0, 2)))
            if etat == "sat":
                portes, sorts = res
                assert simuler(n, portes, sorts, sorties), nom
                meilleur = (k, en_texte(n, portes, sorts))
                k -= 1
            elif etat == "unsat":
                bas = k + 1
                break
            else:
                break
    if meilleur is None and haut is not None and bas == haut:
        # Toutes les tailles sous le corrigé sont impossibles : le corrigé est minimal.
        meilleur = (haut, None)
    return {
        "borne_basse": bas,
        "meilleur": meilleur[0] if meilleur else None,
        "circuit": meilleur[1] if meilleur else None,
        "prouve": meilleur is not None and meilleur[0] == bas,
        "journal": journal,
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("tables")
    ap.add_argument("--temps", type=float, default=60.0)
    ap.add_argument("--noms", default="")
    ap.add_argument("--hauts", default="", help="NOM=k,… : nombre de NAND du corrigé")
    ap.add_argument("--sortie", default="")
    ap.add_argument("--montee", type=int, default=12, help="tailles essayées au-dessus de la borne basse")
    args = ap.parse_args()
    tables = json.load(open(args.tables))
    hauts = dict(
        (h.split("=")[0], int(h.split("=")[1])) for h in args.hauts.split(",") if h
    )
    noms = [x for x in args.noms.split(",") if x] or list(tables)
    resultats = {}
    for nom in noms:
        t = tables[nom]
        r = chercher(nom, t["n"], t["sorties"], args.temps, hauts.get(nom), args.montee)
        resultats[nom] = r
        print(
            f"{nom:<16} n={t['n']} sorties={len(t['sorties'])} borne_basse={r['borne_basse']} "
            f"meilleur={r['meilleur']} prouvé={r['prouve']} {r['journal'][-3:]}",
            flush=True,
        )
    if args.sortie:
        json.dump(resultats, open(args.sortie, "w"), indent=1, ensure_ascii=False)


if __name__ == "__main__":
    sys.exit(main())
