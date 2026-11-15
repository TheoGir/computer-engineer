# Sémantique du simulateur : définition, théorèmes et preuves

Ce document dit exactement ce que le moteur calcule, ce qui en est démontré, et comment chaque affirmation est vérifiée. Chaque résultat porte un statut : **prouvé**, **prouvé par exhaustion** (domaine fini entièrement parcouru par un programme), **vérifié sur N cas** (recherche de contre-exemple, sans valeur de preuve) ou **limite connue**.

Pour tout relancer : `outils/verifier.sh`.

## 1. Le modèle

### Valeurs

Un fil porte l'une de quatre valeurs : `0`, `1`, `X` (inconnu) ou `Z` (haute impédance, personne ne pilote).

L'**ordre d'information** `⊑` est défini par : `X ⊑ v` pour toute valeur `v`, et `0`, `1`, `Z` ne sont comparables qu'à eux-mêmes. « `a ⊑ b` » se lit « `b` précise `a` ». `X` signifie donc « je ne peux rien garantir », jamais « une cinquième valeur ».

### Primitives

- **NAND** à deux entrées : `1` si une entrée vaut `0` ; `0` si les deux valent `1` ; `X` sinon. Une entrée flottante (`Z`) est lue comme inconnue.
- **Tampon trois états** (validation `e`, donnée `a`) : `a` si `e = 1` et `a` binaire ; `Z` si `e = 0` ; `X` sinon.
- **Résolution** des pilotes d'un même fil : `Z` est neutre, des pilotes d'accord donnent leur valeur, tout désaccord donne `X`. Un `0` contre un `1` est un **court-circuit**, signalé à part.

Le NAND est la seule porte logique. Le tampon trois états n'est pas constructible en NAND (un NAND ne sait pas se taire) : il existe pour les bus partagés.

### Temps et délai

Le temps est un entier `t = 0, 1, 2, …` Chaque porte `g` a un délai `d ≥ 1`. À chaque instant :

