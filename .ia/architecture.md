# Architecture — Reflet

> App desktop de **synchronisation miroir unidirectionnelle** (source → destination rendue identique).
> Stack : **Tauri 2** (backend Rust) + **SvelteKit/TypeScript** (`adapter-static`).
> Version 1.0.0. 100 % locale, aucun appel réseau. Textes FR codés en dur (pas d'i18n).
>
> Ce document est basé sur la lecture réelle des sources (`lib.rs`, `commands.rs`, `scheduler.rs`,
> `config.rs`, `state.rs`). Les points à valider sont signalés par **[À CONFIRMER]**.

## 1. Vue d'ensemble en couches

Reflet est structuré en quatre couches, du haut (interface) vers le bas (persistance). Chaque
couche ne dépend **que** de la couche immédiatement inférieure. Le flux d'une action utilisateur
descend toujours dans le même sens.

```
┌─────────────────────────────────────────────────────────────┐
│  1. PRÉSENTATION  — SvelteKit / TypeScript  (src/)            │
│     src/routes/*  (dashboard, settings, logs)                 │
│     src/lib/components/*.svelte                               │
│     src/lib/store.svelte.ts  (état réactif)                   │
│     src/lib/ipc.ts           (pont invoke / listen)           │
│     src/lib/types.ts, format.ts                               │
└───────────────────────────┬───────────────────────────────────┘
                            │  invoke("...")  /  events Tauri
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  2. IPC / FRONTIÈRE  — commands.rs  (#[tauri::command])       │
│     get_app_state, get_settings, update_settings,             │
│     set_scheduler_running, add_pair, update_pair,             │
│     delete_pair, set_pair_enabled, sync_now, sync_all,        │
│     dry_run, get_logs, clear_logs, show/hide_window, quit_app │
│     Enregistrées dans lib.rs (invoke_handler)                 │
└───────────────────────────┬───────────────────────────────────┘
                            │  appels de fonctions Rust
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  3. MÉTIER  — moteur de sync + ordonnanceur                   │
│     sync.rs       (walk_tree, build_plan, execute_plan,       │
│                    blake3, run_sync, dry_run, paths_overlap)  │
│     scheduler.rs  (1 worker unique, timer par paire,          │
│                    anti-overlap via sync_busy)                │
│     state.rs      (AppState partagé : Mutex + AtomicBool)     │
└───────────────────────────┬───────────────────────────────────┘
                            │  lecture/écriture de la config
                            ▼
┌─────────────────────────────────────────────────────────────┐
│  4. DONNÉES  — config.rs                                      │
│     Config { settings: Settings, pairs: Vec<SyncPair> }       │
│     load() / save() ←→ settings.json (serde)                  │
│     %APPDATA%/com.reflet.desktop/settings.json                │
└─────────────────────────────────────────────────────────────┘
```

## 2. Détail des couches

### 2.1 Présentation (SvelteKit, `src/`)
- Trois routes (dashboard `+page`, settings, logs) et des composants `src/lib/components/*.svelte`.
- `store.svelte.ts` : état réactif (runes Svelte 5).
- `ipc.ts` : unique point d'entrée vers le backend (`invoke`) et d'écoute des événements émis
  par le backend (`state:changed`, `sync:busy`).
- `types.ts` / `format.ts` : types partagés et formatage d'affichage.
- La présentation ne connaît **rien** du cœur de sync : elle ne parle qu'aux commandes Tauri.

### 2.2 IPC / frontière (`commands.rs`)
- Seul point de contact entre le monde JS et le monde Rust. La liste des commandes exposées est
  enregistrée dans `lib.rs` via `invoke_handler(tauri::generate_handler![...])` (15 commandes,
  voir `lib.rs:83-100`).
- Chaque commande reçoit `State<AppState>` et/ou `AppHandle`, lit/modifie l'état, persiste si
  besoin (`persist`), puis notifie le frontend (`notify_state_changed` → événement `state:changed`).
- Les commandes de déclenchement (`sync_now`, `sync_all`) **n'exécutent pas** la sync directement :
  elles envoient un `SyncRequest` au worker via un canal (`send_request` → `state.sync_tx`).
  Cela maintient le worker unique et l'anti-overlap.
- `dry_run` est `async` et délègue le travail bloquant à `spawn_blocking(sync::dry_run)`.
- Validations métier appelées ici : `sync::paths_overlap` (interdit source/destination imbriquées)
  dans `add_pair` et `update_pair`.

### 2.3 Métier (`sync.rs`, `scheduler.rs`, `state.rs`)
- **`sync.rs`** : moteur miroir (parcours d'arbre, construction de plan, exécution, hachage blake3,
  `run_sync`, `dry_run`, `paths_overlap`). C'est le cœur ; il ne dépend ni de l'IPC ni de l'UI.
- **`scheduler.rs`** : un **seul** worker lancé au démarrage (`scheduler::start` dans `lib.rs:49`).
  - Boucle `tokio::select!` entre un timer (`compute_sleep` → prochaine échéance par paire) et la
    réception de `SyncRequest` (`All` / `Pair(id)`).
  - Anti-overlap : `sync_busy` (AtomicBool) positionné autour des exécutions ; l'exécution
    effective est confiée à `spawn_blocking(crate::sync::run_sync)` (`scheduler.rs:149-152`).
  - Intervalle effectif par paire : `interval_sec_override` (≥ `MIN_INTERVAL` = 5 s) sinon
    `settings.interval_sec` (borné par `MIN_INTERVAL`).
- **`state.rs`** : `AppState` partagé (`Mutex<Config>`, `Mutex<VecDeque<LogEntry>>` plafonné à
  `MAX_LOGS` = 3000, `statuses`, `last_started`, `scheduler_running`, `really_quit`, `sync_busy`,
  `sync_tx`). C'est la structure d'état que les couches 2 et 3 se partagent.

### 2.4 Données (`config.rs`)
- `Config { settings: Settings, pairs: Vec<SyncPair> }`, sérialisé en JSON via serde
  (`#[serde(rename_all = "camelCase")]`, donc clés camelCase côté fichier et côté JS).
- `load()` / `save()` lisent/écrivent `settings.json` ; `config_path()` résout
  `app_config_dir()` → `%APPDATA%/com.reflet.desktop/settings.json`.
- Tolérance aux erreurs : si `settings.json` est illisible, on retombe sur `Config::default()`
  (log `warn`), sans planter.
- Valeurs de sécurité par défaut confirmées dans `Settings::default()` :
  `delete_behavior = "trash"` (corbeille), `confirm_deletes_with_dry_run = true`,
  `delete_safety_threshold_pct = 50` (seuil anti-wipe), `interval_sec = 900`.

## 3. Sens de dépendance unidirectionnel et absence de cycle

La règle d'architecture est stricte et vérifiée par les fichiers lus :

```
UI (Svelte)  →  commands.rs (IPC)  →  sync.rs / scheduler.rs (métier)  →  config.rs (données)
```

- **Le flux ne remonte jamais.** Le métier ne connaît ni l'UI ni les commandes : `sync.rs` et
  `scheduler.rs` n'importent que `crate::config`, `crate::state` (et `tauri` pour l'`AppHandle`).
  `commands.rs` importe `crate::config`, `crate::state`, `crate::sync` — jamais l'inverse
  (`config.rs` n'importe pas `commands`, `sync.rs` n'importe pas `commands`).
