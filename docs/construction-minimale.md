# La construction minimale

Pour chaque construction, l'application dit combien de portes NAND elle utilise et à quel
point c'est proche de la plus petite construction connue : une ligne « 7 portes · Minimum
prouvé : 4 portes », une jauge, trois étoiles et une phrase.

## Où on la voit

- **Fiche de la quête.** Avant de vérifier : le nombre de portes posées et l'objectif, sous
  « Vérifier ». Une fois la quête réussie : les étoiles, la jauge et la phrase, sous la brique
  gagnée. Un ordinateur ou un grand projet se juge sur les portes **ajoutées** à l'atelier
  (ce qu'il fournit déjà ne compte pas).
- **Liste du parcours.** Les étoiles de chaque brique gagnée, à droite de sa quête.
- **Fiche du plan** (hors quête, et dans une brique ouverte). Une brique gagnée se compare au
  corrigé de sa quête ; n'importe quel plan sans mémoire de 3 bits d'entrée au plus se compare
  à la fonction qu'il calcule. Sinon, une ligne dit qu'aucun minimum n'est connu.

## Ce qu'on affirme, et pourquoi c'est vrai

Trois niveaux de certitude, toujours écrits tels quels à l'écran :

| Statut | À l'écran | Sens |
|---|---|---|
| Prouvé | « Minimum prouvé : n portes » | Une construction de n NAND existe et aucune plus petite. |
| Encadré | « Meilleure connue : n portes » | La meilleure construction trouvée ; il en faut au moins b (b < n, prouvé). |
| Référence | « Référence : n portes » | Le corrigé de la quête ; personne n'a prouvé qu'on ne peut pas faire mieux. |
| Borne basse | « Au moins n portes » | Plan à plusieurs sorties non reconnu : n est le minimum prouvé de sa sortie la plus coûteuse. |

**Modèle.** Portes NAND à deux entrées (une porte peut lire deux fois le même signal, ce qui
fait un NON), sans constante : c'est exactement ce que la palette permet. Seul le nombre de
portes compte, pas la profondeur.

**Preuve d'un minimum** (`outils/minimum_nand.py`). Pour k portes, une formule SAT dit « il
existe k NAND, chacune lisant deux signaux plus anciens, dont certains donnent exactement les
sorties voulues sur toutes les lignes de la table ». Elle est satisfaisable si et seulement si
un tel circuit existe. On essaie k = 0, 1, 2… : la première taille satisfaisable est le
minimum. Seule contrainte ajoutée : chaque porte sert ; elle ne change pas la réponse, car un
circuit minimal n'a pas de porte inutile et l'on essaie toutes les tailles depuis 0.

**Recoupements.**
1. Chaque circuit trouvé est resimulé par le script, puis rangé en texte dans
   `crates/circuit/src/minimums.txt` et `minimums3.txt` ; le test `crates/circuit/tests/minimums.rs`
   le resimule sur toutes les combinaisons et le compare au corrigé.
2. `outils/minimum_nand_enumeration.py` énumère **tous** les circuits jusqu'à 7 portes, sans
   SAT : pour les 228 fonctions de 3 entrées atteintes, même minimum ; les 28 autres sont bien
   au-delà de 7 (0 écart). Le test Rust refait cette énumération jusqu'à 6 portes à chaque CI.
3. Valeurs connues de la littérature retrouvées : NON 1, ET 2, OU 3, OU exclusif 4,
   équivalence 5, demi-additionneur 5, additionneur complet 9.

**Plans à plusieurs sorties.** Un minimum par sortie ne donne qu'une borne basse de
l'ensemble (des portes peuvent servir à plusieurs sorties). Quand les sorties sont exactement
celles d'un composant établi (demi-additionneur, démultiplexeur, décodeur 2 vers 4…), à l'ordre
près des entrées et des sorties, c'est son minimum qui s'applique : renuméroter les entrées
ou les sorties ne change pas le nombre de portes.

**Étoiles.** 3 : on atteint la cible (ou mieux). 2 : au plus 150 % de la cible. 1 : au-delà.
Avec une borne basse seule, pas de deuxième étoile (aucune construction connue à approcher).

## Résultats (15/11/2026)

Les 256 fonctions de 3 entrées : toutes résolues (minimum de 0 à 10 portes ; la plus chère,
« exactement une entrée à 1 », en demande 10).

| Corrigé | NAND du corrigé | Meilleure connue | Statut |
|---|---|---|---|
| NON, ET, OU, NON_OU, OU_X, EQUIV | 1, 2, 3, 4, 4, 5 | idem | prouvé |
| MAJORITE, MUX, DEMI_ADD, ADD_COMPLET | 6, 4, 5, 9 | idem | prouvé |
| VERROU_SR | 2 | 2 | prouvé à la main : q et q_n diffèrent l'une de l'autre et des entrées, et sans constante seul un NAND fait un signal nouveau |
| DEMUX | 5 | **4** | prouvé |
| DECODEUR2 | 10 | **9** | prouvé |
| ENCODEUR4 | 15 | **10** | prouvé |
| GRAY | 12 | 12 | prouvé |
| PREMIER | 115 | **8** | prouvé |
| HAMMING_CODE | 24 | **20** | encadré : au moins 13 (SAT) ; la construction de 20 est faite à la main (un OU exclusif partagé par deux parités) et vérifiée |
| autres | — | corrigé | référence |

Le détail exact, à jour, est dans `crates/circuit/src/minimums.txt`.

## Refaire ou étendre la recherche

```sh
pip install python-sat
# tables.json : {"NOM": {"n": bits d'entrée, "sorties": ["0110", ...]}} (voir l'aide du script)
python3 outils/minimum_nand.py tables.json --temps 120 --noms NOM --hauts NOM=<NAND du corrigé> --sortie res.json
python3 outils/minimum_nand_enumeration.py table3.json --portes 7
```

Puis ajouter la ligne dans `minimums.txt` (composant, NAND, borne basse, circuit ou `-`) et
lancer `cargo test -p ce-circuit --test minimums`. Un ordinateur ou un grand projet se règle
dans `crates/app/src/economie.rs` (`AJOUTS`), vérifié par le test
`les_ajouts_de_reference_sont_ceux_des_solutions`.
