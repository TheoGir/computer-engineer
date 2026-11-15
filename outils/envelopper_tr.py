#!/usr/bin/env python3
"""Aide au passage d'un fichier Rust à `tr!` (src/langue.rs), une seule fois par fichier.

    python3 outils/envelopper_tr.py crates/app/src/panneaux.rs [...]

- un texte français affiché (`"Ouvrir le projet"`) devient `tr!("Ouvrir le projet")` ;
- `format!("… {nom} … {}", x)` en français devient `tr!("… {nom} … {x}", nom = nom, x = x)`.

Les tests (`#[cfg(test)]`), les attributs, les constantes, les messages des assertions et
de `expect` ne sont pas touchés. Le résultat se relit et se compile : ce script ne fait que
le travail mécanique.
"""

import re
import sys

# Macros dont les textes ne s'affichent pas à l'utilisateur, ou se traitent à part.
MACROS_A_LAISSER = {
    "assert", "assert_eq", "assert_ne", "debug_assert", "debug_assert_eq", "debug_assert_ne",
    "panic", "unreachable", "todo", "unimplemented", "concat", "include_str", "include_bytes",
    "env", "matches", "tr", "write", "writeln", "print", "println", "eprint", "eprintln",
    "format_args", "format", "log", "debug", "info", "warn", "error", "trace",
}
METHODES_A_LAISSER = {
    "expect", "context", "with_context", "expect_err", "join", "debug_struct", "debug_tuple",
    "field", "var", "env",
}

LETTRES_FR = "a-zA-ZàâäçéèêëîïôöùûüÿœæÀÂÄÇÉÈÊËÎÏÔÖÙÛÜŸŒÆ"


def a_traduire(texte):
    """Vrai pour un texte qu'un humain lit : des mots, une phrase."""
    nu = re.sub(r"\{[^{}]*\}", "", texte)
    if not re.search(f"[{LETTRES_FR}]{{2,}}", nu):
        return False
    if re.search(r"[àâäçéèêëîïôöùûüÿœæÀÂÄÇÉÈÊËÎÏÔÖÙÛÜŸŒÆ«»’]", nu):
        return True
    if re.fullmatch(r"[A-Z][a-z']+[.!?…:]?", nu.strip()):
        return True
    mots = re.findall(f"[{LETTRES_FR}']+", nu)
    if " " in nu.strip() and len(mots) >= 2 and re.search(r"[a-z]{2,}", nu):
        # Pas un chemin, une clé ou du code.
        if re.search(r"[/\\]|::|\.rs\b|\.json\b|^[a-z_]+$", nu):
            return False
        return True
    return False


def lexemes(src):
    """Découpe en (genre, début, fin) : 'str', 'rawstr', 'char', 'comment', 'ident', 'punct'."""
    i, n = 0, len(src)
    out = []
    while i < n:
        c = src[i]
        if c.isspace():
            i += 1
            continue
        if src.startswith("//", i):
            j = src.find("\n", i)
            j = n if j < 0 else j
            out.append(("comment", i, j))
            i = j
            continue
        if src.startswith("/*", i):
            prof, j = 1, i + 2
            while j < n and prof:
                if src.startswith("/*", j):
                    prof, j = prof + 1, j + 2
                elif src.startswith("*/", j):
                    prof, j = prof - 1, j + 2
                else:
                    j += 1
            out.append(("comment", i, j))
            i = j
            continue
        m = re.match(r'(b?r)(#*)"', src[i:])
        if m:
            fin = '"' + m.group(2)
            j = src.find(fin, i + len(m.group(0)))
            out.append(("rawstr", i, j + len(fin)))
            i = j + len(fin)
            continue
        if c == '"' or (c == "b" and i + 1 < n and src[i + 1] == '"'):
            j = i + (2 if c == "b" else 1)
            while src[j] != '"':
                j += 2 if src[j] == "\\" else 1
            out.append(("str" if c == '"' else "bstr", i, j + 1))
            i = j + 1
            continue
        if c == "'":
            m = re.match(r"'(\\.|\\u\{[0-9a-fA-F]+\}|\\x[0-9a-fA-F]{2}|[^\\'])'", src[i:])
            if m:
                out.append(("char", i, i + len(m.group(0))))
                i += len(m.group(0))
                continue
            m = re.match(r"'[a-zA-Z_][a-zA-Z0-9_]*", src[i:])
            out.append(("lifetime", i, i + len(m.group(0))))
            i += len(m.group(0))
            continue
        m = re.match(r"[a-zA-Z_][a-zA-Z0-9_]*", src[i:])
        if m:
            out.append(("ident", i, i + len(m.group(0))))
            i += len(m.group(0))
            continue
        m = re.match(r"\d[\d_]*(\.\d[\d_]*)?([eE][+-]?\d+)?[a-z0-9_]*", src[i:])
        if m:
            out.append(("num", i, i + len(m.group(0))))
            i += len(m.group(0))
            continue
        for p in ("=>", "::", "->", "==", "!=", "<=", ">=", "&&", "||", ".."):
            if src.startswith(p, i):
                out.append(("punct", i, i + len(p)))
                i += len(p)
                break
        else:
            out.append(("punct", i, i + 1))
            i += 1
    return out


