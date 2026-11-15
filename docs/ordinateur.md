# L'ordinateur miniature

C'est le test d'acceptation du projet : un ordinateur complet, petit mais avec toutes ses
pièces, construit uniquement en portes NAND et simulé porte par porte. Il exécute de vrais
programmes : afficher un texte, calculer la suite de Fibonacci, multiplier deux chiffres tapés au
clavier.

```
ce ordinateur demo
--- calculatrice (touches : 67)
Écran : 6*7=42
139 cycles, 3401 portes NAND.
```

## Ce qu'il contient

| Pièce | Composant | Fait de |
|---|---|---|
| Horloge | `HORLOGE` | la seule primitive qui n'est pas une NAND : elle joue le quartz |
| Processeur | `PROCESSEUR` | décodeur d'instruction, unité de calcul, accumulateur, drapeaux, compteur de programme |
| Mémoire vive | `RAM16` | 16 registres de 8 bits, choisis par un arbre de multiplexeurs |
| Mémoire morte | `MEMOIRE_MORTE` | deux décodeurs, une NAND par instruction, une colonne par bit ; fabriquée à partir du programme |
| Écran | broches `ecran` et `ecran_ecrit` | quand `ecran_ecrit` monte, le caractère sur `ecran` est affiché |
| Clavier | broche `clavier` | le code de la touche enfoncée, 0 sinon |
| Bouton de remise à zéro | broche `raz` | remet `PC`, `A`, `Z` et `C` à zéro |

Les composants s'emboîtent sur six niveaux, de `ORDINATEUR` jusqu'à la porte NAND :

```
ORDINATEUR
├── cpu     PROCESSEUR
│   ├── dec     DECODEUR             code de l'instruction → commandes
│   ├── ual     UAL                  ADD8 (8 × ADD_COMPLET), OU_X, MUX
│   ├── acc     REGISTRE8_RAZ        8 × CELLULE (MUX, ET, BASCULE_D)
│   ├── rz, rc  CELLULE              les drapeaux
│   └── cp      COMPTEUR_PROGRAMME   INCR8, MUX8, REGISTRE8_RAZ
├── rom     MEMOIRE_MORTE            2 × DECODEUR4, lignes, colonnes TOUS_n
└── ram     RAM16 = 2 × RAM8 = 4 × RAM4 = 8 × RAM2 = 16 × REGISTRE8
```

Avec le programme de la calculatrice : 3 401 portes NAND, 49 composants différents.
`ce ordinateur bibliotheque prog.asm` écrit le tout en JSON, au format de la bibliothèque.

## La machine vue par le programmeur

Mots de 8 bits. Un accumulateur `A`, un compteur de programme `PC`, deux drapeaux : `Z` (le
dernier résultat rangé dans `A` vaut zéro) et `C` (retenue). Au plus 256 instructions de 12 bits,
une par cycle d'horloge. 16 octets de mémoire vive (réglable de 2 à 256).

| Code | Instruction | Effet |
|---|---|---|
| 0 | `NOP` | rien |
| 1 | `STA m` | `M[m] ← A` |
| 2 | `IN` | `A ← clavier`, `Z` |
| 3 | `OUT` | affiche `A` |
| 4 | `LDA m` | `A ← M[m]`, `Z` |
| 5 | `ADD m` | `A ← A + M[m]`, `Z`, `C` = retenue |
| 6 | `SUB m` | `A ← A − M[m]`, `Z`, `C` = 1 s'il n'y a pas d'emprunt |
| 7 | `NAND m` | `A ← non (A et M[m])`, `Z` |
| 8 à 11 | `LDI n`, `ADDI n`, `SUBI n`, `NANDI n` | les mêmes avec la valeur `n` au lieu d'une case |
| 12 | `JMP a` | `PC ← a` |
| 13 | `JZ a` | `PC ← a` si `Z` |
| 14 | `JC a` | `PC ← a` si `C` |
| 15 | `HLT` | arrêt |

L'assembleur accepte des étiquettes (`boucle:`), des constantes (`x = 3`), des nombres (`42`,
`0x2A`, `0b101010`), des caractères (`'A'`) et des commentaires après `;`.

```
ce ordinateur exemple calculatrice > calc.asm
ce ordinateur executer calc.asm 98
Écran : 9*8=72
171 cycles d'horloge, machine arrêtée sur HLT. 3401 portes NAND, …
```

## Ce qui est vérifié, et comment

Chaque ligne porte son statut, comme dans `docs/semantique.md`.

