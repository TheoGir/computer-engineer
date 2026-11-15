# Régler l'apparence

Toute l'esthétique de l'application tient dans une seule structure, `Apparence`
(`crates/app/src/apparence.rs`). Le reste du code ne contient aucune couleur, aucun rayon,
aucune durée et aucun seuil de geste en dur. On peut donc changer le style sans toucher au
dessin, et même sans recompiler.

## Sans recompiler

```sh
computer-engineer --ecrire-apparence      # écrit apparence.json dans le dossier courant
computer-engineer                         # lit apparence.json s'il est là
```

Le fichier est cherché à côté de l'exécutable, puis dans le dossier courant. Il est relu
deux fois par seconde : on l'enregistre dans un éditeur de texte et l'application change
sous les yeux. Un fichier invalide ne casse rien : l'apparence en cours est conservée et
l'erreur s'affiche dans un message.

On peut n'écrire que ce qu'on change ; tout le reste garde sa valeur par défaut :

```json
{
  "mode": "clair",
  "sombre": { "un": "#ffd166", "accent": "#ff5ca8" },
  "formes": { "rayon_carte": 8, "fil": 4 },
  "mouvement": { "vitesse_perles": 60, "amortissement": 0.4 }
}
```

## Ce qui se règle

| Section | Contenu |
|---|---|
| `mode` | `"sombre"` ou `"clair"` au lancement. |
| `sombre`, `clair` | Les couleurs de chaque mode, écrites `#rrggbb` ou `#rrggbbaa` : fond, grille, panneaux, texte, signal à 1, signal à 0, indéterminé, attente, accent, danger, ombres. |
| `formes` | Marges, rayons des panneaux et des boutons, largeur des colonnes, hauteur de la barre, épaisseur et lueur des fils, rayon des coudes, perles de courant, grille, taille du texte. |
| `mouvement` | Lissage de la caméra et des éléments, glissade du plan, vitesse du front de courant et des perles, ressort des éléments soulevés, vitesses de simulation proposées, et toutes les transitions (voir plus bas). `sobre` coupe les transitions et les animations décoratives. |
| `manipulation` | Grille d'alignement, distance d'aimantation, distance d'accroche d'un fil, zones sensibles, bornes et sensibilité du zoom, pas du clavier, profondeur d'Annuler. |

### Transitions

Les durées sont en secondes ; à 0, la transition est instantanée. Les courbes s'écrivent
`"lineaire"`, `"sortie"` (rapide puis ralentie), `"entree"`, `"douce"` (lente aux deux bouts) ou
`"rebond"` (dépasse un peu avant de se poser).

| Champ de `mouvement` | Ce qu'il règle |
|---|---|
| `ecran`, `courbe_ecran`, `zoom_ecran` | Fondu entre l'accueil et l'atelier, et léger zoom qui l'accompagne. |
| `rubrique`, `glissement` | Passage d'une rubrique de l'accueil à l'autre, pastille du menu, distance du glissement. |
| `theme` | Fondu entre le thème sombre et le thème clair. |
| `panneau`, `courbe_panneau` | Ouverture et fermeture des panneaux. |
| `plongee`, `courbe_plongee`, `zoom_plongee` | Entrée dans une brique et remontée. |
| `apparition`, `courbe_apparition`, `disparition` | Pose et suppression d'une pièce. |
| `trace_fil`, `eclat` | Fil qui se trace à la pose, se rétracte à la suppression, et l'anneau à l'arrivée. |
| `fete`, `etincelles`, `cas_valide` | Quête réussie : durée de la fête, nombre d'étincelles par voyant, éclat d'un cas validé. |
| `grande_fete`, `confettis`, `badge` | Chapitre ou branche terminés : durée de la pluie de confettis et du bandeau, nombre de confettis ; durée de l'arrivée du badge de la brique gagnée. |
| `eclair_clic`, `lissage_ui`, `allumage`, `eclat_contour` | Boutons : onde du clic, survol, allumage d'un voyant, contour qui s'éclaire. |
| `message_entree`, `message_sortie` | Arrivée et départ des messages passagers. |
| `remplissage`, `lissage_barre`, `lissage_chrono` | Barres d'avancement et chronogramme. |
| `pulsation` | Période des pulsations (pastille d'état, prochaine quête, logo). |

Une transition ne retouche que le dessin : les zones cliquables et l'état du circuit sont déjà
ceux de l'arrivée, et la simulation avance du même pas avec ou sans animations
(`tests/transitions.rs` le vérifie). Les outils sont dans `calques.rs`.

Chaque champ est documenté dans `apparence.rs`. Les valeurs absurdes sont ramenées dans des
bornes où l'application reste utilisable.

Les choix faits dans l'écran Paramètres de l'application (thème, taille du texte, animations,
vitesse au départ) sont enregistrés dans `reglages.json`, à côté des projets, et passent
par-dessus ce fichier : un choix jamais fait laisse la valeur d'`apparence.json`, et la taille du
texte choisie multiplie celle du fichier (`reglages.rs`). `reglages.json` se retouche aussi à la
main : un réglage mal écrit garde sa valeur par défaut, sans emporter les autres.

Quand la barre du haut de l'atelier manque de place (fenêtre étroite, grand texte), les boutons
des panneaux perdent d'abord leurs libellés, puis le nom de l'application et le bouton de vitesse
se réduisent à leur icône. La décision se prend dans `disposition.rs`, d'après les largeurs que le
dessin mesure (`Contenus`). La place du temps « t = … » et celle de l'état ne font que grandir
(`Reserve` dans `panneaux.rs`) : le temps qui défile ou revient à zéro ne déplace pas les boutons.
Si même la rangée réduite ne tient pas, la carte de lecture prend la place qui reste et l'état
s'abrège.

## Dans le code

- **Mise en page** : `crates/app/src/disposition.rs` calcule le rectangle de chaque panneau à
  partir de la taille de la fenêtre. Déplacer un panneau, c'est changer cette fonction, pas
  le dessin du panneau.
- **Dessin du plan** : `crates/app/src/plan.rs`. **Dessin des panneaux** :
  `crates/app/src/panneaux.rs`. Les deux lisent l'apparence.
- **Polices** : trois fichiers dans `crates/app/polices/` (Inter, Space Grotesk, JetBrains
  Mono, licence OFL), déclarés dans `crates/app/src/dessin.rs`. Remplacer un fichier change la
  police partout.
- **Icônes** : des tracés dans `dessin.rs`, sans image.
- **Transitions** : `crates/app/src/calques.rs` (voile sur ce qui vient d'être peint, copie de ce
  qui s'en va). Les durées et les courbes viennent de `mouvement` ; aucune n'est écrite ailleurs.