def valeur(src_lit):
    """La valeur d'un littéral "..." Rust (échappements simples, suites de ligne)."""
    corps = src_lit[1:-1]
    corps = re.sub(r"\\\n\s*", "", corps)
    return corps


def nom_d_argument(expr, pris):
    expr = expr.strip()
    m = re.fullmatch(r"&?\s*(?:[a-z_][a-zA-Z0-9_]*\s*\.\s*)*([a-z_][a-zA-Z0-9_]*)", expr)
    nom = m.group(1) if m else None
    if nom is None:
        m = re.fullmatch(r"&?\s*(?:[a-z_][a-zA-Z0-9_]*\s*\.\s*)*([a-z_][a-zA-Z0-9_]*)\s*\(\s*\)", expr)
        nom = m.group(1) if m else "v"
    if nom in ("self",):
        nom = "v"
    base, k = nom, 2
    while nom in pris and pris[nom] != expr:
        nom, k = f"{base}{k}", k + 1
    return nom


def separer_arguments(texte):
    """Sépare « a, f(b, c), d » au premier niveau."""
    parts, prof, debut, i = [], 0, 0, 0
    lx = lexemes(texte)
    for genre, a, b in lx:
        t = texte[a:b]
        if genre == "punct":
            if t in "([{":
                prof += 1
            elif t in ")]}":
                prof -= 1
            elif t == "," and prof == 0:
                parts.append(texte[debut:a])
                debut = b
    if texte[debut:].strip():
        parts.append(texte[debut:])
    return [p.strip() for p in parts]


