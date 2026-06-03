# Arborescence - Reflet

> Carte de l'arbre **reel** du depot (verifie fichier par fichier le 2026-06-03).
> Reflet = app de bureau de synchronisation **miroir unidirectionnelle** (source -> destination rendue identique).
> Stack : Tauri 2 (backend Rust) + SvelteKit/TypeScript (frontend, adapter-static). Version 1.0.0, identifiant `com.reflet.desktop`.
> Une ligne de role par fichier/dossier important. Les dossiers generes (`node_modules/`, `build/`, `.svelte-kit/`, `src-tauri/target/`, `src-tauri/gen/`) sont volontairement non detailles.

## Vue d'ensemble

```
reflet/
├── .ia/                  # Docs methodo (voir section dediee)
├── src/                  # Frontend SvelteKit (UI)
│   ├── lib/              # Etat, pont IPC, types, composants
│   ├── routes/           # Pages (dashboard, reglages, journal)
│   └── styles/           # CSS (tokens + global)
├── src-tauri/            # Backend Rust (Tauri) + config app
│   └── src/              # Modules Rust (coeur de l'app)
├── static/               # Assets statiques copies tels quels
├── build/                # (genere) sortie front, sert frontendDist
├── .svelte-kit/          # (genere) artefacts SvelteKit
├── node_modules/         # (genere) dependances npm
├── package.json          # Dependances npm + scripts (dev/build)
├── svelte.config.js      # Config SvelteKit (adapter-static)
├── tsconfig.json         # Config TypeScript
├── README.md             # Presentation publique du projet
└── PLAN.md               # Cahier des charges / plan d'origine
```

> A confirmer : la presence d'un `vite.config.ts` n'a pas ete trouvee (Glob negatif). La config Vite est probablement integree a `svelte.config.js` ou implicite via le plugin Tauri. **A verifier par l'utilisateur** si une config Vite explicite est attendue.

## Sens de dependance (rappel)

UI Svelte (`src/`) -> commandes Tauri (`src-tauri/src/commands.rs`, frontiere IPC) -> coeur sync (`sync.rs`, `scheduler.rs`). Jamais l'inverse. App 100 % locale, aucun appel reseau.

---

## src-tauri/src/ — Backend Rust (coeur)

