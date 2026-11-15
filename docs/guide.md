# Guide de Computer Engineer

Computer Engineer fait construire un ordinateur complet à partir d'une seule porte logique, la
NAND. Ce guide dit comment l'installer, s'en servir et régler les problèmes. Dans
l'application, **F1** ouvre la même aide.

## Installer

1. Télécharge l'installateur sur la page des versions :
   <https://github.com/TheoGir/computer-engineer-releases/releases/latest>
   (`computer-engineer-setup-vX.Y.Z-x86_64.exe`).
2. Lance-le. Au tout premier lancement, Windows peut afficher « Windows a protégé votre
   ordinateur » : l'installateur n'est pas signé par un certificat payant. Clique sur
   « Informations complémentaires », puis « Exécuter quand même ». La fenêtre de l'installateur
   peut mettre quelques secondes à apparaître (l'antivirus l'examine) : patiente sans le relancer.
3. Clique sur « Installer » : l'application s'installe pour toi seul, sans mot de passe
   d'administrateur, dans `%LOCALAPPDATA%\Programs\Computer Engineer`. Elle pose un raccourci
   dans le menu Démarrer et, sauf si tu décoches la case, sur le bureau.

Elle apparaît ensuite dans les Paramètres de Windows, « Applications installées », comme toute
application. Rien d'autre à installer : ni droits d'administrateur, ni bibliothèque.

Sans installateur : l'archive `computer-engineer-vX.Y.Z-x86_64-pc-windows-msvc.zip` de la même
page contient l'application seule. Extrais-la dans un dossier à toi (clic droit, « Extraire
tout ») et lance `computer-engineer.exe` ; elle pose alors elle-même ses raccourcis au premier
lancement.

## Mises à jour

L'application cherche une nouvelle version à chaque lancement. Si elle en trouve une, un
message propose « Mettre à jour » : elle la télécharge, **vérifie sa signature** avant de
remplacer quoi que ce soit, puis propose de redémarrer. Paramètres : couper la recherche
automatique, ou « Chercher maintenant ». Une version plus récente ouvre toujours les projets
des plus anciennes ; l'original d'un projet mis au nouveau format est gardé dans
`avant-migration`.

## Premiers pas

- **La langue** : l'application est en français ou en anglais. Au premier lancement, elle
  prend la langue du système ; change-la dans les paramètres, ligne « Langue / Language ».
  En anglais, les noms des briques et des broches livrées s'affichent aussi en anglais (`ET`
  devient `AND`, `raz` devient `reset`), mais le projet garde les noms d'origine : un projet
  fait en anglais s'ouvre tel quel en français.

- **L'accueil** : « Continuer » (ou Entrée) reprend le dernier projet ; le parcours des
  quêtes ; tes projets ; les paramètres.
- **L'atelier** : glisse des portes NAND, des interrupteurs et des lampes depuis la palette,
  tire des fils entre les points de connexion, clique sur les interrupteurs. Le courant se
  propage porte après porte : allumé (1), éteint (0), inconnu ou en conflit (X), non piloté (Z).
- **Bouton poussoir et afficheur 7 segments** : le poussoir vaut 1 tant que tu appuies dessus,
  0 dès que tu le lâches. L'afficheur est une lampe de 7 bits (8 avec le point) : bit 0 = a en
  haut, puis b à f dans le sens des aiguilles d'une montre, bit 6 = g au milieu. Branche-le
  sur la sortie de ton décodeur 7 segments pour voir tes chiffres.
- **Retourner (R)** : une pièce sélectionnée se retourne de gauche à droite, entrées à droite
  et sorties à gauche. Pratique pour une boucle de mémoire : le signal revient vers la gauche
  sans faire le tour.
