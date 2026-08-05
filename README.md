# devtoolbox — POC Rust/RabbitMQ

Ce dépôt teste une hypothèse précise : un outil peut-il réduire le temps entre
« mon projet a besoin de RabbitMQ » et « mon projet a réellement publié puis
consommé un message » ?

Il ne cherche pas encore à fournir un catalogue universel.

## Parcours démontré

```console
cargo build
./target/debug/toolbox --project demo/rust-app add rabbitmq --dry-run
./target/debug/toolbox --project demo/rust-app add rabbitmq
./target/debug/toolbox --project demo/rust-app up
./target/debug/toolbox --project demo/rust-app doctor
```

`add` génère, après prévisualisation possible :

- `.toolbox/rabbitmq/compose.yaml` avec une image RabbitMQ versionnée et un
  healthcheck ;
- `.toolbox/rabbitmq/check/`, un petit client Rust isolé qui effectue un vrai
  aller-retour AMQP ;
- un bloc géré dans `.env.example` ;
- un bloc géré dans `.gitignore` pour le cache de compilation.

Les fichiers générés ne sont pas remplacés silencieusement après modification :
une nouvelle exécution demande `--force`. Les blocs ajoutés aux fichiers du
projet sont idempotents.

## Provider distant

Le client et `doctor` lisent toujours `RABBITMQ_URL`. Aucun changement de code
n'est donc nécessaire pour diagnostiquer un broker distant :

```console
RABBITMQ_URL='amqps://user:secret@broker.example/vhost' \
  ./target/debug/toolbox --project demo/rust-app doctor
```

L'URL complète et son mot de passe ne sont jamais affichés par le diagnostic.
Ce POC ne stocke pas les identifiants distants.
`up` reste volontairement réservé au provider local et refuse une URL distante.

## Décisions volontairement limitées

- La CLI est écrite en Rust et distribuable comme un binaire autonome.
- Il n'existe pas encore de manifeste générique `toolbox.toml` : avec un seul
  service, ce serait une abstraction non utilisée. Une structure déclarative
  typée ne sera justifiée qu'avec un deuxième service ou provider.
- Le vérificateur est un crate Rust séparé. Il prouve la connectivité réelle
  depuis le projet sans modifier arbitrairement son `Cargo.toml` ou son code
  métier. En contrepartie, il ne constitue pas encore l'intégration métier de
  l'application.
- L'image est figée sur `rabbitmq:4.3.3-management` et le client sur
  `lapin = 4.10.0` afin d'éviter les changements implicites.

## Limites du POC

- Docker avec le plugin Compose et Cargo doivent être installés.
- Seuls les projets Rust contenant un `Cargo.toml` sont détectés.
- Il n'y a ni commande `down`, ni gestion de secrets, ni migration de données.
- Le premier `up` télécharge l'image Docker et les crates du client de test.
- Les ports locaux `5672` et `15672` peuvent entrer en conflit avec des services
  existants. Compose accepte `TOOLBOX_RABBITMQ_PORT` et
  `TOOLBOX_RABBITMQ_MANAGEMENT_PORT`, mais il faut alors ajuster
  `RABBITMQ_URL` en conséquence.
- Le nom du projet Compose contient une empreinte stable du chemin canonique afin
  d'isoler deux dépôts portant le même nom.

## Validation locale

```console
cargo fmt --all -- --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```
