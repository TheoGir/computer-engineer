# Journal des modifications

## Prochaine version

- **Les noms des briques et des broches s'affichent en anglais** dans l'interface anglaise : `ET` devient `AND`, `OU_X` devient `XOR`, `raz` devient `reset`, `retenue` devient `carry`… Partout où un nom de circuit se lit : le plan, la palette, l'inspecteur, les tables de vérité des quêtes, le chronogramme, les sondes, l'analyse (Ctrl+E), les messages, et les consignes des quêtes, réécrites avec ces noms. Seul l'affichage change : le projet enregistre toujours les noms d'origine, si bien qu'un projet fait en anglais s'ouvre à l'identique en français, et inversement. Un banc d'essai (Ctrl+T) accepte les deux écritures (`reset` ou `raz`), et renommer une brique sans changer le nom affiché ne la renomme pas. Une brique garde la même taille sur le plan dans les deux langues. Les noms choisis par l'utilisateur ne sont jamais traduits.

## 0.12.0 — 15 novembre 2026

- **L'application parle anglais** : toute l'interface, l'aide (F1), le lexique, les 100 quêtes et leurs fiches « Comprendre », les exemples et les commentaires des programmes livrés existent en anglais. Au premier lancement, l'application prend la langue du système (le français si le système est en français, l'anglais sinon) ; elle se change ensuite dans les paramètres, ligne « Langue / Language ». Les pluriels et la virgule décimale suivent la langue. Les noms des briques et des broches (`ET`, `raz`, `clk`…) sont des noms de circuit et restent les mêmes dans les deux langues, pour que les projets s'échangent.
- L'éditeur d'automates (Ctrl+K) et les bancs d'essai (Ctrl+T) comprennent aussi les mots-clés anglais : `inputs`, `outputs`, `state`, `if`, `and`, `not`, `wait`.
- Le réglage de la langue s'ajoute à `reglages.json` (champ facultatif `langue`) ; un fichier de réglages d'une version précédente se lit comme avant. Les formats des projets et de l'avancement ne changent pas.
- Vérifié par deux tests : chaque texte du code et des données a sa traduction, avec les mêmes trous à remplir ; et un parcours de tous les écrans en anglais (accueil, parcours, chaque quête, chaque exemple, l'aide, les fenêtres d'outils) ne trouve plus de français.

## 0.11.1 — 15 novembre 2026

Correctif : brancher un bus de 2 ou 3 bits sur un découpeur.

- Le découpeur de la palette a quatre sorties d'un bit et refusait tout bus plus étroit : impossible, par exemple, de séparer le `sel` de 2 bits du multiplexeur à 4 voies ou l'adresse du décodeur 2 vers 4. Tant qu'aucune de ses sorties n'est branchée, il se refait maintenant à la taille du bus qu'on y branche, un fil par bit.
- La consigne du multiplexeur à 4 voies, première quête avec des bus, explique qu'un découpeur sépare `d` et `sel` en fils d'un bit.

## 0.11.0 — 15 novembre 2026

- **Retourner une pièce (R)** : une porte, une brique, un interrupteur ou une lampe se retourne de gauche à droite, entrées à droite et sorties à gauche. Les fils suivent, et un signal qui revient en arrière (une boucle de mémoire, un retour de bus) n'a plus à faire le tour. Aussi dans la barre de la sélection ; Ctrl+Z l'annule.
- **Éditeur d'automates (Ctrl+K)** : décris des états en quelques lignes (« état attente : voitures », « si bouton -> orange »…) et l'application fabrique la brique qui fait la même chose, en portes NAND et en bascules D : les sorties de chaque état, la logique de l'état suivant simplifiée par Quine-McCluskey, une entrée `raz` pour revenir au départ, et une sortie `etat` pour voir où il en est. Vérifié par un test qui compare la brique à l'automate sur des centaines de fronts tirés au hasard.
- Format des projets et de l'avancement : `projet/5` et `parcours/4` (champ facultatif `miroir` des éléments). Les fichiers des versions précédentes s'ouvrent comme avant ; une version plus ancienne refuse clairement un fichier du nouveau format.

## 0.10.0 — 15 novembre 2026

- **Une branche « Dépannage »** : quatre circuits déjà construits (OU exclusif, demi-additionneur, multiplexeur, additionneur complet), chacun avec un seul fil mal branché. Il faut essayer les entrées, trouver les cas faux, remonter le courant jusqu'à la porte fautive et la rebrancher. La panne est calculée à partir du corrigé : un seul fil change, et la table de vérité devient fausse (vérifié par un test pour chaque quête).
- **Exporter le plan en image** : le bouton appareil photo, à côté du zoom, écrit le circuit affiché en PNG dans le dossier `exports`, sans les panneaux, cadré sur tout le circuit, avec les valeurs du moment. Pour un exposé, un cahier, ou le montrer à quelqu'un.
- **Bancs d'essai (Ctrl+T)** : écris tes propres essais, une ligne par cas (`a b -> s c`, puis `1 1 -> 0 1`…), et lance-les tous d'un coup : chaque ligne dit si elle est juste, et sinon ce qui était attendu et ce qui est sorti. Les lignes se jouent dans l'ordre, mémoire comprise : un interrupteur peut servir d'horloge, et `attendre 50` laisse passer le temps. Chaque plan et chaque brique a son banc, enregistré avec le projet.
- Format des projets : `projet/4` (champ facultatif `bancs`). Les projets des versions précédentes s'ouvrent comme avant ; une version plus ancienne refuse clairement un projet du nouveau format au lieu d'en perdre les bancs. L'avancement reste en `parcours/3`.

## 0.9.0 — 15 novembre 2026

Voir l'ordinateur penser, s'arrêter au bon moment, retrouver ses briques.

- **La machine en marche (Ctrl+P)** : sous le programme, l'instruction en cours est surlignée dans le programme chargé, avec l'accumulateur et chaque case de la mémoire vive (`··` tant qu'elle n'a pas été écrite), lus sur les portes NAND pendant que l'ordinateur tourne. Vérifié contre l'émulateur de référence.
- **Point d'arrêt** : sélectionne un fil d'un bit, puis « Point d'arrêt ». La lecture, même la plus rapide, se met en pause dès qu'il passe à 1, et dit à quel instant.
- **Chercher dans « Mes briques »** : au-delà de quatre briques, un champ filtre la liste par nom ou description, sans se soucier des accents.
- **Recommencer le parcours**, dans les paramètres (« Mes données ») : deux clics, et l'ancien avancement est gardé dans `parcours-avant-recommencement.json`.

## 0.8.0 — 15 novembre 2026

De nouvelles pièces pour voir et toucher ses circuits.

- **Bouton poussoir** : vaut 1 tant qu'on appuie dessus, 0 dès qu'on le lâche. Pour une sonnette, un compteur qu'on fait avancer à la main, un « Entrée » d'ordinateur.
- **Afficheur 7 segments** : une lampe de 7 bits (8 avec le point) qui dessine son chiffre. La quête « L'afficheur 7 segments » le montre directement sur sa sortie.
- **Les briques se décrivent** : une description, écrite dans le panneau de la brique, s'affiche au survol dans la palette ; le panneau dit aussi où la brique est utilisée.
- Format des projets et de l'avancement : `projet/3` et `parcours/3` (champ facultatif `aspect` des interrupteurs et des lampes). Les fichiers des versions précédentes s'ouvrent comme avant ; une version plus ancienne refuse clairement un fichier du nouveau format au lieu d'en perdre une partie.
- Le guide précise que la fenêtre de l'installateur peut mettre quelques secondes à apparaître après l'avertissement de Windows.

## 0.7.0 — 15 novembre 2026

Comprendre son circuit, s'en sortir quand on bloque, et partager ce qu'on construit.

- **Des indices dans chaque quête.** Sous « Comprendre ce composant », « Un indice » en révèle un à la fois, du plus léger au plus précis : par où commencer, les briques qu'assemble le corrigé, puis, pour un circuit sans mémoire, l'expression attendue de chaque lampe.
- **Analyser le circuit (Ctrl+E).** Pour un plan sans mémoire : sa table de vérité complète, calculée en simulant chaque combinaison, et l'expression simplifiée de chaque lampe (`·` ET, `+` OU, `¬` NON), avec le moins de produits possible (méthode de Quine-McCluskey, bibliothèque `quine-mc_cluskey`). Un circuit à mémoire ou à horloge est refusé avec la raison.
- **Exporter en Verilog**, depuis la même fenêtre : le plan, briques mises à plat, une primitive `nand` par porte, dans le dossier `exports`. Pour un autre simulateur ou un vrai circuit programmable (FPGA).
- **Copier, couper, coller (Ctrl+C, Ctrl+X, Ctrl+V)**, avec les fils entre les éléments copiés et les briques dont ils ont besoin, dans le même projet ou dans un autre. Une brique de même nom mais différente n'est jamais écrasée : la brique collée prend un nom libre. Ce qui est copié est aussi mis en texte dans le presse-papiers : on peut le coller après avoir relancé l'application, ou l'envoyer à quelqu'un.
- **L'aide grandit.** Une rubrique « Algèbre de Boole » (lois, De Morgan, pourquoi la NAND suffit, de la table au circuit, tables de Karnaugh, Quine-McCluskey), un lexique de 48 mots, et une recherche dans toute l'aide, sans se soucier des accents. Les lois écrites dans l'aide sont vérifiées par un test, pour toutes les valeurs.

## 0.6.0 — 15 novembre 2026

La construction minimale : à quel point ton circuit est-il économe ?

- Chaque quête affiche le nombre de portes NAND de ta construction et la plus petite construction connue, avec ce qu'on en sait vraiment : « minimum prouvé », « meilleure connue » ou « référence ». Avant de vérifier, c'est ton objectif ; une fois la quête réussie, trois étoiles, une jauge et une phrase te disent à quel point tu en es proche.
- La liste du parcours montre les étoiles de chaque brique gagnée. La fiche d'un plan ou d'une brique la compare aussi : au corrigé de sa quête pour une brique gagnée, ou à la fonction qu'elle calcule pour tout circuit sans mémoire de 3 entrées au plus. Un ordinateur ou un grand projet se juge sur les portes que tu ajoutes à l'atelier.
- Les minimums sont démontrés, pas devinés : les 256 fonctions de 3 entrées et 17 corrigés, par un solveur SAT recoupé par l'énumération de tous les circuits jusqu'à 7 portes. Parfois on fait bien mieux que nos corrigés : le nombre premier tient en 8 portes au lieu de 115. Le détail : [docs/construction-minimale.md](https://github.com/TheoGir/computer-engineer/blob/main/docs/construction-minimale.md).

## 0.5.0 — 15 novembre 2026

Un vrai installateur, comme pour toute application Windows.

- Sur la page des versions, `computer-engineer-setup-v0.5.0-x86_64.exe` installe l'application pour toi seul, sans mot de passe d'administrateur, dans `%LOCALAPPDATA%\Programs\Computer Engineer`. Il pose un raccourci dans le menu Démarrer et sur le bureau (case à décocher si tu n'en veux pas), et peut lancer l'application tout de suite.
- L'application apparaît dans les Paramètres de Windows, « Applications installées », avec son icône et sa version, et se désinstalle de là : application, raccourcis et inscription sont retirés, tes projets sont gardés.
- Les mises à jour automatiques continuent de remplacer l'application sur place, sans rien demander ; la version affichée dans les Paramètres de Windows suit.
- Une application déjà ouverte est fermée proprement avant d'être remplacée. Les raccourcis posés par l'installateur ne sont pas reposés en double au premier lancement.
- L'archive `.zip` reste publiée à côté : c'est elle que téléchargent les mises à jour automatiques, et elle sert toujours à qui veut l'application sans l'installer.
- À chaque CI, l'installateur est construit à partir de l'exécutable testé, puis installé, lancé et désinstallé comme par un utilisateur, en vérifiant chaque fichier, raccourci et inscription.

## 0.4.0 — 15 novembre 2026

Une application redessinée de bout en bout, le programme de l'ordinateur s'écrit dans l'application, 40 nouvelles quêtes t'attendent, et rien ne se perd plus : l'application installée propose d'elle-même cette version. (La 0.3.0 n'a pas été publiée : tout ce qu'elle apportait est ici.)

Un nouveau design, « Néon de précision ».

- Tous les écrans sont redessinés : atelier en îlots flottants (barre du haut en trois îlots, volets en contrôle segmenté, état en pilule), table de vérité, chronogramme et journal, accueil en cartes vitrines, parcours avec une carte d'avancement sur trois colonnes quand il y a la place, fiches de quête, projets, corbeille et paramètres.
- Les fenêtres d'aide (F1) et de programme (Ctrl+P) et les info-bulles suivent le même style.
- Lisible sans les couleurs : un signal inconnu (X) est tireté et un signal non piloté (Z) pointillé, sur le plan comme dans le chronogramme, et les pastilles écrivent toujours 0, 1 ou X. Le contraste des textes atteint le niveau AA dans les deux thèmes, vérifié par un test.

Programmer l'ordinateur, et de l'aide partout.

- Dans l'ordinateur miniature, **Ctrl+P** ouvre le programme de sa mémoire morte : écris en assembleur, les erreurs s'affichent pendant que tu tapes, puis « Charger dans la mémoire ». La mémoire morte est refabriquée en portes NAND et l'ordinateur redémarre avec ton programme.
- **F1** ouvre l'aide : premiers pas, raccourcis, les seize instructions du processeur, tes fichiers et les mises à jour, et « À propos » avec les licences des 232 bibliothèques et des polices utilisées.
- Un guide complet : [docs/guide.md](docs/guide.md).

Solide comme une version 1.0 : rien ne se perd, rien ne s'arrête en silence.

- Plantage : un rapport est écrit dans le dossier `rapports` (version, système, message, endroit du code), et au lancement suivant un message le dit et propose « Voir le rapport ». Le projet ouvert avait été enregistré juste avant (enregistrement automatique).
- Fenêtre impossible à ouvrir (pilote de carte graphique trop ancien) : une boîte de message l'explique au lieu de rien du tout, et un rapport est écrit.
- Une seule fenêtre à la fois : une deuxième fenêtre ouverte sur les mêmes projets les aurait écrasés ; elle prévient et se ferme. Le redémarrage après une mise à jour attend que l'ancienne fenêtre soit fermée.
- Fichiers numérotés et migrations (`docs/formats.md`) : un projet ou un avancement d'un format plus ancien est migré à l'ouverture, son original gardé dans `avant-migration` ; un fichier écrit par une version plus récente est refusé avec un message clair et n'est jamais réécrit. Un corpus de vrais fichiers, des fichiers abîmés au hasard, la liste des clés de quêtes et la forme exacte de ce qui est écrit sont vérifiés à chaque CI.
- Le contrat que les versions installées attendent de leurs mises à jour (dépôt, nom d'archive, clé de signature) est écrit dans `docs/mises-a-jour.md` et vérifié à chaque CI.
- Noms et dossiers accentués (`C:\Users\Zoé`) vérifiés.

Vitesse du moteur, sans changer un seul résultat.

- Mode rapide compilé : un circuit qui passe du temps en mode rapide est traduit en code machine, en arrière-plan (WebAssembly compilé par Cranelift à travers Wasmtime, sans code `unsafe`). La logique y est découpée en blocs de 16 portes, sautés quand rien de ce qu'ils lisent n'a changé ; seules les bascules dont l'entrée a bougé sont reprises. L'état à chaque arrêt reste identique au calcul porte par porte (lemme C du théorème 6). L'ordinateur de base passe d'environ 160 000 à environ 600 000 cycles par seconde, l'ordinateur avec 256 octets de 37 000 à environ 58 000, le compteur de 4 à 30 millions. `Options::compilation` et `--sans-compilation` pour comparer. Les exécutables grossissent d'environ 10 Mo (le compilateur Cranelift est embarqué).

Six nouvelles branches de quêtes, des grands projets, une fiche pour comprendre chaque composant, et des fêtes de réussite.

- Le parcours passe de 56 à 78 quêtes. Au parcours principal, qui mène à l'ordinateur, s'ajoutent quatre branches, chacune menant à un grand élément : **la carte réseau** (détecteur de front, cadence, contrôle de parité, récepteur série, deux machines qui se parlent, carte réseau), **la montre numérique** (compteur en cascade, secondes et minutes, heures, réveil, montre), **le contrôleur d'écran** (balayage, police de caractères, mémoire vidéo, sérialiseur, contrôleur d'écran) et **maths et automates** (code de Gray, reconnaisseur de motif, divisible par 3, cellule du jeu de la vie, automate cellulaire de la règle 30, machine d'Euclide). Chaque nouvelle quête est vérifiée contre un corrigé en NAND, lui-même comparé à sa définition.
- Puis de 78 à 96 quêtes : la branche **codes et secrets** (chiffre de César, masque jetable, somme de contrôle, code de Hamming, et un correcteur qui répare tout seul un bit faux), la branche **jeux** (arbitre de pierre-feuille-ciseaux, juge du morpion, dé électronique, meilleur score) et un chapitre **nombres remarquables** dans la branche des maths (nombres premiers, carré, binaire vers décimal, suite de Fibonacci).
- Des **grands projets**, plus grands que l'ordinateur : une mémoire de 64 puis de 256 octets, un ordinateur à 256 octets bâti autour d'elle (il rend à l'envers 24 touches tapées), une messagerie entre deux ordinateurs complets reliés par une boîte aux lettres, et une messagerie secrète où le premier chiffre et le second déchiffre. Ils se vérifient comme l'ordinateur : on tape au clavier, l'écran doit afficher la bonne réponse.
- Chaque quête a sa fiche « Comprendre » : à quoi sert le composant, comment il marche, sa place dans le tout, où on le croise dans la vraie vie, un « Le savais-tu ? » et une petite expérience « À essayer ». Elle est dans la fiche de la quête du parcours, et se déplie d'un clic dans le panneau de la quête.
- Le parcours montre ses branches, et chaque chapitre dit ce qu'on y apprend. Dans l'atelier, le panneau de la quête dit où l'on est (« Calcul · étape 3 sur 14 ») avec une pastille par étape du chapitre.
- Réussir une quête remplit son étape en rebondissant, et le badge de la brique gagnée arrive avec un reflet. Terminer un chapitre ou une branche fait pleuvoir des confettis sur le plan, avec un bandeau qui annonce la suite. Durées et nombre de confettis dans `apparence.json` (`grande_fete`, `confettis`, `badge`) ; rien de tout cela en animations réduites.

Un raccourci sur le bureau pour lancer l'application.

- Au premier lancement sous Windows, l'application pose un raccourci « Computer Engineer », avec son icône, sur le bureau et dans le menu Démarrer (on la retrouve aussi en tapant son nom après la touche Windows). Un message le dit.
- Paramètres, « Raccourcis pour lancer l'application » : « Sur le bureau » et « Dans le menu Démarrer », Oui ou Non. Un raccourci supprimé à la main n'est pas remis au lancement suivant.
- Le raccourci suit l'application : une mise à jour la remplace sur place, et si son dossier est déplacé, le raccourci est refait vers le nouvel endroit au lancement suivant.
- Lancée directement depuis l'archive sans l'extraire, l'application ne pose pas de raccourci (il mènerait à un fichier temporaire que Windows efface) et explique comment faire.

## 0.2.0 — 14 novembre 2026

Un accueil, des menus, une icône, et des animations partout. L'application installée propose d'elle-même cette version.

Des animations et des transitions partout.

- Changer d'écran ne saute plus : l'accueil et l'atelier se fondent l'un dans l'autre, les rubriques de l'accueil glissent, la pastille du menu suit, et le thème sombre ou clair passe en fondu. Le premier écran apparaît en douceur au lancement.
- Entrer dans une brique plonge dedans, et en sortir remonte vers elle.
- Une pièce posée apparaît avec un petit rebond, une pièce supprimée s'efface en rapetissant. Un fil posé se trace d'un bout à l'autre, un fil retiré se rétracte.
- Quête réussie : les voyants lancent des anneaux et des étincelles, et le bouton « Quête suivante » arrive juste sous les félicitations, sans avoir à faire défiler.
- Les boutons répondent au survol et au clic, les barres d'avancement se remplissent, les cartes du parcours et les messages passagers arrivent en glissant.
- Paramètres, « Animations » : « Complètes » ou « Réduites ». Réduites, plus aucune transition, perle de courant ni pulsation : tout change d'un coup.
- Les animations ne ralentissent pas la simulation et ne changent rien à ce qu'elle affiche : un clic pendant une transition va déjà au nouvel écran.
- Les durées et les courbes se règlent dans `apparence.json`, section `mouvement` (voir `docs/apparence.md`).
- Sur une brique à plusieurs sorties, les noms des sorties ne se chevauchent plus.
- Le message de réussite d'une quête ne s'affiche plus deux fois quand le panneau de la quête est ouvert.

Un écran d'accueil, des menus, et une icône.

- L'application s'ouvre sur un accueil au lieu du plan : « Continuer » (ou Entrée) reprend le dernier projet là où il en était, avec son aperçu animé ; la prochaine quête conseillée, l'avancement par chapitre et les projets récents sont à un clic. Au tout premier lancement, l'accueil propose la première quête et explique comment on joue.
- Parcours : les 8 chapitres et leurs 56 quêtes, avec ce qui est réussi et ce qui est conseillé. La fiche d'une quête donne sa consigne, la brique qu'elle fait gagner et comment elle est vérifiée ; on la commence, ou on reprend le projet où on l'avait commencée.
- Mes projets : tous les projets enregistrés, du plus récent au plus ancien, avec leur quête et leur date. On les ouvre, on les renomme, on les duplique, on les met à la corbeille (avec « Annuler » juste après), et on ouvre le dossier où ils sont rangés. Les exemples tout faits sont en bas de la liste.
- Paramètres : thème sombre ou clair, taille du texte, animations complètes ou réduites, vitesse de simulation au départ, accueil ou reprise directe au lancement, recherche automatique des mises à jour et recherche immédiate, dossier des données, raccourcis clavier. Tout s'applique et s'enregistre tout de suite ; le thème choisi avec le bouton soleil ou lune est retenu aussi.
- Dans l'atelier, un clic sur le nom de l'application, en haut à gauche, ramène à l'accueil (la recherche de mise à jour est passée dans les paramètres). Sur l'accueil, Échap revient au projet.
- L'application a son icône, une porte NAND néon : dans la barre des tâches, la fenêtre et l'explorateur de fichiers. Le titre de la fenêtre donne le nom du projet ouvert.
- Renommer un projet en ne changeant que des majuscules (« brouillon » en « Brouillon ») ne le rebaptise plus « Brouillon 2 » sous Windows : le fichier est d'abord renommé, puis réécrit. Une coupure de l'application à ce moment-là ne laisse ni doublon ni projet perdu.
- Un exemple tout fait n'est rangé dans tes projets que si tu le modifies : le regarder ne laisse pas de copie. Le rouvrir après l'avoir modifié ouvre une copie neuve (« Ordinateur miniature 2 ») au lieu d'écraser la tienne.
- Un projet copié ou renommé dans l'explorateur de fichiers prend le nom de son fichier et y reste tant qu'on ne le renomme pas, sans écraser le projet dont il portait le nom, même si ce nom contient des signes que l'application n'écrit pas elle-même.
- Si le dernier projet ne peut pas être rouvert au lancement (fichier abîmé, écrit par une version plus récente), l'accueil le dit, et plus rien n'est écrit par-dessus un autre projet.
- Dupliquer ou renommer un projet au nom très long ne bloque plus l'application, et un long nom en lettres larges (émojis, caractères rares) ne dépasse plus la longueur que le disque accepte.
- Un enregistrement qui échoue (disque plein, dossier inaccessible) est signalé une fois, puis retenté jusqu'à réussir, au lieu d'être tenu pour fait.
- Un avancement illisible (`parcours.json` abîmé ou écrit par une version plus récente) n'est plus remplacé à la prochaine quête réussie : il est gardé sous le nom `parcours-illisible.json`, et l'application le dit.
- Deux projets du même nom mis à la corbeille l'un après l'autre y restent tous les deux.
- Les messages passagers de l'accueil s'en vont après quelques secondes, et ne cachent plus de clic une fois partis.
- Un réglage illisible dans `reglages.json` (retouché à la main) garde sa valeur par défaut sans emporter les autres : la recherche automatique des mises à jour reste coupée si on l'avait coupée. Un fichier dont l'écriture est abîmée (virgule en trop, guillemets oubliés) ou enregistré en UTF-16 par PowerShell est lu réglage par réglage, et l'application le signale. Un fichier de réglages qu'on ne peut pas lire du tout n'est plus remplacé.
- Les noms de projet que Windows réserve aux périphériques (COM5, LPT4…) sont acceptés.
- La barre du haut de l'atelier ne se chevauche plus dans une fenêtre étroite ou avec un grand texte : les boutons des panneaux perdent d'abord leurs libellés, puis le nom de l'application et la vitesse se réduisent à leur icône, et en dernier recours l'état s'abrège. Elle ne bouge plus quand le temps défile ou revient à zéro.

## 0.1.0 — 13 novembre 2026

Première version publiée : une archive Windows à décompresser, qui se met ensuite à jour toute seule.

Mises à jour automatiques.

- Au lancement, l'application cherche une nouvelle version en arrière-plan, sans jamais bloquer ni gêner sans réseau. Si elle en trouve une, un message propose « Mettre à jour » : la version est téléchargée, sa signature vérifiée, puis l'application propose de redémarrer dessus. Une archive dont la signature n'est pas valide est refusée et rien n'est modifié.
- Un clic sur le nom de l'application, en haut à gauche, cherche une nouvelle version à tout moment.
- La recherche automatique se coupe dans `reglages.json`, à côté du dossier des projets.
- Si le remplacement de l'exécutable échoue en cours de route (antivirus qui bloque le fichier, disque plein), l'ancienne version est remise en place. Un dossier où l'on ne peut pas écrire (Program Files) est signalé avant tout téléchargement.

Le parcours des quêtes, de la porte NON à l'ordinateur complet.

- 56 quêtes en 8 chapitres, dans le panneau Quête : portes logiques, aiguillages et codes (multiplexeurs, décodeurs, encodeur, afficheur 7 segments, parité), calcul (additionneurs dont un rapide, comparateurs, soustracteur, décaleur, multiplicateur, diviseur, unité de calcul), mémoire (verrous, bascules D, T et JK, registres, banc de registres, mémoires vives, pile, file), compteurs et minuteries (décimal, haut-bas, aléatoire, feu tricolore, chronomètre), communication (liaison série, interruptions), le processeur, et trois ordinateurs complets.
- Le bouton « Vérifier » compare ton circuit à un corrigé fait de portes NAND : toutes les combinaisons quand c'est possible, sinon des centaines de cas choisis, et pas à pas pour les circuits à mémoire. Le message dit le premier cas faux.
- Chaque quête réussie fait gagner ta solution comme brique, disponible dans la palette des quêtes suivantes : l'additionneur se construit avec ton demi-additionneur, et ainsi de suite jusqu'à l'ordinateur. L'avancement est enregistré.
- La dernière quête demande de bâtir l'ordinateur toi-même : processeur, mémoire vive, mémoire morte, horloge, clavier et écran, puis de lui faire calculer 6 × 7.
- Ouvrir une brique posée montre maintenant cet exemplaire précis en marche, avec ses valeurs en direct, au lieu d'une copie simulée à part. Dans l'ordinateur ouvert, on voit le processeur calculer pendant qu'on tape.

Des briques réutilisables, et l'ordinateur miniature dans l'application.

- Sélectionne des portes (Maj + glisser un cadre, ou Maj + clic), puis « Regrouper » ou Ctrl+G : elles deviennent une brique, rangée dans la palette, qui se pose autant de fois qu'on veut et se range dans d'autres briques sans limite de profondeur. Le circuit se comporte exactement comme avant.
- Double-clic sur une brique pour l'ouvrir et voir ou modifier son plan ; ce qu'on y change vaut partout où elle est posée. Le fil d'Ariane, en haut du plan, ramène au niveau voulu ; Échap remonte d'un niveau.
- Le plan entier s'enregistre comme une brique : ses interrupteurs et ses lampes deviennent ses broches.
- Nouveaux éléments dans la palette : horloge, bouton de démarrage, écran, clavier, et des fils de plusieurs bits (bus) avec découpeur et jonction.
- L'ordinateur miniature s'ouvre depuis la liste des projets : on clique sur son clavier, on tape deux chiffres, son écran affiche le produit. On peut l'ouvrir brique par brique jusqu'aux portes NAND. Un second exemple réécrit en majuscules ce qu'on tape.
- Vitesses rapides (jusqu'à « max ») pour les circuits à horloge ; la barre du haut affiche la vitesse réellement atteinte.
- Les projets s'enregistrent tout seuls, une seconde après chaque modification, et le dernier se rouvre au lancement.
- Le panneau des infos permet de renommer un élément ou une brique, de changer le nombre de bits, les morceaux d'un découpeur, la période d'une horloge.
- Les grands plans restent fluides : seuls les éléments visibles sont dessinés, le tracé des fils n'est recalculé que s'ils bougent.

Vitesse du moteur, sans changer un seul résultat.

- Noyau événementiel réécrit : portes compactes, chemin rapide pour les portes de délai 1. Environ 1,3 à 1,7 fois plus rapide sur la logique active.
- Mise en sommeil des registres qui répètent le même cycle à chaque période d'horloge, avec réveil exact au premier écart (théorème 5 de `docs/semantique.md`). La mémoire de 4 Ko en NAND passe de 90 à 14 000 cycles par seconde, l'ordinateur avec 256 octets de mémoire de 1 600 à 15 000, l'ordinateur de base de 16 600 à 48 000.
- Mode rapide pour les circuits synchrones (bascules D et logique sans boucle sous une même horloge) : le simulateur saute d'une période d'horloge à la suivante, avec un état identique, fil par fil et porte par porte, au calcul porte par porte (théorème 6 de `docs/semantique.md`). Il revient seul au calcul porte par porte dès qu'on regarde l'intérieur d'une période. L'ordinateur de base passe de 48 000 à environ 167 000 cycles par seconde, l'ordinateur avec 256 octets de 15 000 à 37 000, un compteur de 830 000 à 3,4 millions. `Options::rapide` et `--sans-rapide` pour comparer.
- `ce mesure` : mesures reproductibles (mémoires jusqu'à 64 Ko, multiplicateur 32 bits, ferme de 64 ordinateurs), vérifiées contre leurs modèles, `--json` pour comparer, `--sans-sommeil` pour mesurer sans la mise en sommeil.
- `est_stable` et `stabiliser` ne comptent plus comme activité une porte dont la réévaluation ne changerait rien ; `premier_conflit` ne dépend plus de l'ordre interne des calculs.

Le test d'acceptation : un ordinateur complet miniature.

- Un ordinateur entier en portes NAND : processeur 8 bits, 16 octets de mémoire vive, mémoire morte fabriquée à partir du programme, horloge, sortie écran, entrée clavier, bouton de remise à zéro. 3 401 portes, six niveaux de composants emboîtés.
- Seize instructions, un assembleur avec étiquettes et constantes, des programmes d'exemple : afficher un texte, la suite de Fibonacci, une calculatrice qui multiplie deux chiffres tapés au clavier.
- La machine en NAND est comparée cycle par cycle à un modèle de référence sur des programmes tirés au hasard ; l'unité de calcul et le décodeur sont vérifiés sur toutes leurs entrées.
- Commandes `ce ordinateur demo`, `executer`, `bibliotheque`, `exemple`.

Étape 2 de la feuille de route, en cours : l'application.

- Application graphique native `computer-engineer` : le plan du circuit, en sombre ou en clair.
- On déplace les portes, on tire les fils d'un point de connexion à un autre, ils s'accrochent tout seuls au point le plus proche et se redressent quand une porte est presque alignée.
- Le courant est animé : la nouvelle valeur parcourt les fils, les portes qui vont basculer sont entourées, celles qui basculent lancent un éclat.
- Zoom à la molette autour du pointeur, déplacement du plan avec glissade, recadrage automatique.
- Annuler et rétablir, suppression, déplacement au clavier.
- Première quête : le demi-additionneur, validé cas par cas par la simulation.
- Panneaux repliables : briques, quête, infos, chronogramme et journal. Cliquer dans le chronogramme remonte le temps.
- Toute l'apparence (couleurs, tailles, vitesses des animations, seuils des gestes) se règle dans un fichier `apparence.json`, relu pendant que l'application tourne.

Étape 1 de la feuille de route : le moteur de simulation.

- Logique à quatre valeurs (`0`, `1`, `X`, `Z`), porte NAND, tampon trois états, résolution des pilotes multiples et détection des courts-circuits.
- Simulateur événementiel à délais inertiels, boucles autorisées, horloges, sondes, détection d'oscillation.
- Sémantique de référence exécutable, et preuve par exploration exhaustive que le simulateur optimisé la respecte pour tout délai jusqu'à 8.
- Théorème de sûreté du `X`, démontré dans `docs/semantique.md`.
- Composants hiérarchiques avec bus, sauvegarde JSON, mise à plat, chemins hiérarchiques.
- Table de vérité exhaustive, équivalence de deux composants, chemin critique.
- Bibliothèque de référence en NAND : portes, additionneur 8 bits, verrous, bascule D, registre, compteur.
- Comparaison avec Icarus Verilog sur des circuits aléatoires.
- Outil en ligne de commande `ce`.