- **Communication montante par événements, pas par appels.** Quand le backend doit informer
  l'UI (fin de sync, changement d'état), il **émet un événement** (`state:changed`, `sync:busy`)
  que `ipc.ts` écoute. Il n'appelle jamais le frontend : pas d'inversion de dépendance.
- **Communication asynchrone vers le worker par canal.** Les commandes ne traversent pas le métier
  en synchrone pour lancer une sync : elles déposent un `SyncRequest` dans un canal
  (`state.sync_tx`) consommé par l'unique worker. Cela découple IPC et exécution et garantit
  l'absence d'exécutions concurrentes.
- **Absence de cycle** : le graphe de modules est un DAG. `lib.rs` joue le rôle de composition root
  (il déclare les modules, monte les plugins, crée `AppState`, démarre le scheduler, branche le
  handler IPC) mais n'introduit pas de dépendance circulaire entre couches.

`main.rs` se contente d'appeler `reflet_lib::run()` (point d'entrée binaire) **[À CONFIRMER : non
relu ici, mais conforme au schéma Tauri 2 lib.rs + main.rs]**.

## 4. Démarrage (composition root — `lib.rs`)

Ordre confirmé dans `run()` (`lib.rs:42-81`) :
1. `logging::init` (tracing).
2. `config::load` → `AppState::new(cfg)` puis `app.manage(...)`.
3. `scheduler::start(handle)` → le `Sender<SyncRequest>` est stocké dans `state.sync_tx`.
4. `tray::build` (icône systray).
5. `apply_autostart` selon `settings.autostart`.
6. Si lancé avec `--autostart` **et** `start_minimized`, la fenêtre est cachée.

