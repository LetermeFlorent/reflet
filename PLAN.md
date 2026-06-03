# Plan projet — Reflet : synchronisation miroir unidirectionnelle (Tauri v2)

> App desktop Windows + Linux. Nom de travail : **Reflet** (renommable). Dossier projet : `C:\Users\ipmss\Projects\reflet`.

## 1. Vue d'ensemble

Application desktop multiplateforme (Windows + Linux) construite en **Rust + Tauri v2** qui synchronise une ou plusieurs paires (dossier source → dossier destination) selon un **miroir unidirectionnel** : la source fait autorité, la destination est rendue strictement identique (copie des nouveautés, écrasement des fichiers différents, suppression de tout ce qui n'existe plus dans la source). L'app vit dans la barre système (tray), se lance au démarrage de l'OS, synchronise en arrière-plan sur un minuteur configurable (plus un « Sync now » manuel), et sécurise les suppressions destructrices via corbeille, mode dry-run/preview et journalisation détaillée.

## 2. Stack technique

Frontend : **Svelte 5 + SvelteKit** en mode SPA/SSG (adapter-static, SSR désactivé) — choix recommandé par Tauri, runtime le plus léger et démarrage le plus rapide dans WebView2/WebKitGTK. Style : **CSS écrit à la main + design tokens** (pas de Tailwind à cette échelle).

| Nom | Rôle |
|---|---|
| `tauri` v2 (feature `tray-icon`) | Cœur applicatif, fenêtre, tray, IPC, runtime |
| Svelte 5 + SvelteKit (`@sveltejs/adapter-static`) | Frontend SPA, runes `$state/$derived/$effect` |
| `tauri-plugin-single-instance` | Instance unique (enregistré **en premier**) |
| `tauri-plugin-autostart` (2.5.x) | Lancement au démarrage OS (HKCU Run / XDG autostart) |
| `tauri-plugin-store` | Persistance des réglages (JSON dans le dossier config OS) |
| `tauri-plugin-notification` | Notifications natives (fin/erreur de sync) |
| `tauri-plugin-dialog` | Sélecteur de dossier natif (`open({directory:true})`) |
| `walkdir` 2.5 | Parcours des arbres source et destination |
| `filetime` 0.2 | Lecture/écriture mtime portable (détection + stamp post-copie) |
| `trash` 5 | Suppression sécurisée → Corbeille (Windows) / Trash XDG (Linux) |
| `globset` 0.4 | Patterns d'exclusion (ignore) appliqués aux deux arbres |
| `blake3` 1 | Vérification de contenu optionnelle (mode « verify by content ») |
| `tracing` + `tracing-subscriber` + `tracing-appender` | Journal d'audit roulant des opérations |
| `serde` / `serde_json` | (Dé)sérialisation config & payloads IPC |
| `dunce` | Normalisation chemins Windows + préfixe long `\\?\` |
| `tokio` (fourni par Tauri) | `interval`, `spawn_blocking`, `Mutex`, `mpsc` |

Crates optionnels (post-MVP, derrière feature flag) : `jwalk` (parcours parallèle si profilage le justifie), `xxhash-rust` (xxh3, vérif rapide non-crypto), `notify` 8 + `notify-debouncer-full` 0.7 (déclencheur quasi temps-réel).

## 3. Architecture

### Backend Rust (`src-tauri/`)

```
main.rs / lib.rs        → Builder Tauri : ordre plugins, setup(), run()
config/                 → modèle Settings + SyncPair, chargement/sauvegarde via store
sync/
  ├─ walk.rs            → énumération (walkdir) → Map<RelPath, Entry> filtrée (globset + symlinks)
  ├─ diff.rs            → calcul du plan (to_create_dir / to_copy / to_overwrite / to_delete)
  ├─ detect.rs          → détection de changement (taille + mtime ± tolérance, hash optionnel)
  ├─ copy.rs            → copie atomique (temp + rename même volume) + stamp mtime
  ├─ delete.rs          → suppression corbeille/permanente, deepest-first, seuil de sécurité
  ├─ engine.rs          → orchestration 3 phases, exécution dry-run vs réel, émission événements
  └─ plan.rs            → structs SyncPlan / PlanItem / SyncResult / SyncEvent
scheduler.rs            → boucle tokio interval + canal mpsc « Sync now », Mutex anti-overlap
tray.rs                 → TrayIconBuilder, menu, on_menu_event, on_tray_icon_event
lifecycle.rs            → close-to-tray (prevent_close), really_quit AtomicBool, ExitRequested
commands.rs             → #[tauri::command] (get_app_state, CRUD pairs, sync_now, dry_run, settings, logs…)
logging.rs              → init tracing + appender roulant dans app_config_dir
state.rs                → AppState partagé (Arc) : pairs, statut, sync en cours, progression
```

**Modules clés :**
- **Sync engine** : transforme deux arbres en un `SyncPlan` (3 phases) ; le même code exécute le dry-run (`execute=false`) et la sync réelle (`execute=true`).
- **Scheduler** : une boucle `tauri::async_runtime::spawn` avec `tokio::time::interval` ; « Sync now » envoie un message sur un `mpsc` vers le même worker ; un unique `tokio::sync::Mutex` garantit qu'aucune passe ne se chevauche (si le minuteur tire pendant une sync, on **skip** et on logge).
- **Config store** : `tauri-plugin-store` (`settings.json`) ; source de vérité côté Rust (jamais localStorage).
- **Tray / lifecycle / autostart / single-instance** : voir §7.

### Frontend (`src/`)

SvelteKit SPA, runes pour l'état local. La source de vérité reste en Rust : au montage / à l'affichage de la fenêtre, le frontend appelle `get_app_state()` pour hydrater l'état complet, **puis** s'abonne aux deltas.

### Flux de données (3 voies IPC)

1. **`invoke()` (requête/réponse)** — toutes les actions/CRUD : `get_app_state`, `list_pairs`, `add_pair`, `update_pair`, `delete_pair`, `set_pair_enabled`, `sync_now`, `sync_all`, `dry_run`, `get_settings`, `update_settings`, `get_logs`, `clear_logs`, `set_scheduler_running`, `show_window`/`hide_window`/`quit_app`.
2. **`tauri::ipc::Channel<SyncEvent>` (flux haute fréquence)** — progression par fichier d'**une** sync/dry-run active (ordonnée, volumineuse). Anti-pattern à éviter : émettre des centaines d'événements globaux.
3. **`app.emit` → `listen` (broadcasts globaux basse fréquence)** — `scheduler:tick`, `sync:started`, `sync:finished`, `pair:status-changed`, `app:error`. Le tray et toute fenêtre s'y abonnent ; le frontend conserve les handles `unlisten()` pour le nettoyage.

```
[Svelte UI] --invoke--> [commands.rs] --> [engine/scheduler/state]
[Svelte UI] <--Channel<SyncEvent>-- [engine]   (progression fichier par fichier)
[Svelte UI] <--emit/listen-- [scheduler/engine] (statut global, tray)
[Tray menu] --mêmes commandes Rust--> (parité UI/tray)
```

## 4. Modèle de config

Persisté par `tauri-plugin-store` dans `settings.json` (Windows : `%APPDATA%\<bundle-id>\` ; Linux : `~/.config/<bundle-id>/`). Source de vérité = Rust ; `save()` explicite après chaque modification.

```ts
SyncPair {
  id: string                      // uuid
  name: string
  source: string                  // chemin absolu
  destination: string             // chemin absolu
  enabled: boolean
  intervalSecOverride?: number    // sinon hérite de l'intervalle global
  ignorePatterns?: string[]       // globs propres à la paire (fusionnés avec globaux)
  verifyByContent?: 'off'|'blake3' // override, sinon réglage global
  symlinkPolicy?: 'skip'|'recreate'
  lastRun?: { at: string, status: 'ok'|'error', copied: number, updated: number, deleted: number, errors: number }
  status: 'idle'|'syncing'|'error'|'disabled'   // runtime (non persisté)
}

Settings {
  intervalSec: number
  deleteBehavior: 'trash'|'permanent'    // défaut 'trash'
  autostart: boolean
  startMinimized: boolean
  theme: 'system'|'light'|'dark'
  confirmDeletesWithDryRun: boolean
  ignorePatterns: string[]               // globaux
  verifyByContent: 'off'|'blake3'        // défaut 'off'
  symlinkPolicy: 'skip'|'recreate'       // défaut 'skip'
  mtimeToleranceSec: number              // défaut 2 (FAT/SMB)
  deleteSafetyThresholdPct: number       // défaut 50 : pause si > N% de la dest serait supprimée
  logLevel: 'info'|'warn'|'error'
  logRetentionDays: number
}
```

Exemple `settings.json` :

```json
{
  "settings": {
    "intervalSec": 900,
    "deleteBehavior": "trash",
    "autostart": true,
    "startMinimized": true,
    "theme": "system",
    "confirmDeletesWithDryRun": true,
    "ignorePatterns": ["*.tmp", "~$*", "node_modules/", "Thumbs.db", ".DS_Store"],
    "verifyByContent": "off",
    "symlinkPolicy": "skip",
    "mtimeToleranceSec": 2,
    "deleteSafetyThresholdPct": 50,
    "logLevel": "info",
    "logRetentionDays": 30
  },
  "pairs": [
    {
      "id": "b1f2c3d4-0001",
      "name": "Documents → NAS",
      "source": "C:\\Users\\moi\\Documents",
      "destination": "Z:\\backup\\Documents",
      "enabled": true,
      "intervalSecOverride": 600,
      "ignorePatterns": ["*.log"],
      "verifyByContent": "off",
      "symlinkPolicy": "skip"
    },
    {
      "id": "b1f2c3d4-0002",
      "name": "Photos → Disque externe",
      "source": "/home/moi/Photos",
      "destination": "/mnt/usb/Photos",
      "enabled": true
    }
  ]
}
```

## 5. Moteur de sync (algorithme miroir)

Tout tourne dans `tokio::task::spawn_blocking` (IO bloquante : ne jamais staller le runtime / la boucle tray).

```
fn run_sync(pair, settings, execute: bool, on_event: Channel<SyncEvent>) -> Result<SyncPlan|SyncResult>:

  # 0. GARDE ANTI-OVERLAP (scheduler) : acquérir Mutex global ; si pris → SKIP + log
  #    valider que destination n'est PAS imbriquée dans source (ni l'inverse) → sinon refus
  # 0bis. Nettoyer les .synctmp-* orphelins dans la destination

  # 1. ÉNUMÉRATION (les deux arbres, walkdir, follow_links(false))
  build globset depuis settings.ignorePatterns + pair.ignorePatterns
  src_map  = walk(source)      -> Map<RelKey, Entry{is_dir,size,mtime,is_symlink}>
  dest_map = walk(destination) -> Map<RelKey, Entry{...}>
    - filtrer chaque chemin via globset AVANT insertion
    - RelKey = chemin relatif, case-folded sur FS insensible à la casse (Windows)
    - politique symlink : skip (défaut) ou recreate
    - sur Windows, chemins via helper appliquant le préfixe \\?\ (chemins longs)

  # 2. DIFF → plan en 3 phases
  to_create_dir = dirs ∈ src absents de dest                       (trier shallow→deep)
  to_copy       = fichiers ∈ src absents de dest                   (reason: 'new')
  to_overwrite  = fichiers ∈ src ∩ dest où detect_changed()        (reason: size|mtime|content)
  to_delete     = TOUT ∈ dest absent de src (fichiers ET dossiers) (trier deep→shallow)
                + conflits de type (fichier↔dossier) : supprimer dest puis recréer

  # 2bis. detect_changed():
    if src.size != dest.size: return true            # court-circuit : jamais de hash si tailles ≠
    if |src.mtime - dest.mtime| > mtimeToleranceSec: return true
    if verifyByContent == 'blake3': return blake3(src) != blake3(dest)
    return false

  # 3. SEUIL DE SÉCURITÉ
  if to_delete.len() / max(1, dest_map.len()) * 100 > deleteSafetyThresholdPct:
      émettre app:error « suppression massive » + PAUSE + exiger confirmation

  # 4. DRY-RUN
  if !execute: return SyncPlan { toCopy, toOverwrite, toDelete, totalBytes }   # rien écrit

  # 5. EXÉCUTION — ordre impératif
  on_event.send(started{...})
  # 5a. créer les dossiers manquants (parents d'abord)
  # 5b. copier + écraser :
  #     tmp = dest_dir/.synctmp-<rand> (MÊME volume) ; copy ; filetime::set_file_mtime(tmp, src.mtime) ; rename(tmp, final)
  #     re-stat source avant/après ; verrou Win (err 32) / permission (err 5) → retry court + log + continue
  #     throttle progress (~100ms / N fichiers)
  # 5c. supprimer EN DERNIER, deepest-first :
  #     trash::delete(p) si deleteBehavior=='trash' ; échec réseau → log + skip (JAMAIS hard-delete silencieux)
  on_event.send(finished{...}) ; tracing audit chaque op ; notification OS
```

**Cas limites couverts :** tolérance mtime (FAT/exFAT 2 s, SMB) ; stamp mtime post-copie obligatoire ; ignore appliqué aux **deux** arbres ; suppression deepest-first et après les copies ; clés case-folded ; corbeille faillible → log+skip ; verrous/permissions Windows par-fichier ; `MAX_PATH` → préfixe `\\?\` ; symlinks non suivis ; fichier modifié en cours de sync → re-stat + re-queue ; court-circuit taille ; rename atomique même volume + nettoyage `.synctmp` ; anti-overlap par Mutex unique.

## 6. UI / écrans

1. **Dashboard — Liste des paires** : carte par paire (source → dest, badge statut, dernier run + compteurs copié/màj/supprimé, toggle, « Sync now » + « Dry-run », éditer/supprimer). Barre de progression overlay pendant une sync.
2. **Modal Ajouter / Éditer** : pickers source & dest (`plugin-dialog`), nom, intervalle override, patterns ignore, toggle. **Validations** : refus si destination imbriquée dans source ; avertissement « le contenu de la destination sera rendu identique (suppressions possibles) ».
3. **Réglages globaux** : intervalle (presets 1/5/15/30/60 min + custom), suppression (Corbeille vs Permanent + avertissement), autostart, démarrer minimisé, thème, dry-run avant suppression, vérif contenu, symlinks, tolérance mtime, seuil sécurité %, logs.
4. **Modal Aperçu Dry-run** (sécurité) : diff groupé Va copier / Va écraser / **Va supprimer (rouge)** + compteurs & octets ; Confirmer & appliquer / Annuler.
5. **Journal / Activité** : liste virtualisée filtrable (paire, action, niveau), export/clear ; alimentée par le `Channel` + hydratée depuis Rust.

**Design « Apple-like » (tokens) :** espacement 4/8/12/16/24/32 ; rayons 6/10/14 ; ombres subtiles (`0 1px 2px /.04, 0 4px 12px /.06`) ; accent unique `#007AFF` clair / `#0A84FF` sombre ; mouvement 150–300 ms ease-out ; police `-apple-system, 'Segoe UI', system-ui, Inter` ; thème via `[data-theme]` + `prefers-color-scheme` ; beaucoup de blanc.

**Menu tray (parité)** : Afficher/Masquer, Synchroniser tout, Pause/Reprendre planificateur, Ouvrir journaux, Quitter. Icône/tooltip reflète le statut global. `show_menu_on_left_click(true)` (clic gauche peu fiable sous Linux).

## 7. Comportement runtime

- **Ordre plugins** (impératif) : `single_instance` **en premier**, puis `autostart`, `store`, `notification`, `dialog`. Tray + sync dans `.setup()`.
- **Single-instance** : callback dans l'instance primaire → `get_webview_window("main")` + `.show()` + `.unminimize()` + `.set_focus()`.
- **Autostart** : `tauri-plugin-autostart` avec `args(["--autostart"])`. Windows = HKCU Run ; Linux = `~/.config/autostart/<App>.desktop`. **Activé surtout pour builds installés** (.deb/.rpm/NSIS) ; pour portable .exe, revalider le chemin à chaque lancement.
- **Démarrer minimisé** : fenêtre `"visible": false` dans `tauri.conf.json` (ou `hide()` si `--autostart`).
- **Close-to-tray** : `WindowEvent::CloseRequested` → `window.hide()` + `api.prevent_close()`. Vrai Quit : item tray pose `AtomicBool really_quit=true` puis `app.exit(0)` ; backstop `RunEvent::ExitRequested` → `prevent_exit()` tant que `!really_quit`.
- **Minuteur** : `tauri::async_runtime::spawn` (jamais `tokio::spawn` nu) + `interval` ; « Sync now » via `mpsc`, même Mutex.
- **Notifications** : permission demandée au démarrage. Toasts Windows fiables surtout sur build NSIS installé (AppUserModelID).
- **Tray Linux** : dépendance runtime `libayatana-appindicator3` dans .deb/.rpm.

## 8. Pipeline de build / release

**Cibles bundle** (`tauri.conf.json → bundle.targets`) :
- **Linux** : `["deb","rpm"]`.
- **Windows** : `["nsis"]` (+ `["msi"]` optionnel entreprise).
- **Portable .exe** : pas de cible dédiée → binaire brut `src-tauri/target/release/<app>.exe` (zippé). Tourne standalone **si WebView2 présent** ; perd updater/toasts fiables.

**Quel OS construit quoi (pas de cross-build inter-familles) :** `.deb`/`.rpm` → Linux uniquement ; `.msi` → Windows (WiX) ; NSIS `.exe` → Windows (binaire MSVC). Base Linux = **Ubuntu 22.04 / Debian 12** (WebKitGTK 4.1).

**Deps runtime :** Windows = WebView2 (préinstallé Win10 1803+/Win11 ; sinon bootstrapper NSIS/MSI). Linux = `libwebkit2gtk-4.1-0`, `libgtk-3-0`, `libappindicator3-1`.

**Matrice GitHub Actions** (`tauri-apps/tauri-action@v0`) :
```
matrix:
  - os: ubuntu-22.04    # deb + rpm
  - os: windows-latest  # nsis (+ msi optionnel)
steps:
  - actions/checkout@v4
  - actions/setup-node@v4            # Node LTS 20+
  - dtolnay/rust-toolchain@stable    # host MSVC sur Windows
  - swatinem/rust-cache@v2
  - (Linux) apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
  - tauri-apps/tauri-action@v0       # args: --bundles deb,rpm  /  --bundles nsis
  - actions/upload-artifact          # zipper le target/release/*.exe portable
```
Déclencher sur tags `app-v*`. **Signature Authenticode Windows** à planifier tôt (SmartScreen/UAC).

**Prérequis poste de build :** Rust stable via rustup (host `x86_64-pc-windows-msvc` sur Windows, PAS GNU) ; Node LTS 20+ ; MSVC C++ Build Tools sur Windows ; deps Linux ci-dessus. Icônes via `cargo tauri icon ./app-icon.png` (PNG RGBA 1024×1024).

## 9. Arborescence projet

```
reflet/
├─ package.json                      # scripts npm, @tauri-apps/cli, deps frontend
├─ svelte.config.js                  # adapter-static, ssr=false
├─ vite.config.ts
├─ tsconfig.json
├─ app-icon.png                      # source 1024x1024 RGBA pour `tauri icon`
├─ PLAN.md                           # ce document
├─ src/                              # frontend SvelteKit (SPA)
│  ├─ app.html
│  ├─ routes/
│  │  ├─ +layout.ts                  # export const ssr=false; prerender
│  │  ├─ +layout.svelte              # shell, thème [data-theme], navigation
│  │  ├─ +page.svelte                # Dashboard (liste des paires)
│  │  ├─ settings/+page.svelte
│  │  └─ logs/+page.svelte
│  ├─ lib/
│  │  ├─ ipc.ts                      # wrappers invoke() + Channel + listen()
│  │  ├─ stores.svelte.ts            # runes : pairs, appStatus, progress, settings
│  │  ├─ types.ts                    # SyncPair, Settings, SyncPlan, SyncEvent…
│  │  └─ components/
│  │     ├─ PairCard.svelte
│  │     ├─ PairEditModal.svelte
│  │     ├─ DryRunModal.svelte
│  │     ├─ ProgressBar.svelte
│  │     └─ StatusBadge.svelte
│  └─ styles/
│     ├─ tokens.css                  # design tokens (:root + [data-theme])
│     └─ global.css
├─ src-tauri/
│  ├─ Cargo.toml
│  ├─ tauri.conf.json                # bundle.targets, nsis, deb, icon, window.visible=false
│  ├─ build.rs
│  ├─ icons/                         # généré
│  ├─ capabilities/default.json      # dialog:allow-open, store:default, autostart:*, notification:*
│  └─ src/
│     ├─ main.rs
│     ├─ lib.rs                      # Builder : plugins (single-instance 1er), setup(), run()
│     ├─ state.rs
│     ├─ config/{mod.rs, model.rs}
│     ├─ sync/{mod.rs, walk.rs, diff.rs, detect.rs, copy.rs, delete.rs, engine.rs, plan.rs}
│     ├─ scheduler.rs
│     ├─ tray.rs
│     ├─ lifecycle.rs
│     ├─ commands.rs
│     └─ logging.rs
└─ .github/workflows/release.yml     # matrice ubuntu-22.04 + windows-latest
```

## 10. Jalons (milestones)

- [ ] **M0 — Scaffold** : `npm create tauri-app@latest` (Svelte 5 + SvelteKit, TS) ; `adapter-static` + `ssr=false` ; `cargo tauri dev` lance une fenêtre. Générer icônes.
- [ ] **M1 — Modèle & config** : structs `Settings`/`SyncPair` (serde) ; `tauri-plugin-store` ; commandes settings + CRUD pairs ; persistance vérifiée.
- [ ] **M2 — Moteur de sync (cœur)** : walk + diff + detect (size+mtime) + copy atomique + stamp mtime + delete corbeille + globset ; tests unitaires sur arbres temporaires.
- [ ] **M3 — Dry-run** : `SyncPlan` retourné sans écriture ; commande `dry_run`.
- [ ] **M4 — Scheduler + anti-overlap** : boucle interval + mpsc « Sync now » + Mutex unique ; `Channel<SyncEvent>` + emit globaux. **▶ MVP UTILISABLE** : synchronise réellement sur minuteur + manuel, corbeille + dry-run (UI minimale).
- [ ] **M5 — UI complète** : Dashboard, modal Ajouter/Éditer (pickers + validations), Réglages, modal Dry-run (suppressions rouge), Journal ; tokens + thème + transitions.
- [ ] **M6 — Runtime tray-resident** : tray + menu (parité), close-to-tray + really_quit, single-instance, autostart + start-minimized, notifications.
- [ ] **M7 — Sécurité avancée** : seuil suppression %, verrous/permissions Windows + retry, préfixe `\\?\`, re-stat mid-sync, nettoyage `.synctmp`, vérif `blake3` optionnelle, rotation logs.
- [ ] **M8 — Packaging & CI** : bundle targets ; build local .deb/.rpm + NSIS + portable .exe ; workflow GitHub Actions attachant les 4 artefacts ; (option) signature Authenticode.
- [ ] **M9 — Finitions** : QA cross-plateforme, MSI optionnel, doc utilisateur, polish UI WebKitGTK.

## 11. Risques & sécurité

Le miroir unidirectionnel **supprime** dans la destination — risque majeur de perte de données. Mitigations (intégrées, non optionnelles) :

| Risque | Mitigation |
|---|---|
| Source vide/non montée → wipe destination | **Seuil de suppression** (défaut 50 %) : pause + confirmation si > N% supprimé |
| Suppression irréversible | **Corbeille par défaut** ; mode permanent en opt-in explicite |
| Destination imbriquée dans source (ou inverse) | **Validation à l'ajout/édition** : refus/avertissement bloquant |
| Fichiers dest légitimes supprimés car ignorés | **Ignore appliqué aux DEUX arbres** |
| Re-copie perpétuelle / fausses diffs | **Stamp mtime post-copie** + tolérance mtime + court-circuit taille |
| Crash en cours de copie → fichier corrompu | **Copie atomique** (temp + rename) + nettoyage `.synctmp` |
| Échec corbeille (réseau/cap) | **Log + skip**, jamais de hard-delete silencieux |
| Fichier modifié pendant la sync | **Re-stat avant/après**, flag + re-queue |
| Passes concurrentes (timer + Sync now) | **Mutex unique** ; passe chevauchante skippée + loggée |
| Verrous/permissions Windows (err 32/5) | Gestion **par-fichier** : retry + log + continue |
| Suppressions par erreur | **Dry-run/preview** avec compteur de suppressions en rouge |
| Chemins longs Windows (MAX_PATH) | Préfixe **`\\?\`** systématique |
| Audit / traçabilité | **Journal roulant `tracing`** : chaque op avec chemin + raison |
| Symlinks → boucles | `follow_links(false)` + **skip par défaut** |
