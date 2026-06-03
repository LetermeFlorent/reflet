# Suivi des modifications

Journal des modifications de Reflet (app desktop de synchronisation miroir unidirectionnelle, Tauri 2 + SvelteKit). On conserve au maximum les 9 derniers jours en detail, plus un seul bloc « Resume » qui condense tout ce qui precede.

## Resume (avant les 9 derniers jours)

_(vide pour l'instant)_

## 2026-06-03

Modifications de la journee (quoi + pourquoi) :

- **Bouton « Synchroniser » a barre de progression integree (effet « passe plat »)** — sur chaque carte de paire, le bouton se remplit horizontalement pendant la synchro et affiche le pourcentage (ou un etat indetermine anime tant que le total n'est pas connu). _Pourquoi : donner un retour visuel de l'avancement directement sur l'action, sans bloc de progression separe._

- **ETA « temps restant » pendant la synchro** — la ligne de statut de la carte affiche une estimation du temps restant (« ~X min Y s restant »), calculee cote frontend a partir du debit observe (fichiers traites / temps ecoule). _Pourquoi : informer l'utilisateur de la duree restante sur les gros transferts._

- **Compte a rebours avant la prochaine synchro auto** — la carte affiche « prochaine dans … » en se basant sur la valeur `nextRunSec` desormais renvoyee par le backend (calculee dans `commands.rs` a partir de l'intervalle effectif de la paire et du dernier lancement, uniquement si la paire est activee et que l'ordonnanceur tourne). _Pourquoi : rendre visible le rythme de l'automatisation, qui etait jusque-la invisible cote UI._

- **Mise a jour ciblee du store (merge par id)** — le store frontend fusionne desormais l'etat entrant avec l'existant paire par paire (cle = `id`), en ne remplacant que les champs reellement modifies et en preservant l'ordre/les references stables quand rien ne change. _Pourquoi : eviter de recreer toute la liste a chaque rafraichissement, ce qui reduit les re-rendus inutiles et evite que les animations/etats locaux des cartes ne sautent._

- **Suppression de tous les commentaires du code (demande explicite)** — les commentaires ont ete retires des modules Rust (`src-tauri/src`) et du frontend TypeScript/Svelte (`src/lib`, `src/routes`). _Pourquoi : demande explicite de l'utilisateur ; correspond au commit HEAD `refactor: suppression de tous les commentaires du code + nettoyage repo`._ A confirmer par l'utilisateur : `src-tauri/Cargo.toml` contient encore des commentaires de configuration (non touches car il ne s'agit pas de « code »).

- **Uniformisation des boutons (fin de `btn-primary`)** — plus aucun composant n'applique la classe `btn-primary` ; tous les boutons utilisent le style `btn` commun (avec variantes `btn-sm`, `btn-ghost`, `btn-icon`, `btn-danger`). _Pourquoi : harmoniser l'apparence et eviter un bouton « accent » isole._ A confirmer par l'utilisateur : la definition CSS `.btn-primary` subsiste dans `src/styles/global.css` (classe orpheline, plus referencee) — a supprimer si on veut nettoyer completement.

- **Reglage global « cartes compactes » (actif par defaut)** — nouvelle option dans Reglages (section « Affichage ») : en mode compact, la ligne du bas des cartes n'affiche que le statut court (« A jour » / « Jamais synchronise » / « Synchro avec erreurs ») + le temps avant la prochaine synchro, au lieu du detail copies/maj/suppr. Persistee cote backend (`compact_cards`, valeur par defaut `true`). _Pourquoi : alleger l'affichage de la liste de paires par defaut._

- **Bump de version 1.0.0** — version alignee a 1.0.0 dans `package.json`, `src-tauri/Cargo.toml` et `src-tauri/tauri.conf.json`. _Pourquoi : marquer la premiere version stable._

- **Repo GitHub public + branches `main`/`dev` + release v1.0.0 avec installeur** — depot public `github.com/LetermeFlorent/reflet` (remote `origin` confirme localement), branches `main` et `dev` presentes. Installeur NSIS genere localement : `src-tauri/target/release/bundle/nsis/Reflet_1.0.0_x64-setup.exe`. _Pourquoi : publier la v1.0.0 distribuable._ A confirmer par l'utilisateur : aucun tag `v1.0.0` n'existe en local et la release GitHub n'a pas pu etre verifiee ici (CLI `gh` indisponible, pas d'acces reseau) — verifier que le tag/la release et l'asset installeur sont bien publies sur GitHub.

- **Creation du dossier `.ia` (documentation methodo)** — ajout d'un dossier `.ia` destine a la documentation de methode/suivi (ce fichier `suivi.md` y est cree). _Pourquoi : centraliser le suivi des modifications et la doc methodo du projet._

- **Popups de confirmation in-app (remplacement des dialogues systeme natifs)** — les confirmations « Supprimer la paire » et « Vider le journal » n'utilisent plus le dialogue natif du systeme (`@tauri-apps/plugin-dialog`) mais une vraie modale in-app au style de l'app. Nouveau module dedie `src/lib/confirm.svelte.ts` (controleur `confirmCtl.ask(...) -> Promise<boolean>`) et composant `src/lib/components/ConfirmModal.svelte` (rendu une fois dans le layout, supporte Echap = annuler / Entree = confirmer, bouton danger). `api.confirm` et l'import `confirm` du plugin dialog ont ete retires de `ipc.ts` (code mort). _Pourquoi : demande utilisateur d'avoir une vraie popup integree plutot que le popup systeme. Le selecteur de dossier (`open`) reste natif car il necessite l'acces fichiers de l'OS._

- **Exclusion par defaut des dossiers/fichiers caches (`**/.*`)** — ajout du motif `**/.*` aux exclusions globales par defaut (code) et dans la config utilisateur ; remplace `.git/**` et `.DS_Store` devenus redondants. _Pourquoi : ne jamais miroiter les dossiers caches (.git, .vscode, .svelte-kit...)._

- **Onglet « Exclusions » dedie (tableau + popup d'ajout)** — nouvelle page/onglet `/exclusions` : les exclusions globales sont gerees dans un tableau (Type infere / Motif / actions Modifier-Supprimer) avec un unique bouton « + Ajouter » ouvrant une popup multi-mode (`ExclusionAddModal.svelte`) : Dossier (parcourir un dossier ou saisir un nom, recursif ou non), Fichiers (selection multiple via l'OS ou par nom), Manuel (glob libre), avec apercu du motif genere. Nouveau `api.pickFiles`. La section Exclusions a ete retiree de Reglages. Suppression via la modale de confirmation in-app. _Pourquoi : demande utilisateur d'une gestion pro/complete des exclusions plutot qu'un simple textarea._

- **Bump de version 1.1.0** — `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`. _Pourquoi : nouvelle version livrable (popups in-app, mode compact, onglet Exclusions)._