- **Les briques** : sélectionne plusieurs éléments (Maj + glisser), puis Ctrl+G. La brique va
  dans la palette ; double-clic pour entrer dedans, Échap pour remonter. Sélectionne une
  brique posée pour lui écrire une description (elle s'affiche au survol dans la palette) et
  voir où elle est utilisée.
- **Les quêtes** : de la porte NON jusqu'à plusieurs ordinateurs, et des grands projets.
  « Vérifier » compare ton circuit à un corrigé prouvé ; chaque réussite devient une brique.
  La branche « Dépannage » donne au contraire un circuit déjà construit avec un seul fil mal
  branché : trouve-le et rebranche-le.
- **L'ordinateur miniature** : dans tes projets, ouvre l'exemple « Ordinateur miniature ».
  Clique sur son clavier et tape deux chiffres : il affiche leur produit.

- **Mes briques** : au-delà de quatre briques, un champ « Chercher une brique… » filtre la
  liste par nom ou par description, sans se soucier des accents.
- **Point d'arrêt** : sélectionne un fil d'un bit, puis « Point d'arrêt » dans la barre de la
  sélection. La lecture, même rapide, se met en pause dès que ce fil passe à 1, et dit à
  quel instant. Le même bouton le retire.
- **Recommencer le parcours** : dans les paramètres, « Mes données ». Il faut cliquer deux
  fois ; l'ancien avancement est gardé dans `parcours-avant-recommencement.json`.

## Programmer l'ordinateur

Dans l'ordinateur miniature, **Ctrl+P** ouvre le programme de sa mémoire morte. Écris en
assembleur (F1, « Programmer l'ordinateur », liste les seize instructions), puis « Charger
dans la mémoire » : la mémoire morte est refabriquée en portes NAND et l'ordinateur
redémarre avec ton programme. Sous le texte, « La machine en marche » montre l'instruction en
cours (surlignée), l'accumulateur et chaque case de la mémoire vive (`··` : jamais écrite),
lus sur les portes pendant que l'ordinateur tourne. Exemple, qui écrit `HI` :

```
        LDI 'H'
        OUT
        LDI 'I'
        OUT
fin:    HLT
```

## Comprendre et partager un circuit

- **Bloqué dans une quête ?** Sous « Comprendre ce composant », « Un indice » en révèle un à
  la fois : par où commencer, les briques du corrigé, puis l'expression attendue de chaque
  lampe (pour les circuits sans mémoire).
- **Ctrl+E** analyse le plan affiché quand il n'a pas de mémoire : sa table de vérité complète
  et l'expression simplifiée de chaque lampe (`·` pour ET, `+` pour OU, `¬` pour NON), avec le
  moins de produits possible. Pour analyser une brique seule, entre dedans d'abord.
- **Exporter en Verilog** (dans la même fenêtre) écrit le plan, briques mises à plat, dans le
  dossier `exports` de tes données : on peut le simuler avec un autre outil ou le charger
  dans un vrai circuit programmable (FPGA).
- **Banc d'essai (Ctrl+T)** : écris une ligne par essai, les interrupteurs puis les lampes :

  ```
  a b -> s c
  0 0 -> 0 0
  1 1 -> 0 1
  ```

  « Lancer le banc » joue toutes les lignes dans l'ordre et dit lesquelles sont fausses, avec
  ce qui était attendu et ce qui est sorti. Valeurs en décimal, `0b101` ou `0x1F` ; `-` laisse
  un interrupteur tel quel ou n'attend rien d'une lampe ; `attendre 50` laisse passer le temps.
  La mémoire est gardée d'une ligne à l'autre : un interrupteur peut servir d'horloge. Chaque
  plan et chaque brique garde son banc dans le projet.
- **Automates (Ctrl+K)** : décris des états et leurs transitions, l'application fabrique la
  brique en portes NAND et en bascules D :

  ```
  entrées : bouton
  sorties : voitures, pietons
  état attente : voitures
    si bouton -> orange
  état orange :
    -> traversee
  état traversee : pietons
    si non bouton -> attente
  ```

  Le premier état est celui du départ, où `raz` ramène. À chaque front de `clk`, la première
  transition vraie est suivie, sinon on reste. La brique a aussi une sortie `etat` : le numéro
  de l'état en binaire, dans l'ordre du texte.
- **Exporter en image** : le bouton appareil photo, à côté du zoom, écrit le plan affiché en
  PNG dans le dossier `exports`, sans les panneaux et cadré sur tout le circuit.
- **Copier, coller** : Ctrl+C copie la sélection avec ses fils et ses briques ; Ctrl+V la
  colle ici ou dans un autre projet, en y ajoutant les briques qui manquent. Ce qui est copié
  est aussi du texte : colle-le dans un message pour l'envoyer, la personne qui le reçoit le
  colle dans son application.
- **F1** contient aussi un lexique de près de cinquante mots, l'algèbre de Boole (De Morgan,
  tables de Karnaugh) et une recherche dans toute l'aide.

## Raccourcis

| Touche | Effet |
|---|---|
| F1 | aide |
| Espace | lecture ou pause |
| Ctrl+Z / Ctrl+Y | annuler / rétablir |
| Ctrl+A, Ctrl+G, Ctrl+D | tout sélectionner, regrouper en brique, dupliquer |
| Ctrl+C / Ctrl+X / Ctrl+V | copier / couper / coller, dans ce projet ou dans un autre |
| Ctrl+E | analyser le circuit : table de vérité, expressions, export Verilog |
| Ctrl+T | banc d'essai du plan : écrire et lancer ses essais |
| R | retourner la sélection de gauche à droite |
| Ctrl+K | éditeur d'automates : des états en texte, une brique en NAND |
| Ctrl+P | programme de la mémoire morte |
| Suppr | supprimer |
| Échap | lâcher la sélection, puis remonter d'une brique |
| F, flèches, molette | recadrer, déplacer la sélection, zoomer |

## Tes fichiers

Tout s'enregistre tout seul, une seconde après chaque modification, dans
`%APPDATA%\Computer Engineer` (Mes projets, « Ouvrir le dossier ») : `projets`, `corbeille`,
`parcours.json` (quêtes réussies), `reglages.json`, `avant-migration`, `rapports`, `exports`
(circuits exportés en Verilog). Pour
sauvegarder ou changer d'ordinateur, copie ce dossier.

## En cas de problème

- **La fenêtre ne s'ouvre pas** : un message explique pourquoi. Le plus souvent, le pilote de
  la carte graphique est trop ancien : mets-le à jour (Windows Update ou le site du fabricant).
- **« Computer Engineer est déjà ouvert »** : une seule fenêtre à la fois, pour que deux
  fenêtres n'écrasent pas les mêmes projets. Cherche l'autre dans la barre des tâches.
- **L'application s'est arrêtée brusquement** : au lancement suivant, un message propose
  « Voir le rapport ». Le projet avait été enregistré juste avant. Le rapport (dossier
  `rapports`) ne contient aucun de tes projets ; tu peux l'envoyer pour aider à corriger.
- **Un projet « illisible »** : s'il vient d'une version plus récente, mets l'application à
  jour ; il n'a pas été modifié. S'il est abîmé, il reste dans `projets`, où l'on peut le
  réparer à la main (JSON lisible).

## Désinstaller

Paramètres de Windows, « Applications » puis « Applications installées », Computer Engineer,
« Désinstaller ». L'application, ses raccourcis et son inscription sont retirés. Tes projets
restent dans `%APPDATA%\Computer Engineer` : supprime aussi ce dossier si tu ne veux pas les
garder.

Sans installateur (archive extraite) : supprime le dossier de l'application, puis les raccourcis
du bureau et du menu Démarrer.