| Affirmation | Statut | Test |
|---|---|---|
| L'unité de calcul est juste pour toutes les entrées | prouvé par exhaustion (262 144 cas) | `l_unite_de_calcul_est_juste_pour_toutes_les_entrees` |
| Le décodeur donne les bonnes commandes | prouvé par exhaustion (16 codes × 4 drapeaux) | `le_decodeur_donne_les_bonnes_commandes…` |
| L'incrémenteur et le détecteur de zéro sont justes | prouvé par exhaustion (256 cas) | `l_incrementeur_et_le_detecteur_de_zero…` |
| La mémoire morte rend chaque mot à son adresse | vérifié sur 5 contenus, toutes les adresses | `la_memoire_morte_rend_chaque_mot_a_son_adresse` |
| La mémoire vive se souvient | vérifié sur 600 opérations au hasard | `la_memoire_vive_se_souvient…` |
| La machine entière suit le modèle de référence à chaque cycle (`PC`, `A`, `Z`, `C`, écran, puis mémoire) | vérifié sur 60 programmes au hasard de 200 instructions, 400 cycles chacun, les 16 instructions exécutées | `programmes_aleatoires_compares_au_modele_cycle_par_cycle` |
| Les programmes d'exemple affichent ce qu'il faut | vérifié contre un résultat calculé à part ; la calculatrice sur les 100 paires de chiffres | `bonjour`, `fibonacci`, `calculatrice_au_clavier`, `majuscules_au_clavier` |
| La commande de l'écran monte une fois par caractère, sans parasite | vérifié sur tous les programmes ci-dessus (sonde du moteur) | fonction `comparer` |
| L'horloge laisse le temps au plus long chemin | mesuré : 65 unités après un front montant pour une période de 128 | `l_horloge_laisse_le_temps_au_plus_long_chemin` |
| Tout est en NAND, plus une horloge | prouvé par inspection de la netlist | `la_machine_est_faite_de_quelques_milliers_de_nand…` |

Le modèle de référence (`ordinateur/emulateur.rs`) tient en une centaine de lignes de Rust : il
dit ce que chaque instruction doit faire, sans portes ni horloge.

## Vitesse

`ce mesure`, sur la machine de développement (un seul cœur utilisé) : environ 12 000 cycles
d'horloge par seconde, soit 66 millions d'évaluations de portes par seconde. Chaque cycle
réévalue environ 5 400 portes, surtout parce que l'horloge atteint toutes les bascules de la
mémoire à chaque front.

## Dans l'application

La liste des projets propose « Ordinateur miniature » (programme de la calculatrice) et
« Ordinateur en majuscules ». La bibliothèque de composants est importée telle quelle
(`crates/app/src/importer.rs`) : chaque composant devient une brique de l'application, avec ses
portes NAND, ses horloges et ses bus ; puis le plan et ses briques repassent par la même mise à
plat que les fichiers (`crates/app/src/pont.rs`). Le test
`l_ordinateur_importe_calcule_six_fois_sept_avec_son_clavier_et_son_ecran` vérifie que l'import
garde exactement le nombre de portes de la machine d'origine, et que 6 × 7 donne `6*7=42` sur
l'écran de l'application ; le test d'interface
`l_ordinateur_miniature_s_ouvre_calcule_au_clavier_et_se_visite_brique_par_brique` fait la même
chose à la souris et au clavier.

Mesures (`cargo test --release -p ce-app --test interface -- --ignored --nocapture cadence`,
calcul d'une image sans le rendu par la carte graphique) : 0,7 ms par image à 32 768 pas par
seconde ; à la vitesse « max », la simulation prend le temps de calcul accordé à chaque image
(9 ms par défaut, réglable dans `apparence.json`) et avance d'environ 23 000 pas par image.

Une brique ouverte seule se simule seule : ses interrupteurs remplacent ce qui la pilote dans le
plan du dessus. Ouvrir le processeur de l'ordinateur en marche ne montre donc pas encore les
valeurs de cet exemplaire-là ; c'est la prochaine étape de l'interface.

## Limites connues

- La mémoire vive est inconnue (`X`) à la mise sous tension, comme une vraie : un programme qui
  lit une case avant d'y écrire propage de l'inconnu, et la machine peut s'y perdre.
- Pas d'adressage indirect ni d'appel de sous-programme : le jeu d'instructions est volontairement
  minuscule.
- Dans le moteur seul, l'écran et le clavier sont des signaux lus et pilotés par le banc de
  test ; dans l'application, ce sont des éléments du plan.
- La période de l'horloge est fixée par une mesure, pas par un calcul : le moteur ne sait pas
  encore borner le plus long chemin d'un circuit à mémoire.
