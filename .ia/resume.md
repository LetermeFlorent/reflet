# Reflet — Résumé fonctionnel

Reflet est une application de bureau (Windows / Linux) qui maintient un **miroir** d'un dossier vers un autre. La synchronisation est **unidirectionnelle** : la **destination** est rendue **identique à la source**. La source fait autorité ; tout ce qui existe en trop côté destination est supprimé pour refléter exactement la source.

L'application fonctionne entièrement en local : aucune donnée n'est envoyée sur un réseau.

---

## Paires source → destination

Une « paire » décrit un dossier source et un dossier destination à garder en miroir. L'utilisateur peut créer plusieurs paires, chacune gérée indépendamment.

Pour chaque paire on définit :
- un **nom** (si laissé vide, le chemin source est utilisé comme nom) ;
- le **dossier source** (l'autorité) ;
- le **dossier destination** (qui sera reflété) ;
- un **intervalle de synchro automatique propre** ou, à défaut, l'intervalle global par défaut ;
- des **exclusions** propres à la paire (motifs glob, un par ligne) ;
- l'**activation/désactivation** de la paire ;
- deux interrupteurs de notification pour cette paire (notification système « PC » et notification dans l'app).

Règles fonctionnelles :
- Source et destination ne peuvent pas être **imbriquées** (l'une contenant l'autre, ou identiques) : c'est interdit, signalé dans le formulaire et refusé à l'enregistrement.
- Source et destination sont **obligatoires**.
- On peut choisir les dossiers via un bouton **Parcourir** (sélecteur de dossier) ou saisir le chemin à la main.
- **Supprimer une paire** retire seulement la configuration de l'app : les dossiers source et destination **ne sont pas touchés** (confirmation demandée avant suppression).

Le tableau de bord liste les paires sous forme de cartes et affiche, par paire : le nom, un badge de **statut**, l'intervalle de synchro auto, les chemins source → destination, et un résumé de la dernière exécution (ou « Jamais synchronisé »).

---

## Synchronisation miroir (automatique et manuelle)

Ce que fait concrètement une synchronisation, pour rendre la destination identique à la source :
- **Copier** les fichiers présents dans la source mais absents de la destination (« nouveaux »).
- **Mettre à jour** les fichiers présents des deux côtés mais qui diffèrent (« modifiés »).
- **Supprimer** dans la destination les fichiers/dossiers qui n'existent plus dans la source.
- **Créer** les dossiers manquants côté destination.

Détection des fichiers modifiés (par défaut) : comparaison par **taille** puis par **date de modification**, avec une **tolérance d'horodatage** réglable (utile pour les systèmes de fichiers ou partages réseau qui arrondissent les dates). Une vérification **par empreinte du contenu** peut être activée en plus (plus sûr, plus lent).

Les éléments correspondant aux **exclusions** (globales + propres à la paire) sont ignorés. Les liens symboliques et points de jonction ne sont pas suivis.

Deux façons de déclencher une synchro :

**Automatique (planificateur)**
- Chaque paire active est synchronisée à son **intervalle** (propre ou par défaut).
- Le planificateur peut être **mis en pause / repris** globalement (bouton « Mettre en pause » / « Reprendre » sur le tableau de bord, ou réglage « Planificateur actif »). En pause, seule la synchro manuelle reste possible.
- Une seule synchro s'exécute à la fois (pas de chevauchement) ; les paires dues sont traitées l'une après l'autre.
- La carte d'une paire affiche un compte à rebours « prochaine dans … » avant sa prochaine synchro auto.

**Manuelle**
- **Synchroniser** une paire précise (bouton sur sa carte).
- **Tout synchroniser** : lance toutes les paires actives (bouton du tableau de bord, désactivé s'il n'y a aucune paire ou si une synchro est déjà en cours).

Pendant une synchro, la carte affiche une **progression** (pourcentage, ou animation « indéterminée » au démarrage) et une **estimation du temps restant**. La synchro peut aussi être déclenchée depuis l'icône de la barre système (« Synchroniser tout »).

> À confirmer : la **mise à jour des dates de modification** sur les fichiers copiés et la **copie atomique** (passage par un fichier temporaire puis renommage, avec une nouvelle tentative en cas d'échec) sont des comportements internes de fiabilité, sans option visible pour l'utilisateur. Ils sont décrits ici pour information mais ne relèvent pas d'un réglage exposé.

---

## Aperçu (dry-run)

Avant d'appliquer quoi que ce soit, le bouton **Aperçu** d'une paire calcule et affiche, sans rien modifier, ce que la synchro ferait :
- nombre de fichiers **nouveaux** (à copier),
- nombre de fichiers **modifiés**,
- nombre d'éléments **à supprimer** dans la destination,
- volume total **à copier**.

L'aperçu liste aussi le détail des chemins concernés (nouveaux, modifiés avec la raison, et à supprimer), tronqué à 200 éléments par catégorie avec un compteur « …et N de plus ».

Depuis l'aperçu, on peut :
- **Appliquer maintenant** (lance la synchro réelle de cette paire) — désactivé s'il n'y a rien à faire ;
- ou **Fermer** sans rien changer.

Si la destination est déjà identique à la source, l'aperçu indique « Rien à faire ».

> À confirmer : un réglage interne « confirmer les suppressions par un aperçu » existe dans la configuration, mais il **n'est pas exposé dans l'interface des Réglages** et ne semble pas relié à un comportement obligatoire actuel. À clarifier (intention : forcer un passage par l'aperçu avant toute suppression ?).

---

## Sécurité des suppressions

Plusieurs garde-fous protègent la destination contre des suppressions massives accidentelles :

- **Méthode de suppression** :
  - **Corbeille** (par défaut, recommandé) : les fichiers supprimés côté destination sont récupérables ;
  - **Permanent** : suppression irréversible (un avertissement s'affiche quand ce mode est choisi).

- **Seuil de sécurité anti-wipe** (en %, 50 % par défaut) : si la synchro devait supprimer **plus que ce pourcentage** de la destination — cas typique d'une source vide ou d'un disque/partage non monté — les **suppressions sont retenues** (non exécutées), un avertissement est journalisé et une alerte s'affiche. Les copies et mises à jour, elles, ont quand même lieu. Régler le seuil à **100 % désactive** cette protection.

- **Source introuvable** : si le dossier source n'existe pas, la synchro est **annulée** plutôt que de vider la destination.

- **Tolérance d'horodatage** : évite des écrasements inutiles (et donc du bruit) à cause de petits écarts de date.

L'aperçu signale aussi, le cas échéant, qu'un seuil de sécurité serait dépassé.

---

## Notifications

Deux canaux de notification, chacun à deux niveaux (un interrupteur **maître** global + un interrupteur **par paire**) :

- **Notifications système (PC)** : notifications Windows/Linux à la fin d'une synchro (résumé : copiés / mis à jour / supprimés, et erreurs éventuelles) ou en cas d'erreur.
- **Notifications dans l'app (toasts)** : bulles affichées dans la fenêtre (succès, info, erreurs) ; les messages d'erreur restent affichés plus longtemps.

Une notification ne s'affiche que si **le type est activé à la fois au niveau global (Réglages) et au niveau de la paire**.

> À confirmer : par défaut, les deux interrupteurs maîtres (PC et app) sont **désactivés** côté réglages, alors que les interrupteurs par paire sont activés par défaut. Donc, à l'installation, **aucune notification ne s'affiche tant que l'utilisateur n'active pas le canal correspondant dans les Réglages**. À confirmer si c'est l'intention.

---

## Journal

Un onglet **Journal** retrace les opérations effectuées, ligne par ligne, avec : horodatage, type d'action (copie, mise à jour, suppression, ignoré, erreur, info), paire concernée, message et chemin.

Fonctions :
- **Filtrer** par niveau : Tout / Infos / Avertissements / Erreurs.
- **Rafraîchir** (le journal se met aussi à jour automatiquement à la fin d'une synchro).
- **Vider** le journal (confirmation demandée).

Les actions sont mises en évidence par couleur (copie en vert, mise à jour en bleu, suppression et erreur en rouge, « ignoré » en orange).

> À confirmer : le journal affiché est l'historique **en mémoire** de la session courante ; sa persistance entre deux lancements de l'app n'est pas garantie côté interface (à clarifier avec le comportement attendu).

---

## Réglages et affichage

Onglet **Réglages**, regroupé en sections ; les modifications sont prises en compte via le bouton **Enregistrer**.

**Synchronisation**
- **Intervalle par défaut** (utilisé par les paires sans intervalle propre).
- **Planificateur actif** (active/désactive la synchro automatique au minuteur).
- **Vérification du contenu** : « Taille + date (rapide) » ou « + empreinte » (plus sûr, plus lent).

**Sécurité des suppressions**
- Méthode de suppression (Corbeille / Permanent), seuil anti-wipe, tolérance d'horodatage (voir section Sécurité).

**Notifications**
- Interrupteurs maîtres « Notifications système (PC) » et « Notifications dans l'app » (voir section Notifications).

**Démarrage**
- **Lancer au démarrage de l'ordinateur** : Reflet démarre avec la session et reste dans la barre système.
- **Démarrer minimisé** : au lancement automatique, masquer la fenêtre (icône de la barre système seulement).

**Affichage**
- **Cartes compactes** : la ligne du bas de chaque carte n'affiche qu'un statut court (« À jour » / « Jamais synchronisé » / « Synchro avec erreurs ») + le temps avant la prochaine synchro, au lieu du détail copiés / mis à jour / supprimés.

**Exclusions globales**
- Liste de motifs glob (un par ligne) appliqués à **toutes** les paires, en plus des exclusions propres à chaque paire. Comparés au chemin relatif. Exemples fournis dans l'interface : `**/*.tmp` (un type de fichier), `**/node_modules/**` (un dossier entier), `**/secret.txt` (un fichier précis où qu'il soit), `cache/**` (un dossier à la racine de la paire).
- Exclusions par défaut fournies : fichiers temporaires `**/*.tmp`, fichiers de verrouillage Office `**/~$*`, `**/Thumbs.db`, `**/.DS_Store`, et le dossier `**/.git/**`.

---

## Comportement de la fenêtre et de la barre système

- **Fermer la fenêtre ne quitte pas l'application** : la fenêtre est seulement masquée et Reflet continue de tourner dans la **barre système** (icône de notification).
- L'icône de la barre système propose un menu : **Afficher Reflet**, **Synchroniser tout**, **Quitter**. Un clic gauche sur l'icône rouvre la fenêtre.
- **Quitter** réellement l'application se fait via ce menu.

---

*Document de synthèse fonctionnelle (ce que l'application fait pour l'utilisateur), établi à partir du code source réel le 2026-06-03. Les points marqués « À confirmer » nécessitent une validation de l'intention produit.*