| Fichier | Role |
|---------|------|
| `main.rs` | Point d'entree binaire ; appelle `reflet_lib::run()`. Masque la console en release (`windows_subsystem`). |
| `lib.rs` | Setup Tauri : enregistre les plugins (single-instance, dialog, notification, autostart), initialise logging/config/state, demarre le scheduler et le tray, applique l'autostart, declare l'`invoke_handler` (liste des commandes IPC), gere `ExitRequested` (empeche la sortie sauf vrai quit). |
| `commands.rs` | **Frontiere IPC** : toutes les commandes `#[tauri::command]` exposees au front (get/update settings, CRUD paires, sync_now/sync_all, dry_run, logs, show/hide/quit). Persiste via `config`, notifie le front par l'evenement `state:changed`. Contient aussi `apply_autostart`, `trigger_all`, l'envoi de requetes au worker, et le DTO `NewPair`. |
| `sync.rs` | **Moteur miroir** : `walk_tree` (parcours des deux arbres, ignore reparse points/symlinks et patterns glob), `build_plan` (diff source/destination -> creations dossiers, copies, ecrasements, suppressions + calcul du seuil anti-wipe), `execute_plan` (execution + emission de progression), `dry_run`, copie atomique (`copy_file_atomic` via fichier temporaire + rename, chemins verbatim `\\?\` sous Windows), suppression securisee (`safe_delete` corbeille via crate `trash`), detection de changement (taille/mtime/BLAKE3), `run_sync` (orchestration + notifications). Verifie l'imbrication source/destination (`paths_overlap`). |
| `scheduler.rs` | **Planificateur** : un worker async unique (canal `tokio` borne a 16). Boucle `select!` entre un timer calcule (`compute_sleep`, intervalle par paire ou global, borne 3..1800 s) et les requetes manuelles. Anti-overlap via `sync_busy` + execution sequentielle ; chaque paire reelle tourne dans `spawn_blocking`. Constantes : `MIN_INTERVAL=5`. |
| `config.rs` | **Persistance JSON** : structs `Settings`, `SyncPair`, `LastRun`, `Config` (serde, `camelCase`). `load`/`save` vers `settings.json` dans `app_config_dir` (= `%APPDATA%/com.reflet.desktop/settings.json`). Valeurs par defaut (intervalle 900 s, corbeille, seuil suppression 50 %, patterns d'exclusion de base). |
| `state.rs` | **Etat partage** (`AppState`) protege par `Mutex`/`AtomicBool` : config, logs (`VecDeque` plafonne a `MAX_LOGS=3000`), statuts par paire, drapeaux `scheduler_running`/`really_quit`/`sync_busy`, sender vers le worker, horodatage `last_started`. Types `LogEntry` et `SyncRequest` (Pair/All). |
| `tray.rs` | **Icone systeme** : construit le menu tray (Afficher / Synchroniser tout / Quitter), clic gauche -> affiche la fenetre, gere les evenements de menu. |
| `lifecycle.rs` | **Cycle de vie fenetre** : intercepte `CloseRequested` -> empeche la fermeture et masque la fenetre (fermeture = reduction au tray), sauf si `really_quit`. |
| `logging.rs` | **Journalisation fichier** : initialise `tracing` vers un fichier journalier `reflet.log` dans `app_log_dir` (non-blocking, sans ANSI). |

## src-tauri/ — Config et ressources backend

| Element | Role |
|---------|------|
| `tauri.conf.json` | Config Tauri : productName `Reflet`, version 1.0.0, identifiant `com.reflet.desktop`, fenetre `main` 960x680, `frontendDist: ../build`, commandes dev/build npm, bundle (nsis/deb/rpm) + icones. CSP a `null` (**a confirmer** si durcissement souhaite). |
| `Cargo.toml` | Manifeste Rust : lib `reflet_lib`, dependances (tauri + plugins, moteur sync : `walkdir`/`filetime`/`trash`/`globset`/`blake3`, logging `tracing`, `tokio`, `uuid`, `chrono`, `dunce`). |
| `Cargo.lock` | Versions verrouillees des crates. |
| `build.rs` | Script de build Tauri (`tauri-build`). |
| `capabilities/default.json` | Permissions/ACL Tauri 2 accordees a la fenetre (frontiere de securite IPC). |
| `icons/` | Icones de l'app (toutes plateformes : .ico/.icns/.png, ios/, android/). |
| `gen/` | (genere) schemas ACL Tauri. |
| `target/` | (genere) sortie de compilation Rust. |
| `.gitignore` | Exclusions git cote backend. |

---

## src/lib/ — Frontend : etat, pont IPC, utilitaires

| Fichier | Role |
|---------|------|
| `store.svelte.ts` | **Etat reactif global** (runes Svelte 5 `$state`) : settings, paires, scheduler, progression, toasts. `refresh()` recharge l'etat via IPC, `mergePairs` fusionne sans casser les references (diff superficiel), `initListeners` abonne aux evenements Tauri (`state:changed`, `sync:busy/progress/started/finished`, `app:error`) et declenche toasts/refresh. Singleton exporte `store`. |
| `ipc.ts` | **Pont IPC** : objet `api` qui mappe chaque commande Rust via `invoke` (typage TS), plus `pickFolder` (dialog dossier OS), reexport de `listen`. Seul point qui parle au backend. |
| `confirm.svelte.ts` | **Controleur de confirmation in-app** : `confirmCtl.ask(message, opts) -> Promise<boolean>` ; alimente la modale `ConfirmModal`. Remplace le dialogue de confirmation natif du systeme. |
| `types.ts` | **Types TypeScript** miroirs des structs Rust en `camelCase` : `SyncPair`, `NewPair`, `Settings`, `LastRun`, `AppStateDto`, `SyncPlan`, `PlanItem`, `LogEntry`. |
| `format.ts` | Formatage FR : `formatBytes` (o/Ko/Mo...), `formatDate` (locale fr-FR), `formatInterval` (s/min/h/j). |

### src/lib/components/ — Composants Svelte

| Fichier | Role |
|---------|------|
| `PairCard.svelte` | Carte d'une paire de synchro (nom, source/dest, statut, derniere/prochaine passe) + actions (editer, dry-run, supprimer, activer). |
| `PairEditModal.svelte` | Modale d'ajout/edition d'une paire (chemins, intervalle propre, notifications, patterns d'exclusion). |
| `DryRunModal.svelte` | Modale d'apercu (dry-run) : liste copies/ecrasements/suppressions, volume total, alerte seuil de securite avant execution. |
| `ConfirmModal.svelte` | Modale de confirmation in-app (rendue une fois dans le layout), pilotee par `confirm.svelte.ts` ; Echap = annuler, Entree = confirmer, bouton danger. Remplace les popups systeme. |
| `ExclusionAddModal.svelte` | Popup d'ajout/modification d'une exclusion globale : modes Dossier (parcourir/nom, recursif), Fichiers (selection OS multiple/nom), Manuel (glob libre), avec apercu du motif. Utilisee par l'onglet `/exclusions`. |
| `StatusBadge.svelte` | Badge de statut d'une paire (À jour / Synchro… / Erreur / Désactivé) avec pastille de couleur. |
| `Switch.svelte` | Interrupteur on/off reutilisable (bindable). |
| `ProgressBar.svelte` | Barre de progression (deterministe `done/total` ou indeterminee). |
| `VirtualList.svelte` | Liste virtualisee generique (perf sur grandes listes, ex. journal) via `Snippet` de ligne. |
| `IntervalPicker.svelte` | Selecteur d'intervalle (valeur + unite s/min/h/j), option "valeur par defaut". |
| `Select.svelte` | Menu deroulant stylise reutilisable (bindable). |

## src/routes/ — Pages (SvelteKit)

| Fichier | Role |
|---------|------|
| `+layout.svelte` | Coquille de l'app : barre laterale de navigation (Tableau de bord / Reglages / Journal), zone de toasts, init du store et des listeners au montage, raccourcis F11/Echap (plein ecran), bouton Quitter. |
| `+layout.ts` | `export const ssr = false` : rendu cote client uniquement (app desktop, pas de SSR). |
| `+page.svelte` | **Tableau de bord** : liste des paires (`PairCard`), ouverture des modales d'edition et de dry-run, confirmation de suppression. |
| `settings/+page.svelte` | **Reglages** : intervalle global, comportement suppression (corbeille/permanent), verification par contenu (off/blake3), patterns d'exclusion, autostart, demarrage minimise, notifications. Copie locale editable puis sauvegarde. |
| `logs/+page.svelte` | **Journal** : affichage des logs (liste virtualisee), filtre par niveau, rafraichissement sur `sync:finished`, effacement. |

## src/styles/

| Fichier | Role |
|---------|------|
| `tokens.css` | Variables de design (couleurs, espacements, rayons, ombres, easing). |
| `global.css` | Styles globaux et classes utilitaires (boutons, etc.). |

---

## static/ — Assets statiques

`favicon.png`, `svelte.svg`, `vite.svg`, `tauri.svg` : copies telles quelles dans la sortie. **A confirmer** : ces SVG par defaut du template sont-ils encore utilises ou a nettoyer.

---

## .ia/ — Documentation methodo

> Dossier de **documentation de methode / pilotage projet** (non livre, hors code applicatif). Sert de memoire de travail et de cartographie.

| Fichier | Role |
|---------|------|
| `arborescence.md` | **Ce document** : cartographie de l'arbre reel du depot, un role par fichier. |
| `plan.md` | Plan consolide (but, fonctionnalites) a partir de `PLAN.md` et `README.md`. |
| `architecture.md` | Note d'architecture (presente dans le dossier ; contenu **a confirmer/relire** par l'utilisateur). |
| `performance.md` | Note sur la performance (presente dans le dossier ; contenu **a confirmer/relire** par l'utilisateur). |

---

## Notes a confirmer par l'utilisateur

- **Pas de `vite.config.ts` trouve** : la config Vite semble portee par `svelte.config.js` / le plugin Tauri. A confirmer si une config Vite dediee est attendue.
- **Textes en francais codes en dur** : aucune i18n. Si une internationalisation est prevue, l'arbre `src/lib` evoluera (ex. dossier `i18n/` ou `locales/`).
- **CSP `null`** dans `tauri.conf.json` : a confirmer s'il faut durcir la politique de securite.
- **Assets de template** (`svelte.svg`, `vite.svg`, `tauri.svg`) dans `static/` : a confirmer s'ils servent encore.
- **`.ia/architecture.md` et `.ia/performance.md`** : contenus non relus ici, a valider par l'utilisateur.
