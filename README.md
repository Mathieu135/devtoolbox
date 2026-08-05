# devtoolbox — POC interactif Rust

Ce dépôt teste une hypothèse précise : un outil peut-il réduire le temps entre
« mon projet a besoin d'un service » et « mon application l'utilise vraiment » ?

Le POC fournit deux services :

- **RabbitMQ** : un bouton du site publie un message JSON ; l'interface ne
  l'affiche qu'après sa consommation et son ACK par un worker ;
- **Redis** : un second bouton exécute un vrai `INCR`, conservé dans un volume
  Docker entre les redémarrages.

Chaque carte contient aussi un bouton de check explicite : RabbitMQ attend une
publication, consommation et ACK de sa sonde ; Redis vérifie `PING` puis un
aller-retour `SET/GET` temporaire, supprimé immédiatement après lecture.
L'historique RabbitMQ affiché par le site est une vue locale des messages déjà
consommés et ACKés : une queue vide après leur affichage est donc le résultat
attendu.

## Installer le binaire

Les releases fournissent des binaires macOS et Linux pour architectures x86_64
et ARM64. Le script vérifie le checksum SHA-256 avant installation :

```console
curl -fsSLO https://raw.githubusercontent.com/Mathieu135/devtoolbox/main/scripts/install.sh
less install.sh
bash install.sh
```

Le binaire est installé dans `~/.local/bin` par défaut. Le dossier peut être
modifié avec `DEVTOOLBOX_INSTALL_DIR` et une version précise peut être choisie
avec `DEVTOOLBOX_VERSION=v0.1.0`. Ajouter le dossier au `PATH` si nécessaire :

```console
export PATH="$HOME/.local/bin:$PATH"
```

Les artefacts publics disposent également d'une attestation GitHub vérifiable :

```console
gh attestation verify toolbox-macos-aarch64.tar.gz --repo Mathieu135/devtoolbox
```

Pour travailler directement depuis les sources, remplacer simplement
`toolbox` par `./target/debug/toolbox` après `cargo build`.

## Démo en quatre commandes

```console
toolbox --project demo/rust-app add rabbitmq
toolbox --project demo/rust-app add redis
toolbox --project demo/rust-app up
cargo run --manifest-path demo/rust-app/Cargo.toml
```

Ouvrir ensuite [http://127.0.0.1:3000](http://127.0.0.1:3000).

Au démarrage, le site publie déjà un premier message automatique dans RabbitMQ.
Les deux actions visibles permettent ensuite de vérifier le chemin complet :

```text
navigateur → API Axum → RabbitMQ → consumer → ACK → interface
navigateur → API Axum → Redis INCR → volume persistant → interface
```

Pour diagnostiquer ou arrêter les deux services installés :

```console
toolbox --project demo/rust-app doctor
toolbox --project demo/rust-app down
```

Un service peut également être ciblé explicitement avec `up redis`,
`doctor rabbitmq` ou `down redis`. `down` conserve toujours les volumes.

## Ce que `add` génère

`toolbox add rabbitmq` ajoute :

- `.toolbox/rabbitmq/compose.yaml`, avec une image versionnée et un healthcheck ;
- `.toolbox/rabbitmq/check/`, un client Rust isolé qui effectue un aller-retour
  AMQP réel pour `doctor` ;
- `RABBITMQ_URL` dans un bloc géré de `.env.example`.

`toolbox add redis` ajoute :

- `.toolbox/redis/compose.yaml`, avec persistance AOF et healthcheck ;
- `REDIS_URL` dans un bloc géré distinct de `.env.example`.

Les fichiers générés ne sont pas remplacés silencieusement après une
modification locale : une nouvelle exécution demande `--force`. Les blocs
ajoutés aux fichiers existants sont idempotents. Tous les ports Docker sont
liés à `127.0.0.1` uniquement.

## Providers distants

L'application et `doctor` lisent `RABBITMQ_URL` et `REDIS_URL`. Aucun changement
de code n'est nécessaire pour tester des services distants :

```console
RABBITMQ_URL='amqps://user:secret@broker.example/vhost' \
  toolbox --project demo/rust-app doctor rabbitmq

REDIS_URL='rediss://user:secret@cache.example/' \
  toolbox --project demo/rust-app doctor redis
```

Les diagnostics n'affichent jamais les identifiants. `up` reste réservé aux
providers locaux et refuse une URL distante.

## Choix d'architecture du POC

- La CLI est écrite en Rust et distribuable comme un binaire autonome.
- La présence de `.toolbox/<service>/compose.yaml` indique qu'un service est
  installé. Il n'existe toujours pas de manifeste générique : deux adapters
  concrets permettent maintenant de découvrir les champs réellement communs
  avant de figer ce contrat.
- Les images sont figées sur les tags `rabbitmq:4.3.3-management` et
  `redis:8.8.1-alpine`, ainsi que sur leur digest multi-architecture. Les
  clients sont figés sur `lapin = 4.10.0` et `redis = 1.4.1`.
- Le site de démonstration est une vraie application Axum. Il montre comment un
  projet consomme les variables générées, sans prétendre modifier
  automatiquement le code métier d'un projet arbitraire.
- Les noms de projets Compose contiennent une empreinte du chemin canonique afin
  d'isoler deux dépôts portant le même nom. Redis utilise en plus un projet
  Compose séparé pour ne pas perturber RabbitMQ.

## Limites encore assumées

- Docker avec le plugin Compose et Cargo doivent être installés.
- Seuls les projets Rust contenant un `Cargo.toml` sont détectés.
- Le premier lancement télécharge les images et compile les dépendances de la
  démo. Le binaire `toolbox` lui-même est précompilé pour les plateformes
  supportées.
- Il n'y a pas encore de catalogue distant, gestion de secrets, migrations ou
  abstraction cloud commune.
- Les providers locaux utilisent des identifiants de développement et ne sont
  pas destinés à la production.

## Validation locale

```console
cargo fmt --all -- --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --manifest-path demo/rust-app/Cargo.toml -- --check
cargo check --manifest-path demo/rust-app/Cargo.toml
```

Le protocole de bêta mesurable est décrit dans
[`docs/beta-test.md`](docs/beta-test.md).