def traiter(src):
    lx = lexemes(src)
    test = re.search(r"#\[cfg\(test\)\]\s*(pub\s+)?mod\s", src)
    limite = test.start() if test else len(src)
    remplacements = []  # (début, fin, texte)
    pile = []  # (ouvrant, nom de macro ou de méthode)
    attribut = 0
    const_prof = None
    prof = 0
    for k, (genre, a, b) in enumerate(lx):
        if a >= limite:
            break
        t = src[a:b]
        if genre == "punct":
            if t == "#" and k + 1 < len(lx) and src[lx[k + 1][1]:lx[k + 1][2]] in ("[", "!"):
                attribut += 1
            if t in "([{":
                nom = None
                if k >= 2 and src[lx[k - 1][1]:lx[k - 1][2]] == "!" and lx[k - 2][0] == "ident":
                    nom = src[lx[k - 2][1]:lx[k - 2][2]] + "!"
                elif k >= 2 and lx[k - 1][0] == "ident" and src[lx[k - 2][1]:lx[k - 2][2]] == ".":
                    nom = "." + src[lx[k - 1][1]:lx[k - 1][2]]
                elif attribut and t == "[":
                    nom = "#"
                    attribut -= 1
                pile.append((t, nom, a))
                prof += 1
            elif t in ")]}":
                if pile:
                    pile.pop()
                prof -= 1
            elif t == ";" and const_prof is not None and prof == const_prof:
                const_prof = None
            continue
        if genre == "ident" and t in ("const", "static") and const_prof is None:
            suivant = src[lx[k + 1][1]:lx[k + 1][2]] if k + 1 < len(lx) else ""
            if suivant not in ("fn", "mut") or t == "static":
                if suivant != "fn":
                    const_prof = prof
            continue
        if genre != "str":
            continue
        if const_prof is not None:
            continue
        noms = [n for (_, n, _) in pile if n]
        if any(n == "#" for n in noms):
            continue
        if any(n.endswith("!") and n[:-1] in MACROS_A_LAISSER and n != "format!" for n in noms):
            continue
        if any(n.startswith(".") and n[1:] in METHODES_A_LAISSER for n in noms):
            continue
        texte = valeur(t)
        # Motif d'un match : laissé tel quel.
        suite = src[b:b + 4].lstrip()
        if suite.startswith("=>") or suite.startswith("|"):
            continue
        dedans_format = pile and pile[-1][1] == "format!" and k >= 1 and src[lx[k - 1][1]:lx[k - 1][2]] == "("
        if dedans_format:
            if not a_traduire(texte):
                continue
            ouvrant = pile[-1][2]
            # Fin de l'appel : la parenthèse correspondante.
            prof2, j = 0, None
            for g2, a2, b2 in lx[k - 1:]:
                s2 = src[a2:b2]
                if g2 == "punct" and s2 in "([{":
                    prof2 += 1
                elif g2 == "punct" and s2 in ")]}":
                    prof2 -= 1
                    if prof2 == 0:
                        j = a2
                        break
            debut_macro = lx[k - 3][1]  # format ! (
            args = separer_arguments(src[b:j].lstrip(",").strip()) if src[b:j].strip(" \n,") else []
            positionnels = [x for x in args if not re.match(r"^[a-z_][a-zA-Z0-9_]*\s*=[^=]", x)]
            nommes = [x for x in args if re.match(r"^[a-z_][a-zA-Z0-9_]*\s*=[^=]", x)]
            if re.search(r"\{[^{}]*:[^{}]*\}|\{\d+\}", texte.replace("{{", "").replace("}}", "")):
                print(f"  à la main (format spécial) : {texte[:70]!r}", file=sys.stderr)
                continue
            pris = {}
            for n in re.findall(r"\{([a-z_][a-zA-Z0-9_]*)\}", texte.replace("{{", "").replace("}}", "")):
                pris[n] = n
            for x in nommes:
                n, e = x.split("=", 1)
                pris[n.strip()] = e.strip()
            ip = 0
            sortie = ""
            reste = texte
            ok = True
            while reste:
                m = re.search(r"\{\{|\}\}|\{([a-z_][a-zA-Z0-9_]*)?\}", reste)
                if not m:
                    sortie += reste
                    break
                sortie += reste[:m.start()]
                if m.group(0) in ("{{", "}}"):
                    sortie += m.group(0)
                elif m.group(1):
                    n = m.group(1)
                    pris.setdefault(n, n)
                    sortie += m.group(0)
                else:
                    if ip >= len(positionnels):
                        ok = False
                        break
                    e = positionnels[ip]
                    ip += 1
                    n = nom_d_argument(e, pris)
                    pris[n] = e
                    sortie += "{" + n + "}"
                reste = reste[m.end():]
            if not ok or ip != len(positionnels):
                print(f"  à la main (arguments) : {texte[:70]!r}", file=sys.stderr)
                continue
            liste = ", ".join(f"{n} = {e}" for n, e in pris.items())
            nouveau = f'tr!("{sortie}"' + (f", {liste})" if liste else ")")
            if not pris:
                nouveau = f'tr!("{sortie}").to_owned()'
            remplacements.append((debut_macro, j + 1, nouveau))
            continue
        if not a_traduire(texte):
            continue
        if k >= 2 and src[lx[k - 1][1]:lx[k - 1][2]] == "(" and src[lx[k - 2][1]:lx[k - 2][2]] == "!":
            continue
        remplacements.append((a, b, f"tr!({t})"))
    # Applique de la fin vers le début ; un format! remplacé englobe ses littéraux.
    remplacements.sort(key=lambda r: (r[0], -r[1]))
    garde, fin = [], -1
    for r in remplacements:
        if r[0] < fin:
            continue
        garde.append(r)
        fin = r[1]
    for a, b, t in reversed(garde):
        src = src[:a] + t + src[b:]
    return src, len(garde)


if __name__ == "__main__":
    for chemin in sys.argv[1:]:
        src = open(chemin, encoding="utf-8").read()
        nouveau, n = traiter(src)
        open(chemin, "w", encoding="utf-8").write(nouveau)
        print(f"{chemin} : {n} textes")
