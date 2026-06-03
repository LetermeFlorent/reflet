# Configuration du projet Reflet

> Recensement des **sources de configuration** du projet et de la **source unique de vérité (SSOT)** par domaine.
> Basé sur les fichiers réels du dépôt (lecture directe, pas d'invention). Date : 2026-06-03.
> Les points à valider sont marqués **À CONFIRMER**.

---

## 1. Vue d'ensemble : une SSOT par domaine

| Domaine | Source unique de vérité | Fichier |
|---|---|---|
| Identité applicative (nom, version, identifiant, fenêtre, bundle) | Tauri | `src-tauri/tauri.conf.json` |
| Dépendances & métadonnées Rust (crate, version, plugins, lib) | Cargo | `src-tauri/Cargo.toml` |
| Dépendances & scripts Node/front (npm, version) | npm | `package.json` |
| Build front + adaptateur SvelteKit (SPA/SSG) | SvelteKit | `svelte.config.js` |
| Serveur de dev / bundling Vite (port 1420, watch) | Vite | `vite.config.js` |
| Typage TypeScript / svelte-check | TypeScript | `tsconfig.json` |
| Permissions IPC (capabilities Tauri) | Tauri ACL | `src-tauri/capabilities/default.json` |
| Préférences utilisateur **à l'exécution** (paires, intervalle, sécurités) | Runtime | `settings.json` (hors dépôt, `%APPDATA%`) |
| CI / release | GitHub Actions | `.github/workflows/release.yml` |
| Éditeur (non normatif) | VS Code | `.vscode/settings.json`, `.vscode/extensions.json` (gitignorés) |

**Règle de non-duplication :** chaque valeur n'a **qu'une** source faisant autorité.
Quand une donnée apparaît à deux endroits (ex. la **version** `1.0.0`), voir §10 (doublons connus à synchroniser à la main).

---

## 2. `src-tauri/tauri.conf.json` — Identité applicative & bundle

SSOT pour : nom produit, version applicative, identifiant, fenêtre, sécurité (CSP), bundle.

- `productName`: `Reflet`
- `version`: `1.0.0`
- `identifier`: `com.reflet.desktop` → détermine le dossier de config runtime (`%APPDATA%/com.reflet.desktop/`, voir §8).
- `build`:
  - `beforeDevCommand`: `npm run dev`, `devUrl`: `http://localhost:1420`
  - `beforeBuildCommand`: `npm run build`, `frontendDist`: `../build`
- `app.windows[main]`: 960×680, min 720×520, visible/resizable/decorations = true.
- `app.security.csp`: `null` (pas de Content-Security-Policy définie).
  **À CONFIRMER** : CSP volontairement absente pour une app 100 % locale ; à durcir si du contenu distant était un jour chargé (ce qui n'est pas le cas aujourd'hui).
- `bundle.active`: `true` ; `targets`: `nsis` (Windows), `deb`, `rpm` (Linux) ; icônes `icons/*` ; dépendances Linux deb listées.

> Lien build : `devUrl` (1420) et `frontendDist` (`../build`) doivent rester cohérents avec Vite (§5) et l'adapter-static (§4).

---

## 3. `src-tauri/Cargo.toml` — Dépendances & métadonnées Rust

SSOT pour : nom de crate, version Rust, édition, plugins Tauri côté Rust, dépendances du moteur de sync.

- `[package]`: `name = "reflet"`, `version = "1.0.0"`, `edition = "2021"`.
  - `authors = ["you"]` → **À CONFIRMER** : valeur placeholder par défaut, à remplacer (ex. `Florent Leterme`).
- `[lib]`: `name = "reflet_lib"`, `crate-type = ["staticlib", "cdylib", "rlib"]`.
- Tauri & plugins : `tauri` (feature `tray-icon`), `tauri-plugin-dialog`, `tauri-plugin-notification`, et (hors mobile) `tauri-plugin-single-instance`, `tauri-plugin-autostart`.
- Moteur de sync : `walkdir`, `filetime`, `trash` (corbeille), `globset`, `blake3`.
- Logging : `tracing`, `tracing-subscriber` (`env-filter`), `tracing-appender`.
- Divers : `serde`/`serde_json`, `uuid` (v4), `chrono`, `dunce`, `tokio` (`time`, `sync`, `rt`, `macros`).

> Note : les versions sont en sémantique large (`"2"`, `"1"`, etc.). Le **verrouillage réel** des versions Rust est dans `src-tauri/Cargo.lock` (présent dans l'arbre = SSOT des versions résolues côté Rust). **À CONFIRMER** : `Cargo.lock` est bien suivi par git (recommandé pour un binaire d'application — il n'apparaît pas dans `.gitignore`, donc a priori versionné).

---

## 4. `svelte.config.js` — Build front (SPA/SSG)

SSOT pour : préprocesseur Svelte et adaptateur de sortie.

- `preprocess: vitePreprocess()`.
- `kit.adapter: adapter-static` avec `fallback: "index.html"` → mode **SPA** (rendu côté client), cohérent avec une app Tauri locale.

> La sortie statique alimente `frontendDist: "../build"` côté Tauri (§2).

---

## 5. `vite.config.js` — Serveur de dev & bundling

SSOT pour : port de dev, hôte, HMR, exclusions de watch.

- `plugins: [sveltekit()]`, `clearScreen: false`.
- `server.port: 1420`, `strictPort: true` → doit rester aligné avec `devUrl` de Tauri (§2).
- `server.host`: piloté par la variable d'environnement `TAURI_DEV_HOST` (injectée par la CLI Tauri ; sinon `false`).
- HMR sur port `1421` (ws) uniquement si `TAURI_DEV_HOST` est défini.
- `server.watch.ignored: ["**/src-tauri/**"]` (évite les boucles de rebuild front quand le Rust change).

---

## 6. `tsconfig.json` — Typage TypeScript

SSOT pour : options de compilation TS / vérification `svelte-check`.

- `extends: "./.svelte-kit/tsconfig.json"` (config générée par SvelteKit ; alias `$lib` géré par SvelteKit, pas ici).
- `compilerOptions`: `strict: true`, `checkJs: true`, `allowJs: true`, `esModuleInterop`, `forceConsistentCasingInFileNames`, `resolveJsonModule`, `skipLibCheck`, `sourceMap`, `moduleResolution: "bundler"`.

> `.svelte-kit/` est généré (gitignoré) ; ne pas éditer le tsconfig généré à la main.

---

## 7. `package.json` — Dépendances & scripts Node

SSOT pour : version npm du projet, scripts, dépendances front et outillage.

- `name: "reflet"`, `version: "1.0.0"`, `type: "module"`, `license: "MIT"`.
  - `description` vide → **À CONFIRMER** (cohérence avec la description Cargo « synchronisation miroir unidirectionnelle »).
- Scripts : `dev` (`vite dev`), `build` (`vite build`), `preview`, `check` / `check:watch` (`svelte-kit sync` + `svelte-check`), `tauri`.
- Dépendances : `@tauri-apps/api`, `@tauri-apps/plugin-dialog`, `@tauri-apps/plugin-opener`.
- DevDeps : `@sveltejs/adapter-static`, `@sveltejs/kit`, `@sveltejs/vite-plugin-svelte`, `@tauri-apps/cli`, `sharp`, `svelte` (5), `svelte-check`, `typescript ~5.6.2`, `vite ^6`.

> Le **verrouillage réel** des versions JS est dans `package-lock.json` (présent à la racine) = SSOT des versions résolues côté Node.

---

## 8. `settings.json` (runtime) — Préférences utilisateur

**Hors dépôt.** Source unique de vérité pour l'**état/préférences à l'exécution** : aucune valeur de ce fichier n'est codée ailleurs.

- **Emplacement** : `app_config_dir()/settings.json` (cf. `src-tauri/src/config.rs`, `config_path`). Sous Windows : `%APPDATA%/com.reflet.desktop/settings.json` (l'identifiant vient de `tauri.conf.json`, §2).
- **Création** : à la première sauvegarde (`config::save` crée le dossier parent). Absent du dépôt et **non gitignoré explicitement** — il ne peut pas y arriver puisqu'il vit dans `%APPDATA%`.
- **Schéma** (sérialisé `camelCase` via serde) :
  - `Config` = `{ settings: Settings, pairs: Vec<SyncPair> }`.
  - `Settings` (valeurs par défaut, `config.rs`) :
    - `intervalSec: 900`, `deleteBehavior: "trash"`, `autostart: false`, `startMinimized: false`,
    - `confirmDeletesWithDryRun: true`, `verifyByContent: "off"`, `mtimeToleranceSec: 2`,
    - `deleteSafetyThresholdPct: 50`, `schedulerRunning: true`,
    - `notifyPc: false`, `notifyApp: false`, `compactCards: true`,
    - `ignorePatterns`: `["**/*.tmp", "**/~$*", "**/Thumbs.db", "**/.DS_Store", "**/.git/**"]`.
  - `SyncPair` : `id`, `name`, `source`, `destination`, `enabled`, `intervalSecOverride?`, `notifyPc`, `notifyApp`, `ignorePatterns`, `lastRun?`.
- **Robustesse** : si `settings.json` est illisible, `load` journalise un warning et retombe sur `Config::default()`.

> Conséquence : les **valeurs par défaut** des réglages sont définies en dur dans `config.rs` (`impl Default for Settings`). C'est la SSOT des **défauts** ; le fichier runtime est la SSOT de l'**état effectif** sur la machine de l'utilisateur.

---

## 9. `src-tauri/capabilities/default.json` — Permissions IPC (ACL Tauri)

SSOT pour : permissions accordées à la fenêtre `main` (frontière IPC Tauri v2).

- `windows: ["main"]`.
- `permissions`: `core:default`, plusieurs `core:window:*` (start-dragging, minimize, toggle-maximize, set/is-fullscreen), `dialog:default`, `notification:default`.

> `$schema` pointe vers `../gen/schemas/desktop-schema.json` (généré dans `src-tauri/gen/schemas/`, artefact de build).

---

## 10. Doublons connus (à synchroniser manuellement)

Ces valeurs existent dans plusieurs fichiers par nécessité de l'outillage ; il n'y a pas de mécanisme automatique de propagation.

- **Version `1.0.0`** présente dans : `tauri.conf.json`, `Cargo.toml`, `package.json`.
  - SSOT « affichée à l'utilisateur » = `tauri.conf.json` (`version` du produit packagé).
  - **À CONFIRMER** : convention de bump retenue (mettre à jour les 3 fichiers à chaque release ; voir aussi le workflow `.github/workflows/release.yml`).
- **Nom `reflet`** : `Cargo.toml` (`package.name`) et `package.json` (`name`) ; `productName` (`Reflet`) est distinct et propre à Tauri.
- **Port `1420`** : `vite.config.js` (`server.port`) et `tauri.conf.json` (`devUrl`) — à garder identiques.
- **Patterns d'ignore par défaut** : définis dans `config.rs` (défauts) ; rien d'équivalent dans un fichier de config statique → pas de doublon.

---

## 11. Décision : pas de fichier de config projet dédié supplémentaire

**Décision (À CONFIRMER par l'utilisateur)** : ne **pas** introduire de fichier de configuration projet additionnel (ex. `reflet.config.json`, `app.config.json`, fichier `.json` maison versionné).

Justification au vu des fichiers réels :
- Chaque domaine a déjà une SSOT claire (§1) couverte par l'outillage standard (Tauri / Cargo / npm / Vite / SvelteKit / TS).
- Les **préférences utilisateur** vivent dans `settings.json` runtime (`%APPDATA%`), pas dans le dépôt — par conception (app 100 % locale, aucune valeur sensible à versionner).
- Les **défauts** sont en dur dans `config.rs`, ce qui évite un fichier de config statique parallèle à maintenir.

Ajouter un `.json` de config projet dédié créerait un nouveau point de duplication (versions, ports, patterns) sans bénéfice tant que l'app reste locale et mono-cible.

---

## 12. Vérification : aucun secret versionné

Recherche effectuée sur tout le dépôt (hors `node_modules`) des motifs `secret|password|token|api_key|private_key`.

- **Aucun secret réel détecté.** Les correspondances trouvées sont toutes légitimes :
  - `.github/workflows/release.yml` : `GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}` → **référence** au secret injecté par GitHub Actions, aucune valeur en clair.
  - `src/routes/settings/+page.svelte` : exemple de pattern d'ignore `**/secret.txt` (texte d'aide UI).
  - `PLAN.md` et `src/styles/global.css` : occurrences du mot « **tokens** » au sens *design tokens*, sans rapport avec un secret.
- **`.gitignore`** protège déjà : `.env`, `.env.*` (sauf `.env.example`), `node_modules`, `/build`, `/.svelte-kit`, `/src-tauri/target`, `.vscode`, `.idea`, etc.
- Aucun fichier `.env*` présent dans le dépôt.
- `settings.json` runtime (qui pourrait contenir des **chemins** source/destination personnels) vit dans `%APPDATA%`, donc jamais versionné. Il ne contient de toute façon aucun secret d'authentification (app sans réseau).

**Conclusion** : pas de secret dans le dépôt. **À CONFIRMER** uniquement : que le dépôt public ne contienne pas non plus de `settings.json` d'exemple avec des chemins personnels (aucun trouvé à ce jour).

---

## Annexe — Fichiers de config réellement présents (racine et `src-tauri`)

- Racine : `package.json`, `package-lock.json`, `svelte.config.js`, `vite.config.js`, `tsconfig.json`, `.gitignore`.
- `src-tauri/` : `tauri.conf.json`, `Cargo.toml`, `Cargo.lock` (présent ; non listé dans `.gitignore` donc a priori versionné), `capabilities/default.json`, `gen/schemas/*.json` (générés).
- `.vscode/` : `settings.json`, `extensions.json` (gitignorés via `.vscode`).
- `.github/workflows/` : `release.yml`.
