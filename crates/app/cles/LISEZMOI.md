# Clés de vérification des mises à jour

Les versions publiées sont signées avec une clé privée Ed25519 (`zipsign`) qui ne vit que dans le
secret GitHub Actions `ZIPSIGN_CLE_PRIVEE` et dans une sauvegarde hors ligne. L'application ne
contient que les clés publiques, ici :

- `verification-1.pub` : la clé en service, 32 octets bruts (format de `zipsign gen-key`) ;
- `verification-2.pub` : emplacement libre, pour changer de clé un jour sans casser les versions
  déjà installées (on ajoute la nouvelle clé ici dans une version, puis on signe avec elle).

Un fichier vide est un emplacement libre. Tant qu'aucune clé n'est présente, l'application refuse
toute mise à jour (sans clé, rien ne serait vérifié) et le workflow de publication refuse de publier.
