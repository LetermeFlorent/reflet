# Plan - reflet

> Document consolide a partir de `PLAN.md` et `README.md` du depot.
> Reflet est une application de bureau de **synchronisation miroir unidirectionnelle** de dossiers (Windows + Linux), construite avec **Tauri 2 + SvelteKit + Rust**.
> Date : 2026-06-03. Version actuelle : 1.0.0.

## But

Maintenir un dossier *destination* strictement identique a un dossier *source* (miroir unidirectionnel) : la source fait autorite, la destination est rendue identique.

- Copie des nouveautes, ecrasement des fichiers differents, suppression de tout ce qui n'existe plus dans la source.
- L'app ne touche **jamais** a la source (lecture seule cote source).
- Vit dans la barre systeme (tray), se lance au demarrage de l'OS (optionnel), synchronise en arriere-plan sur un minuteur configurable, plus un declenchement manuel.
- Securise les suppressions destructrices (corbeille, seuil anti-wipe, dry-run, journalisation).
- Application 100 % locale : aucun appel reseau.

## Fonctionnalites

- **Paires de synchronisation** source -> destination, activables individuellement.
- **Synchro automatique** par planificateur : intervalle global ou propre a chaque paire.
- **Synchro manuelle** (« Synchroniser ») avec barre de progression, temps restant estime (ETA) et compte a rebours avant la prochaine synchro auto.
- **Apercu (dry-run)** : voir ce qui serait copie / mis a jour / supprime avant d'executer.
- **Securite suppressions** : seuil configurable (defaut 50 %) qui met en pause une passe si elle supprimerait une trop grande part de la destination.
- **Corbeille ou suppression definitive** au choix (corbeille par defaut).
- **Verification par contenu** optionnelle (BLAKE3), en complement de taille + date de modification.
- **Filtres d'exclusion** (glob) globaux et par paire, appliques aux deux arbres.
- **Notifications** systeme et in-app (opt-in).
- **Journal** des operations (audit).
- **Demarrage automatique** et **demarrage minimise** optionnels.
- **Fermeture = reduction dans le tray** (l'app continue de tourner) ; « Quitter » pour fermer reellement.
- **Menu tray** en parite avec l'UI : afficher/masquer, synchroniser tout, pause/reprise du planificateur, ouvrir journaux, quitter.

## Contraintes techniques

### Plateformes

- **Windows** : WebView2 (preinstalle Windows 11 ; bootstrapper sinon), Build Tools C++ (MSVC). Bundle NSIS (`.exe`), MSI optionnel. Binaire portable via `--no-bundle`.
- **Linux (Debian/Ubuntu)** : `libwebkit2gtk-4.1-0`, `libgtk-3-0`, `libayatana-appindicator3-1` (paquets `-dev` pour le build). Bundle `.deb` / `.rpm`. Base recommandee : Ubuntu 22.04 / Debian 12 (WebKitGTK 4.1).
- Pas de cross-build inter-familles : chaque OS construit ses propres artefacts.

### Tauri 2 / Stack

- **Tauri 2** (feature `tray-icon`) : coeur applicatif, fenetre, tray, IPC, runtime tokio.
- **Frontend** : Svelte 5 + SvelteKit en mode SPA/SSG (`adapter-static`, SSR desactive). Style : CSS ecrit a la main + design tokens (pas de Tailwind).
- **Plugins** (ordre impose au demarrage) : `single-instance` (en premier), puis `autostart`, `store`/persistance, `notification`, `dialog`.
- **Source de verite cote Rust** : config persistee dans `%APPDATA%\com.reflet.desktop\settings.json` (Windows) / `~/.config/com.reflet.desktop/` (Linux), via serde JSON. Jamais localStorage.
- **Frontiere IPC** : UI Svelte -> commandes `#[tauri::command]` (`commands.rs`) -> coeur sync. Sens de dependance strict, jamais l'inverse.
  - `invoke()` pour actions/CRUD ; `Channel<SyncEvent>` pour la progression fichier par fichier ; `emit`/`listen` pour les broadcasts globaux (statut, tray).
- **Moteur sync** (`sync.rs`) : `walk_tree`, `build_plan`, `execute_plan`, hash blake3 ; IO bloquante isolee (spawn_blocking).
- **Scheduler** (`scheduler.rs`) : 1 worker unique, timer par paire, anti-overlap par Mutex (passe chevauchante skippee + loggee).
- **Modules backend** : `main.rs`, `lib.rs` (setup, plugins), `commands.rs`, `sync.rs`, `scheduler.rs`, `config.rs`, `state.rs` (etat partage Mutex), `tray.rs`, `lifecycle.rs`, `logging.rs` (tracing).

### Proxy TLS (entreprise)

Derriere un proxy avec inspection TLS, si le build echoue sur la verification des certificats :

- Cargo : variable de session `CARGO_HTTP_CHECK_REVOKE=false`.
- npm : variable de session `NODE_OPTIONS=--use-system-ca`.
- Definir ces variables pour la session courante avant `npm run tauri dev` / `build`.

## Etapes / Jalons

> Repris des milestones M0–M9 du `PLAN.md`. (A confirmer : l'etat reel d'avancement vs version 1.0.0 deja publiee.)

- **M0 — Scaffold** : projet Tauri (Svelte 5 + SvelteKit, TS), `adapter-static` + `ssr=false`, fenetre lancee, icones generees.
- **M1 — Modele & config** : structs `Settings` / `SyncPair` (serde), persistance JSON, commandes settings + CRUD paires.
- **M2 — Moteur de sync (coeur)** : walk + diff + detection (taille + mtime) + copie atomique + stamp mtime + suppression corbeille + globset ; tests unitaires sur arbres temporaires.
- **M3 — Dry-run** : plan retourne sans ecriture, commande dediee.
- **M4 — Scheduler + anti-overlap** : boucle interval + canal « Sync now » + Mutex unique + evenements. **MVP utilisable** : synchro reelle sur minuteur + manuel, corbeille + dry-run (UI minimale).
- **M5 — UI complete** : Dashboard (liste des paires), modal Ajouter/Editer (pickers + validations), Reglages, modal Dry-run (suppressions en rouge), Journal ; tokens + theme + transitions.
- **M6 — Runtime tray-resident** : tray + menu (parite), close-to-tray + really_quit, single-instance, autostart + demarrage minimise, notifications.
- **M7 — Securite avancee** : seuil de suppression %, verrous/permissions Windows + retry, prefixe `\\?\` (chemins longs), re-stat mid-sync, nettoyage `.synctmp`, verification blake3 optionnelle, rotation des logs.
- **M8 — Packaging & CI** : bundle targets ; builds locaux `.deb` / `.rpm` + NSIS + portable `.exe` ; workflow GitHub Actions (matrice ubuntu-22.04 + windows-latest) attachant les artefacts ; signature Authenticode (option).
- **M9 — Finitions** : QA cross-plateforme, MSI optionnel, doc utilisateur, polish UI WebKitGTK.

### Risques & mitigations (rappel)

Le miroir unidirectionnel **supprime** dans la destination : risque de perte de donnees. Mitigations integrees (non optionnelles) : seuil anti-wipe, corbeille par defaut, refus destination imbriquee dans la source, ignore applique aux deux arbres, stamp mtime + tolerance + court-circuit taille, copie atomique + nettoyage `.synctmp`, log+skip si echec corbeille (jamais de hard-delete silencieux), re-stat mid-sync, Mutex anti-overlap, gestion par-fichier des verrous/permissions Windows, prefixe `\\?\`, journal `tracing` roulant, symlinks non suivis (skip par defaut).

## Questions ouvertes (a confirmer par l'utilisateur)

- **Etat d'avancement reel** : la version 1.0.0 est annoncee comme publiee, mais le `PLAN.md` liste tous les jalons M0–M9 en non coches. Quels jalons sont effectivement termines a date ?
- **Nom du module de config** : `PLAN.md` decrit un dossier `config/` (mod.rs + model.rs) et le plugin `tauri-plugin-store`, alors que le README et le brief decrivent un fichier unique `config.rs` avec persistance serde JSON directe. Quelle est l'implementation reelle a documenter ?
- **Decoupage du moteur sync** : `PLAN.md` decrit un dossier `sync/` multi-fichiers (walk/diff/detect/copy/delete/engine/plan), tandis que le README et le brief decrivent un seul `sync.rs` (`walk_tree`, `build_plan`, `execute_plan`). Confirmer la structure reelle.
- **Nom du store frontend** : `PLAN.md` mentionne `stores.svelte.ts`, le README et le brief mentionnent `store.svelte.ts`. Confirmer le nom exact.
- **Persistance** : `tauri-plugin-store` (PLAN.md) ou serde JSON maison via `config.rs` (README/brief) ? A trancher.
- **i18n** : les textes sont actuellement en francais code en dur, sans i18n. Une internationalisation est-elle prevue / souhaitee ?
- **Notifications** : marquees « opt-in » dans le README mais « permission demandee au demarrage » dans `PLAN.md`. Confirmer le comportement attendu.
- **MSI et signature Authenticode** : sont-ils dans le perimetre de la 1.0 ou reportes ?
- **Declencheur temps-reel** (`notify`/file watcher) : reste-t-il un objectif post-MVP ou abandonne ?
- **Chemin du projet** : `PLAN.md` reference `C:\Users\ipmss\Projects\reflet`, le depot reel est dans `C:\Users\ipmss\Downloads\reflet`. Sans incidence fonctionnelle, a clarifier si besoin.