Plugins montés (desktop) : `single-instance` (refocalise la fenêtre existante), `dialog`,
`notification`, `autostart` (LaunchAgent). Fermeture : interceptée via `RunEvent::ExitRequested`
+ `lifecycle::on_window_event` → l'app reste dans le tray tant que `really_quit` est faux ;
`quit_app` met `really_quit = true` puis `app.exit(0)`.

## 5. Contraintes qui orientent l'architecture

### 5.1 Build Tauri 2
- Séparation imposée backend Rust (`src-tauri/`) / frontend statique (`src/` via `adapter-static`).
  L'IPC `#[tauri::command]` est la **seule** frontière JS↔Rust ; d'où la couche 2 dédiée.
- Le binaire passe par `lib.rs` + `main.rs` (entrée mobile compatible via
  `#[cfg_attr(mobile, tauri::mobile_entry_point)]`), même si la cible est desktop Windows.
- Build **portable** : `tauri build --no-bundle` (noté dans la mémoire projet) **[À CONFIRMER :
  non vérifié dans `tauri.conf.json` ici]**.

### 5.2 Proxy TLS (environnement de build)
Contraintes d'environnement (mémoire projet, **[À CONFIRMER]** car non vérifiables dans les sources Rust) :
- Cargo : `CARGO_HTTP_CHECK_REVOKE=false` (par session) pour récupérer les crates derrière le proxy.
- npm : `NODE_OPTIONS=--use-system-ca` pour faire confiance au certificat du proxy.
- Outillage : sous ce poste, **Bash peut être cassé → utiliser PowerShell** pour les commandes de build.

Ces contraintes ne modifient pas le découpage en couches mais conditionnent la reproductibilité du
build ; elles justifient un cœur 100 % local et sans dépendance réseau à l'exécution (aucun appel
réseau dans `commands.rs` / `sync.rs` / `scheduler.rs`).

### 5.3 Sécurités de suppression (orientent le métier)
Confirmées dans `config.rs` (`Settings`) : corbeille par défaut (`trash`), seuil anti-wipe en %
(`delete_safety_threshold_pct`), dry-run de confirmation (`confirm_deletes_with_dry_run`), copie
atomique **[À CONFIRMER : mécanisme situé dans `sync.rs`, non relu en détail ici]**. Ces garde-fous
vivent dans la couche métier et sont pilotés par la couche données (réglages persistés).

## 6. Points à confirmer (récapitulatif)
- Contenu exact de `sync.rs` (`walk_tree`, `build_plan`, `execute_plan`, copie atomique, blake3) —
  non relu en détail pour ce document.
- `main.rs`, `tray.rs`, `lifecycle.rs`, `logging.rs` — comportement supposé conforme au schéma Tauri 2.
- Détails frontend (`store.svelte.ts`, `ipc.ts`) — structure supposée d'après la convention projet.
- Réglages de build (`--no-bundle`) et variables proxy TLS — issus de la mémoire projet, non
  revérifiés dans `tauri.conf.json` / scripts.
