"""Faux serveur de versions, avec la même API que GitHub, pour essayer la mise à jour automatique
sur une vraie machine (CI Windows) sans rien publier.

    python outils/faux-serveur-de-versions.py <archive.zip> <version> <fichier-adresse>

Sert la dernière version `v<version>` du dépôt `essai/versions`, avec l'archive donnée, sur un
port libre de 127.0.0.1, et écrit l'adresse (http://127.0.0.1:PORT) dans <fichier-adresse> une
fois prêt. Tourne jusqu'à ce qu'on l'arrête.
"""

import http.server
import json
import os
import sys

archive, version, fichier_adresse = sys.argv[1:4]
nom = os.path.basename(archive)


class Gestion(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        chemin = self.path.split("?", 1)[0]
        base = f"http://127.0.0.1:{self.server.server_port}"
        page = {
            "tag_name": f"v{version}",
            "name": f"v{version}",
            "created_at": "2026-11-13T00:00:00Z",
            "body": f"Version d'essai {version}",
            "assets": [
                {
                    "name": nom,
                    "url": f"{base}/fichiers/{nom}",
                    "browser_download_url": f"{base}/fichiers/{nom}",
                }
            ],
        }
        if chemin == "/repos/essai/versions/releases/latest":
            corps = json.dumps(page).encode()
        elif chemin == f"/repos/essai/versions/releases/tags/v{version}":
            corps = json.dumps(page).encode()
        elif chemin == "/repos/essai/versions/releases":
            corps = json.dumps([page]).encode()
        elif chemin == f"/fichiers/{nom}":
            with open(archive, "rb") as f:
                corps = f.read()
        else:
            self.send_response(404)
            self.send_header("Content-Length", "0")
            self.end_headers()
            return
        self.send_response(200)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Content-Length", str(len(corps)))
        self.end_headers()
        self.wfile.write(corps)

    def log_message(self, *args):
        pass


serveur = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Gestion)
with open(fichier_adresse + ".tmp", "w") as f:
    f.write(f"http://127.0.0.1:{serveur.server_port}")
os.replace(fichier_adresse + ".tmp", fichier_adresse)
serveur.serve_forever()