1. **Cible.** `cible_g(t)` est la fonction de la porte appliquée aux valeurs de ses fils d'entrée à l'instant `t`.
2. **Délai inertiel sûr.** Soit `F(t)` la fenêtre des `d` dernières cibles, aux instants `t−d, …, t−1` (les instants négatifs valent `X` : on ne sait rien d'avant la mise sous tension). On pose `sortie_g(0) = X`, puis pour `t ≥ 1` :
   - si toutes les cibles de `F(t)` valent une même valeur `v`, alors `sortie_g(t) = v` ;
   - sinon, si `sortie_g(t−1)` figure dans `F(t)`, alors `sortie_g(t) = sortie_g(t−1)` ;
   - sinon `sortie_g(t) = X`.
3. **Fils.** La valeur d'un fil à `t` est la résolution de ses pilotes à `t` : sorties de portes, entrées externes, constantes, horloges.

En clair : la sortie prend une nouvelle valeur quand la cible l'a tenue pendant tout le délai ; elle garde sa valeur tant que la cible l'a confirmée au moins une fois dans le délai ; sinon on ne sait plus.

Cette définition est le code de `crates/moteur/src/specification.rs`, écrit sans optimisation pour être relu ligne à ligne contre ce texte.

### Ce que le modèle ne représente pas (limites connues)

- Pas d'électronique analogique : ni tensions, ni temps de montée, ni charge d'une sortie par ses lecteurs. Une porte est une fonction logique idéale suivie d'un délai.
- Le délai est le même pour un front montant et un front descendant.
- La métastabilité réelle (une bascule qui hésite) apparaît ici sous deux formes : un `X`, ou une oscillation entretenue quand deux portes de délais strictement égaux se répondent (verrou SR relâché des deux côtés au même instant). Un circuit réel finit par trancher grâce au bruit ; le modèle, déterministe, ne tranche pas.
- Le niveau transistor est prévu plus tard.

## 2. Théorèmes

### Théorème 1 — bonne définition et déterminisme. *Prouvé.*

Pour toute netlist (boucles comprises) dont tous les délais valent au moins 1 et pour tout stimulus, les règles 1 à 3 définissent une valeur unique pour chaque fil à chaque instant.

*Preuve.* Récurrence sur `t`. `sortie_g(t)` ne dépend que de cibles à des instants `< t` (car `d ≥ 1`) et de `sortie_g(t−1)`. Les fils à `t` ne dépendent que des sorties à `t` et du stimulus à `t`. Les cibles à `t` ne dépendent que des fils à `t`. Il n'y a donc aucune dépendance circulaire à l'intérieur d'un instant, et aucun ordre d'évaluation à choisir. ∎

Test associé : `le_resultat_ne_depend_pas_de_l_ordre_des_portes` (200 circuits dont on mélange les portes).

### Théorème 2 — le simulateur événementiel calcule la spécification.

Le simulateur optimisé (`simulateur.rs`) ne recalcule que les portes dont une entrée change. Il garde par porte : sortie, cible, une échéance éventuelle et un drapeau « mixte ».

**2a. Une porte seule : *prouvé par exhaustion* pour tout délai `d ≤ 8`.** Pour un délai fixé, la porte du simulateur et la règle 2 sont deux automates finis. `tests/exhaustif.rs` explore tous les états atteignables de leur produit, avec les quatre cibles possibles à chaque instant, et vérifie l'égalité des sorties : 284 782 états et 1 139 128 transitions pour `d = 8`. Toute suite de cibles, de longueur quelconque, ne passe que par des états vérifiés.

**2b. Une porte seule, `d > 8` : *vérifié*** par les tests différentiels ci-dessous (délais jusqu'à 17).

**2c. Circuit entier : *argument rédigé, et vérifié sur 2 240 circuits*.** L'argument : chaque instant est traité en deux phases, d'abord toutes les sorties arrivées à échéance et la résolution des fils, ensuite l'évaluation, une seule fois, de chaque porte touchée. Une évaluation ne modifie aucun fil de l'instant courant (délai ≥ 1), donc chaque porte voit exactement la suite de cibles de la spécification, et 2a s'applique porte par porte. La résolution par comptage des pilotes à `0`, `1`, `X` est *prouvée par exhaustion* égale à la résolution par pliage jusqu'à 4 pilotes. Vérification : `tests/differentiel.rs` compare les deux simulateurs à chaque instant, sur chaque fil et chaque sortie de porte, sur 2 240 circuits aléatoires (boucles, trois états, pilotes multiples, horloges, délais de 1 à 17, entrées `X` et `Z`). Sept mutations volontaires du simulateur ont été essayées : six sont détectées par ces tests, la septième était du code redondant, supprimé depuis.

### Théorème 3 — sûreté du `X` (monotonie). *Prouvé.*

Soient deux stimuli `S` et `S'` tels que `S(t, i) ⊑ S'(t, i)` pour tout instant et toute entrée (`S'` précise des valeurs que `S` laissait inconnues). Alors, pour tout fil `n` et tout instant `t` : `valeur_S(t, n) ⊑ valeur_S'(t, n)`.

**Corollaire.** Si la simulation affiche `0` ou `1` sur un fil alors que certaines entrées sont inconnues, ce fil a cette valeur quelles que soient les valeurs réelles de ces entrées. Le simulateur ne ment jamais par excès de confiance. Le même raisonnement vaut pour l'état initial : le `X` de la mise sous tension est précisé par n'importe quel état de départ réel.

*Preuve.* Trois lemmes, puis une récurrence sur le temps.

- *Lemme 1 (portes).* NAND et tampon trois états sont monotones. *Prouvé par exhaustion* sur les 256 quadruplets de valeurs (`nand_et_tri_sont_monotones`).
- *Lemme 2 (résolution).* La résolution de deux pilotes est monotone (*prouvé par exhaustion*), associative et commutative ; celle de `n` pilotes en est le pliage, donc monotone.
- *Lemme 3 (délai).* Si `F ⊑ F'` point par point et `p ⊑ p'`, alors `règle(F, p) ⊑ règle(F', p')`. Notons `s` et `s'` les deux résultats. Si `s = X`, rien à prouver. Sinon `s = v` binaire ou `Z`, et deux cas :
  - `F` est constante égale à `v`. Comme `v ≠ X`, chaque cible de `F'` vaut aussi `v`, donc `s' = v`.
  - `F` n'est pas constante, `p = v` et `v` figure dans `F`. Alors `p' = v`, et la cible de `F` qui vaut `v` vaut aussi `v` dans `F'`. Si `F'` est constante, c'est à `v`, et `s' = v`. Sinon `p' = v` figure dans `F'`, et `s' = v`.

  Vérification indépendante *par exhaustion* pour `d ≤ 4` : `delai_inertiel_sur_est_monotone`.
- *Récurrence.* À `t = 0` les sorties valent `X` des deux côtés. Si la propriété vaut pour tous les instants `< t` (cibles) et à `t−1` (sorties), le lemme 3 la donne pour les sorties à `t`, le lemme 2 pour les fils à `t`, le lemme 1 pour les cibles à `t`. ∎

Recherche de contre-exemple : `surete_du_x_sur_circuits_aleatoires`, 1 500 circuits, 4 précisions aléatoires chacun, plus de 100 000 valeurs binaires confirmées et autant de `X` effectivement précisés. Aucun contre-exemple.

**Pourquoi la règle « sinon `X` » est nécessaire.** Le délai inertiel classique (celui de Verilog) traite `X` comme une valeur ordinaire et repousse l'échéance à chaque changement de cible. Il n'est pas sûr. Contre-exemple (`le_x_ne_cache_pas_une_impulsion`) : inverseur de délai 2, entrée `1 1 x 0 x 1 1 1`. Si les deux `x` valaient `0`, la sortie monterait à `1` pendant trois instants. La règle classique annonce `0` tout du long, ce qui est faux pour cette valeur des inconnues. La règle de ce moteur annonce `x` pendant ces trois instants.

### Théorème 4 — temps d'établissement d'un circuit sans boucle. *Prouvé.*

Dans une netlist sans boucle et sans horloge, on définit `L(g) = délai(g) + max L(h)` sur les portes `h` qui pilotent une entrée de `g` (0 s'il n'y en a pas). Si le stimulus ne change plus à partir de l'instant `t0`, alors `sortie_g` est constante à partir de `t0 + L(g)`. Le **chemin critique** est le maximum des `L(g)`.

*Preuve.* Récurrence dans l'ordre topologique. Les pilotes des entrées de `g` sont constants à partir de `r = t0 + max L(h)`, donc ses fils d'entrée et sa cible aussi. Pour `t ≥ r + d`, la fenêtre `F(t)` est tout entière dans la zone constante, donc `sortie_g(t)` vaut cette constante. ∎

Conséquence : la table de vérité d'un composant sans boucle, obtenue en attendant le chemin critique après chaque combinaison, est une **preuve par exhaustion** de sa fonction, pas un échantillonnage. Tests : `le_chemin_critique_borne_le_temps_d_etablissement` (3 000 additions aléatoires, et le pire cas `0xFF + 0 + 1` qui met exactement 17 unités, valeur calculée à la main).

### Théorème 5 — mettre en sommeil une région périodique ne change rien. *Argument rédigé, et vérifié sur 21 500 circuits.*

Pour aller vite, le simulateur cesse de calculer les registres qui ne font que répéter le même mouvement à chaque période d'horloge (`simulateur/sommeil.rs`). Ce théorème dit pourquoi ce qu'on lit ensuite reste exactement ce que donne la spécification.

**Cadre.** Une *région* `R` est un ensemble de portes (la mise à plat désigne les sous-composants de 8 à 128 portes, par exemple un registre ; n'importe quel découpage convient). Un fil est *interne* si tous ses pilotes sont des portes de `R`, *d'entrée* si une porte de `R` le lit et qu'un pilote extérieur peut le changer. `P` est le plus petit multiple commun des périodes des horloges. L'*état* de `R` au début d'un instant `u` est l'état d'automate de chacune de ses portes (sortie, cible, échéance relative, drapeau mixte) ; le théorème 2a dit que cet état et les cibles futures déterminent les sorties futures exactement comme la règle 2.

**Énoncé.** Supposons que, sur la fenêtre `[T, T+P)` :
1. l'état de `R` au début de `T+P` est égal à son état au début de `T`, et aucune porte n'a de changement programmé au début de `T` ;
2. aucune porte de `R` dont la sortie est lue ou pilotée hors de `R` (ou sondée) n'a changé de sortie.

Appelons *surveillé* chaque fil d'entrée qui n'est ni une horloge seule sur son fil, ni piloté seulement par des constantes, sauf ceux qui sont *masqués* : toutes les portes de `R` qui le lisent ont leur autre entrée à `0` pendant toute la fenêtre (un NAND ; pour un tampon trois états, seulement si cette autre entrée est la validation), cette autre entrée étant un fil interne constant sur la fenêtre, une constante, ou un fil d'entrée lui-même surveillé. Quand deux fils d'entrée d'une même porte sont tous deux à `0`, au moins l'un des deux est surveillé, pour que chacun ne soit pas justifié par l'autre.

Si, pour tout instant `u` de `[T+P, U)`, chaque fil surveillé a la même valeur à `u` et à `u − P`, alors pour tout `u` de `[T, U]` l'état de `R` au début de `u` est égal à son état au début de `T + ((u − T) mod P)`, et les fils internes aussi. Pendant `[T+P, U)`, `R` ne change aucun fil lu hors de `R`.

*Preuve.* Récurrence sur `u`. Pour `u ≤ T+P`, c'est l'hypothèse 1. Supposons-le au début de `u` (avec `u < U`) et notons `u' = T + ((u − T) mod P)`. Les sorties des portes de `R` à `u` sont une composante de leur état : égales à celles de `u'`, donc les fils internes aussi. Chaque fil d'entrée non masqué a la même valeur qu'à `u'` : par hypothèse s'il est surveillé (et `u − P` se ramène à `u'` par la récurrence elle-même), par périodicité si c'est une horloge (`P` est multiple de sa période), trivialement si c'est une constante. Un fil masqué peut différer, mais chaque porte qui le lit a son autre entrée à `0` à `u` : fil interne (égal à sa valeur à `u'`, qui est `0`), constante, ou fil surveillé (égal à sa valeur à `u'`). Or un NAND dont une entrée vaut `0` sort `1` quelle que soit l'autre, et un tampon trois états dont la validation vaut `0` sort `Z` quelle que soit sa donnée. Dans le cas où deux fils d'entrée à `0` se masquent l'un l'autre, celui qui est surveillé vaut `0`, ce qui suffit à fixer la cible. Toutes les cibles des portes de `R` à `u` sont donc celles de `u'`, et le même automate déterministe mène au même état au début de `u + 1`. Enfin, par l'hypothèse 2 et la récurrence, les sorties lues hors de `R` sont constantes pendant `[T+P, U)`. ∎

**Mise en œuvre exacte.** Le simulateur observe la région pendant une fenêtre (hypothèses 1 et 2 vérifiées à la fin), garde sa *trace* (les changements d'état de ses portes au fil de la période) et retire ses portes des listes de lecteurs. Il compare ensuite chaque fil surveillé, à chaque changement et à chaque changement attendu qui ne vient pas, à la forme d'onde de la fenêtre. Au premier écart, à l'instant `U`, il reconstitue l'état de `R` au début de `U` en rejouant la trace (exact par le théorème), puis fait évaluer toutes ses portes avec les vraies valeurs de `U` : réévaluer une porte dont les entrées n'ont pas changé ne modifie rien. Avant de rendre la main, chaque appel public (`avancer`, `stabiliser`, `fixer_entree`, `sonder`) reconstitue toutes les régions endormies pour l'instant courant : `valeur`, `codes`, `sortie_porte`, `etat_relatif_porte`, `conflits` lisent l'état exact. `Options { sommeil: false }` désactive le mécanisme.

**Vérification.** `crates/moteur/tests/sommeil.rs` tire 21 500 circuits (boucles, trois états, pilotes multiples, une à trois horloges, délais jusqu'à 4, entrées `X` et `Z`), des découpages en régions quelconques et des suites d'appels quelconques (pas de 1 à 200 instants, `stabiliser`, sondes posées en route). Après chaque appel, il compare le simulateur avec sommeil au simulateur sans sommeil sur tout l'état observable (chaque fil, l'état complet de chaque porte, le repos, les courts-circuits, le journal des sondes), et les valeurs à la spécification. Ces essais font environ 72 000 mises en sommeil et 55 000 réveils. Huit erreurs volontaires dans `sommeil.rs` (contrôle de l'hypothèse 2 retiré, paire masquée sans surveillance, changements attendus non vérifiés, dates non recalées, fil interne variable pris pour un masque, état de fin non comparé, région pas au repos au départ, masque externe mal jugé) sont toutes détectées. `crates/circuit/tests/sommeil.rs` fait de même sur l'ordinateur miniature (trois programmes, arrêts en plein cycle) et sur une mémoire de 128 octets.

**Deux précisions apportées en même temps, valables avec ou sans sommeil.**
- `est_stable` répond selon l'état des portes seulement : une porte que le simulateur doit réévaluer mais dont la cible ne changerait pas ne compte plus comme activité. `stabiliser` peut donc s'arrêter un instant plus tôt qu'avant, à l'instant exact où le circuit est au repos.
- `premier_conflit` désigne, à l'instant du premier court-circuit, le plus petit numéro de fil en court-circuit, au lieu d'un fil qui dépendait de l'ordre interne des calculs.

### Théorème 6 — calculer un circuit synchrone période par période ne change rien. *Lemme de la bascule prouvé par exhaustion ; argument rédigé, et vérifié sur 53 200 circuits.*

Pour aller encore plus vite, le simulateur saute d'une période d'horloge à la suivante quand le circuit est *synchrone* (`simulateur/rapide.rs`, `Options::rapide`). Ce théorème dit pourquoi l'état obtenu à chaque arrêt est exactement celui du calcul porte par porte.

**Cadre.** Une seule horloge, de demi-période `h` (période `2h`), aucun fil à plusieurs pilotes. Une *bascule* est un groupe de neuf NAND câblé exactement comme la bascule D maître-esclave de la bibliothèque (inverseur d'horloge, verrou maître ouvert au niveau bas, verrou esclave ouvert au niveau haut, entrées des NAND dans un ordre quelconque), dont les sept fils internes ne sont lus que par ses propres portes ; ses délais sont quelconques. Tout le reste forme la *logique*, qui doit être sans boucle. Les *sources* de la logique sont les entrées externes, les constantes, l'horloge et les sorties `q`, `q_n` des bascules. Le mode rapide s'arrête aux instants `p, p + 2h, p + 4h, …` d'une même *phase* `φ = p mod 2h` ; les entrées externes ne changent qu'à ces arrêts.

**Lemme A (la bascule), *prouvé par exhaustion* pour chaque jeu de délais et chaque `h` rencontrés.** Le programme simule la bascule seule (son entrée `d` est une entrée externe) et explore en largeur tous ses états atteignables depuis la mise sous tension, l'entrée `d` prenant à chaque instant n'importe laquelle des valeurs `0`, `1`, `X`, sauf dans une *fenêtre* `[f − S, f + H]` autour de chaque front montant `f`. Un état est l'état d'automate des neuf portes (théorème 2a), la phase, la valeur de `d`, son *âge* (instants depuis son dernier changement, plafonné) et la valeur *prise* au dernier front. L'exploration est finie et établit, pour la plus petite fenêtre `S + H` qui convienne :
1. `q` vaut la valeur prise et `q_n` son inverse en dehors des `c_max` instants qui suivent chaque front montant, et `q`, `q_n` ne changent qu'entre `c_min` et `c_max` instants après le front ;
2. un *état établi* `E(niveau, d, prise)` : toutes les portes ont sortie = cible, rien de programmé ;
3. pour chaque phase `φ`, un âge `a(φ)` tel que tout état exploré de phase `φ` où `d` a au moins cet âge est l'état établi correspondant (`a(φ) = ∞` sinon) ;
4. pour chaque phase, les couples (`d`, prise) dont l'état établi a été rencontré.

Un `Z` sur `d` est lu par le NAND exactement comme un `X`, et le NAND est symétrique : le programme le vérifie avant d'explorer. Pour les délais de la bibliothèque (tous à 1), `S = 1`, `H = 0`, `c_min = 2`, `c_max = 6`.

**Lemme B (les temps).** C'est le théorème 4 appliqué à des sources qui changent à des instants connus, la règle mixte comprise. Pour chaque fil `n` de la logique et chaque classe de sources, on calcule le plus court et le plus long chemin (somme des délais) ; un fil ne peut changer pendant une période que dans les intervalles : `[E, A]` pour les entrées (changées à l'arrêt), `[f + E, f + A]` pour chaque front `f` de l'horloge, `[r + c_min + E, r + c_max + A]` pour les bascules (`r` : front montant de la période). Si une source ne change plus après l'instant `t`, chaque porte reprend, au plus tard après le plus long chemin, sortie = cible = sa fonction, sans changement programmé.

**Énoncé.** Une phase `φ` est *admissible* si, avec `r` et `f` les décalages du prochain front montant et descendant (dans `[1, 2h]`) :
- (C1) tout fil lu par la logique, toute sortie de la logique et toute entrée `d` de bascule ont fini de changer au décalage `2h − 1` ;
- (C2) pour chaque bascule, les changements de `d` dus aux causes qui précèdent le front montant (entrées ; front descendant s'il vient avant) tombent strictement entre la fenêtre du front précédent et celle de ce front, et ceux dus aux causes qui le suivent (front montant, bascules ; front descendant s'il vient après) strictement entre la fenêtre de ce front et celle du suivant ;
- (C3) l'âge de `d` à l'arrêt suivant, `2h −` (fin de son dernier intervalle), est au moins `a(φ)`.

Supposons qu'à un arrêt `p` de phase admissible, chaque porte soit établie (sortie = cible = fonction de ses entrées, rien de programmé) et chaque bascule dans un état établi `E(niveau(φ), d, q)` rencontré par l'exploration à la phase `φ`. Alors, à `p + 2h`, l'état exact du simulateur est : chaque bascule `b` dans l'état établi `E(niveau(φ), d_b, v_b)` où `v_b` est la valeur de `d_b` établie par la logique avant le front montant (un `Z` compté comme `X`), chaque fil de la logique à la valeur que donne l'évaluation sans délai de la logique sur les sources finales, chaque porte établie. C'est exactement ce que calcule le mode rapide.

*Preuve.* Par le lemme B, chaque fil de la logique ne change que dans les intervalles annoncés. (C2) : avant le front montant, les causes qui le précèdent ont fini d'agir sur `d` avant la fenêtre ; la valeur de `d` pendant toute la fenêtre est donc celle que donne la logique établie sur les sources d'avant le front (une logique sans boucle établie est la fonction de ses sources : récurrence dans l'ordre topologique), c'est-à-dire ce que le mode rapide lit au moment du front ; les causes qui suivent n'agissent qu'après la fenêtre. Ainsi `d` respecte toutes les fenêtres pendant la période. L'état de départ de la bascule fait partie des états explorés, et la suite de ses entrées (horloge, `d`) est l'une de celles que l'exploration a parcourues : par le lemme A, `q` change seulement entre `r + c_min` et `r + c_max` et prend la valeur `v_b`, ce que le lemme B utilise pour la classe des bascules ; par (C3) et le lemme A.3, la bascule est à `p + 2h` dans l'état établi `E(niveau(φ), d_b(p + 2h), v_b)`. Par (C1) et le lemme B, chaque porte de la logique est établie à `p + 2h`, donc chaque fil vaut la fonction des sources finales. Aucun changement n'est programmé, et aucun front ne tombe à `p + 2h` (sinon (C1) ou (C3) échoue), donc l'instant `p + 2h` ne contient aucune activité en attente, comme dans le simulateur exact. L'état à `p + 2h` vérifie de nouveau l'hypothèse, à la même phase : par récurrence, le résultat vaut à chaque arrêt. ∎

**Mise en œuvre exacte.** À la construction, le simulateur reconnaît les bascules, vérifie que la logique est sans boucle, caractérise chaque jeu de délais (lemme A, environ 25 000 états et 0,1 s pour la bascule de la bibliothèque, mis en mémoire pour tout le programme) et prépare l'analyse des temps. À la fin de chaque `avancer` porte par porte, il contrôle la phase (admissibilité calculée une fois par phase) et l'état de départ ; les régions en sommeil sont d'abord réveillées, car le mode rapide calcule tout. En mode rapide, `avancer` d'un multiple de `2h` évalue la logique niveau par niveau (une évaluation par porte dont une entrée a changé), prend les bascules au front montant, et pose l'état établi des bascules qui ont bougé. Tout autre appel le fait sortir sans rien recalculer, puisque l'état est déjà exact : `avancer` d'une durée qui n'est pas un multiple de la période, `stabiliser`, `sonder`. `fixer_entree` reste permis à l'arrêt. `Options { rapide: false }` désactive le mécanisme. Un circuit qui ne remplit pas le cadre (plusieurs horloges, pilotes multiples, boucle hors des bascules, verrou seul, logique trop lente pour la période) est simulé porte par porte, sans erreur.

**Vérification.** `crates/moteur/tests/rapide.rs` tire 53 200 circuits synchrones (une à huit bascules dont les délais varient d'une instance à l'autre et dont les entrées des NAND sont branchées dans un ordre quelconque, logique aléatoire de NAND et de tampons trois états de délais 1 à 3 lisant entrées, constantes, horloge et sorties de bascules, parfois une boucle, parfois des régions de sommeil, demi-périodes de 8 à 64, entrées `X` et `Z`) et des suites d'appels (surtout des avances d'un nombre entier de périodes, mais aussi des avances quelconques, `stabiliser`, sondes). Après chaque appel, l'état observable complet (chaque fil, l'état de chaque porte, le repos, le journal) est comparé au calcul porte par porte. `crates/circuit/tests/rapide.rs` fait de même sur l'ordinateur miniature (trois programmes, arrêts en plein cycle, remises à zéro) : 390 cycles sur 400 y passent en mode rapide. Douze erreurs volontaires dans `rapide.rs` (fin d'établissement non vérifiée, fenêtres de `d` non vérifiées ou rétrécies d'un instant, âge de `d` non vérifié ou sous-estimé, `X` pris pour un `0`, état interne des bascules non posé ou posé pour le mauvais niveau d'horloge, bascule dont `d` change oubliée au front ou au départ, fronts traités dans un ordre fixe, dans un sens puis dans l'autre) sont toutes détectées. Quatre contrôles ne peuvent pas être mis en défaut par ces essais parce que d'autres contrôles les couvrent déjà : l'état de départ des portes, celui des bascules rencontré par l'exploration, le refus d'un arrêt sur un front, et le retard de `q` dans le lemme A. Ils sont gardés parce que la preuve s'appuie sur eux.

**Lemme C (le code compilé). *Argument rédigé, vérifié sur 2 200 circuits et l'ordinateur miniature.*** Avec `Options::compilation` (par défaut), le simulateur traduit le plan du mode rapide en WebAssembly puis en code machine (`simulateur/rapide/compile.rs`, Cranelift à travers Wasmtime, sans code `unsafe`), une fois le circuit passé 256 périodes en mode rapide, en arrière-plan ; d'ici là, et si la compilation échoue, les périodes sont calculées comme ci-dessus. Le code compilé calcule chaque période autrement, avec le même résultat :

1. *La logique, par blocs.* Les portes de la logique sont rangées dans un ordre topologique (à chaque pas, la porte prête de plus petit numéro, pour garder ensemble celles d'un même composant), puis découpées en blocs de 16 portes consécutives. Chaque bloc a un drapeau ; un bloc dont le drapeau est levé est évalué en entier, dans l'ordre, puis son drapeau est baissé ; quand une porte change la valeur d'un fil lu par un autre bloc, elle lève le drapeau de ce bloc. Les écritures d'une source (entrée changée à l'arrêt, horloge, `q` et `q_n` d'une bascule qui change) lèvent les drapeaux des blocs qui la lisent. *Invariant I* : toute porte qui n'est pas établie (sortie ≠ fonction de ses entrées) est dans un bloc au drapeau levé. Il est vrai au départ (tous les drapeaux levés) et chaque écriture d'une source le préserve. *Une passe sur les blocs, dans l'ordre, établit toute la logique* : par récurrence sur les blocs, quand vient le bloc `k`, les blocs précédents sont établis et ne changeront plus (aucun bloc suivant n'écrit leurs fils). Si son drapeau est baissé, ses portes sont établies par I, et aucune de leurs entrées n'a changé depuis : celles qui viennent d'autres blocs ou des sources auraient levé le drapeau, celles qui viennent du bloc lui-même n'ont pas été réécrites. S'il est levé, chaque porte est évaluée après celles du bloc dont elle dépend, sur des entrées définitives : elle est établie, et tout fil qu'elle change hors du bloc lève le drapeau du lecteur, ce qui préserve I pour les blocs suivants. À la fin, toute la logique est établie ; une logique sans boucle n'a qu'une seule valeur établie pour des sources données (récurrence dans l'ordre topologique), celle que calcule aussi l'évaluation niveau par niveau.
2. *Le front montant.* Le code prend `capture(d)` pour *toutes* les bascules, puis écrit `q` et `q_n` de celles dont la valeur change. Le calcul événementiel ne prend que les bascules *candidates* (dont `d` a changé depuis le front précédent, toutes au départ) ; pour une autre, `d` n'a pas changé depuis le front où elle a pris `capture(d)` : la valeur est la même.
3. *L'état interne des bascules.* Le code garde pour chaque bascule la clé `(niveau, rang(d), v)` de son dernier état interne écrit, et en fin de période n'écrit l'état établi `E` que des bascules dont la clé a changé ; au premier appel après une recopie de l'état du simulateur, il les écrit toutes. Le calcul événementiel écrit celui des bascules *touchées* (`d` ou `v` a changé pendant la période). Une bascule non touchée garde sa clé et son état, déjà `E` de cette clé ; une bascule touchée dont la clé n'a pas changé reçoit le même `E`. Les deux calculs aboutissent au même état.
4. *Les portes.* Le code ne tient à jour que la valeur des fils. Or, à chaque arrêt, chaque porte du calcul événementiel est établie, sans rien de programmé : sa sortie et sa cible sont la valeur de son fil de sortie, le reste de son état ne change pas. Le simulateur reconstitue donc l'état des portes depuis les valeurs (à la sortie du mode rapide, et à la lecture pour `sortie_porte`, `etat_relatif_porte`, `est_stable`).
5. *Les passages.* Toute recopie part de l'état exact du simulateur à un arrêt (entrée en mode rapide, copie du simulateur qui repart sans code instancié, retour au calcul événementiel) : chacun des deux calculs peut prendre la suite de l'autre. Au retour au calcul événementiel, `v` est relue sur `q` et toutes les bascules sont candidates et touchées.

Seul le compteur d'évaluations (`Statistiques::evaluations`), qui mesure le travail et non l'état, diffère : le code compilé compte les portes des blocs évalués. ∎

*Vérification.* Les séries de `crates/moteur/tests/rapide.rs` sont rejouées avec le code compilé dès le départ (1 000 circuits), et en mode mixte (1 200 circuits, dont 200 longs) : compilation à un appel tiré au hasard, retours au calcul événementiel puis au code compilé, copies du simulateur ; 40 000 de plus avec `--ignored`. `crates/circuit/tests/rapide.rs` compare l'ordinateur miniature compilé au calcul porte par porte (trois programmes, 390 cycles compilés sur 400). Dix-neuf erreurs volontaires dans le code compilé et son raccordement (changement non signalé aux blocs lecteurs, aux bascules, au groupe de blocs ; horloge non signalée ; valeur d'un bloc reprise dans le suivant ; `q_n` non inversé ; bascule changée non touchée ; sorties des bascules non signalées ; front montant en un seul temps ; clé des bascules ou groupes candidats mal initialisés ; entrée lue par une bascule non signalée ; entrées non recopiées ; fronts dans le mauvais ordre ; état des portes non reconstitué en sortie, à la lecture ou pour le repos ; retour au calcul événementiel sans relire les bascules ou sans les rendre candidates) sont toutes détectées.

## 3. Comparaison avec Icarus Verilog (*vérifié*)

`crates/circuit/tests/icarus.rs` exporte des circuits aléatoires en Verilog structurel (`nand #d`, `bufif1 #d`), les fait simuler par Icarus Verilog 12.0 et compare toutes les valeurs.

| Famille | Circuits | Valeurs comparées | Résultat |
|---|---|---|---|
| NAND, délai unité, entrées `0 1 x z` | 40 | 1 295 160 | égalité exacte |
| NAND, délais de 1 à 6 | 100 | 2 334 300 | 99,6 % égales ; 8 514 écarts, tous « le moteur dit `x`, Icarus une valeur » ; les 26 circuits sans échéance mixte sont exactement égaux |
| Trois états et pilotes multiples | 71 | 1 942 489 | 99,9 % égales ; 1 014 écarts, tous dans le même sens |

Dans les trois familles : **zéro** cas où Icarus dit `x` et le moteur une valeur, **zéro** contradiction `0` contre `1`.

**Écarts avec Verilog, tous voulus.**
- *Règle mixte* : le théorème 3 l'exige (voir le contre-exemple ci-dessus). À délai unité la fenêtre n'a qu'un instant, le cas n'existe pas, et les deux sémantiques coïncident.
- *Forces de signal* : Verilog sait qu'un tampon trois états à validation inconnue sort « `1` ou `Z` » et le résout contre un autre pilote à `1`. Le moteur n'a pas de forces et répond `x`, ce qui reste sûr.

**Défaut d'Icarus rencontré.** `vvp` plante (erreur de segmentation) sur certaines boucles faites uniquement de tampons trois états, par exemple `bufif1 #1 g (n1, n0, n1);`. Neuf circuits sur 80 de la troisième famille sont écartés pour cette raison. Le moteur les simule, et ils restent couverts par la comparaison à la spécification.

## 4. Circuits de référence (*prouvés par exhaustion* pour les combinatoires)

Tous construits en NAND seulement (`crates/circuit/src/reference.rs`), vérifiés dans `crates/circuit/tests/reference.rs`.

| Composant | Vérification |
|---|---|
| NON, ET, OU, NON_OU, OU_X, MUX, DEMI_ADD, ADD_COMPLET | table de vérité complète contre la définition |
| ADD8 (72 portes) | les 131 072 additions possibles contre l'addition entière |
| SELECTEUR_BUS (trois états) | équivalence exhaustive avec MUX |
| VERROU_SR, VERROU_D | séquences calculées à la main, dont l'état inconnu avant la première écriture |
| BASCULE_D | 500 cycles aléatoires : `q` ne change qu'au front montant |
| REGISTRE8, COMPTEUR4 | 300 chargements aléatoires ; comptage sur 40 fronts, puis 12 550 fronts avec une horloge interne |
| OSCILLATEUR | période = 2 × 3 portes × délai, mesurée |

## 5. Ce qui n'est pas prouvé

- 2c repose sur un argument et des tests, pas sur une preuve mécanisée du programme Rust.
- 2a est prouvé pour `d ≤ 8` ; au-delà, seuls des tests aléatoires (jusqu'à `d = 17`).
- La comparaison avec Icarus est empirique : 211 circuits.
- Les circuits séquentiels de référence sont testés sur des séquences, pas prouvés pour toute séquence.
- Le théorème 5 est un argument rédigé, vérifié par des essais aléatoires ; le programme Rust qui le met en œuvre n'est pas prouvé ligne à ligne.
- Le théorème 6 aussi, à l'exception du lemme A, prouvé par exhaustion par le programme lui-même, pour chaque bascule du circuit. Le lemme C suppose de plus que Cranelift et Wasmtime traduisent fidèlement le WebAssembly produit.
