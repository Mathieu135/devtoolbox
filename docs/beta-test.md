# Protocole bêta v0.1.0

## Question testée

Un développeur connaissant son langage mais peu à l'aise avec Docker peut-il
installer deux dépendances locales, les démarrer, les utiliser et expliquer ce
qu'il s'est passé sans intervention d'une équipe plateforme ?

Ce test cherche des faits observables, pas une validation de politesse. Il doit
être réalisé par 3 à 5 personnes qui n'ont pas participé au développement.

## Préconditions

- macOS ou Linux, Docker avec Compose et Rust stable déjà installés ;
- un navigateur et un terminal ;
- aucun accompagnement pendant le scénario, sauf blocage explicite ;
- ne jamais publier de secret, d'URL privée ou de dump d'environnement dans le
  retour GitHub.

## Scénario chronométré

Le testeur démarre le chronomètre avant l'installation et suit uniquement le
README de la release.

1. Installer `toolbox` et afficher `toolbox --help`.
2. Cloner le dépôt de démonstration.
3. Ajouter RabbitMQ puis Redis au projet `demo/rust-app`.
4. Démarrer les services puis le site.
5. Dans le navigateur, envoyer un message, incrémenter Redis et utiliser les
   deux boutons de diagnostic.
6. Exécuter `toolbox --project demo/rust-app doctor`.
7. Expliquer pourquoi l'historique peut afficher des messages alors que la
   queue RabbitMQ contient zéro message prêt.
8. Arrêter les services avec `toolbox --project demo/rust-app down`.

Le testeur note l'heure du premier service utilisable, l'heure de fin, chaque
erreur bloquante et chaque aide humaine demandée. Une recherche documentaire
autonome n'est pas comptée comme aide, mais son URL et son motif sont notés.

## Mesures et seuils de décision

| Mesure | Signal favorable pour poursuivre |
|---|---:|
| scénario terminé | au moins 4 testeurs sur 5, ou 3 sur 3 |
| temps jusqu'au premier service utilisable | médiane ≤ 10 minutes |
| temps jusqu'au scénario complet | médiane ≤ 20 minutes |
| aides humaines synchrones | médiane ≤ 1 |
| erreurs bloquantes non résolues | 0 pour au moins 80 % des testeurs |
| explication correcte du message ACKé | au moins 80 % des testeurs |

Une explication est correcte si elle distingue la queue RabbitMQ de
l'historique en mémoire de l'application et indique que le consumer a ACKé le
message. Les temps de téléchargement sont conservés dans la mesure mais marqués
séparément pour éviter de confondre lenteur réseau et complexité du produit.

## Décision après 3 à 5 retours

- **Poursuivre** si tous les seuils sont atteints.
- **Itérer** si l'échec se concentre sur une étape réparable et reproductible.
- **Revoir l'hypothèse** si les testeurs réussissent techniquement mais ne
  perçoivent aucun gain par rapport à leur méthode actuelle.
- **Arrêter ce parcours** si moins de la moitié termine sans accompagnement.

Les retours sont déposés avec le formulaire « Retour bêta v0.1.0 ». La synthèse
doit présenter les valeurs individuelles anonymisées, la médiane, les échecs et
les citations paraphrasées ; une moyenne seule masquerait trop facilement un
parcours très difficile pour une minorité.
