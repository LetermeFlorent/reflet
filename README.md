# Reflet ◑

**Synchronisation miroir unidirectionnelle de dossiers** — application de bureau Windows/Linux construite avec **Tauri 2**, **SvelteKit** et **Rust**.

Reflet copie le contenu d'un dossier *source* vers un dossier *destination* pour le maintenir à l'identique (miroir), automatiquement et à intervalle configurable, ou à la demande. Léger, vit dans la zone de notification (tray), et ne touche jamais à la source.

---

## Fonctionnalités

- **Paires de synchronisation** source → destination, activables individuellement.
- **Synchro automatique** par planificateur, intervalle global ou propre à chaque paire.
- **Synchro manuelle** (« Synchroniser ») avec **barre de progression**, **temps restant estimé (ETA)** et **compte à rebours avant la prochaine synchro auto**.
- **Aperçu (dry-run)** : voir ce qui serait copié / mis à jour / supprimé avant d'exécuter.
- **Sécurité suppressions** : seuil configurable qui bloque une passe si elle supprimerait une trop grande part de la destination.
- **Corbeille ou suppression définitive** au choix.
- **Vérification par contenu** optionnelle (BLAKE3) en plus de la date de modification.
- **Filtres d'exclusion** (glob) globaux et par paire.
- **Notifications** système et in-app (opt-in).
- **Journal** des opérations.
- **Démarrage automatique** et **démarrage minimisé** optionnels.
- Fermeture = **réduction dans le tray** (l'app continue de tourner) ; « Quitter » pour fermer vraiment.

## Captures / structure

```
src/                      Frontend SvelteKit (UI)
  lib/components/          Cartes, modales, contrôles
  lib/store.svelte.ts      État réactif partagé (IPC + events Tauri)
  lib/ipc.ts               Pont commandes/événements Tauri
  routes/                  Pages : tableau de bord, réglages, journal
src-tauri/                Backend Rust (Tauri)
  src/sync.rs              Moteur de synchronisation (plan + exécution)
  src/scheduler.rs         Planificateur (1 worker, timer par paire)
  src/commands.rs          Commandes exposées au frontend
  src/config.rs            Modèle + persistance JSON des réglages
  src/state.rs             État applicatif partagé
  src/tray.rs              Icône de notification
  src/lifecycle.rs         Fermeture → tray
```

La configuration est persistée dans le dossier de config de l'app
(`%APPDATA%\com.reflet.desktop\settings.json` sous Windows).

---

## Prérequis

- **Node.js** ≥ 18 et **npm**
- **Rust** (stable) + Cargo — https://rustup.rs
- Dépendances système Tauri 2 :
  - **Windows** : WebView2 (préinstallé sur Windows 11) + Build Tools C++ (MSVC)
  - **Linux (Debian/Ubuntu)** : `libwebkit2gtk-4.1-0`, `libgtk-3-0`, `libayatana-appindicator3-1`
    (build : paquets `-dev` correspondants)

## Installation

```bash
git clone https://github.com/LetermeFlorent/reflet.git
cd reflet
npm install
```

## Développement

Lance Vite + l'app Tauri avec rechargement à chaud :

```bash
npm run tauri dev
```

> Derrière un proxy d'entreprise avec inspection TLS, si le build échoue sur la
> vérification des certificats :
> ```powershell
> $env:CARGO_HTTP_CHECK_REVOKE="false"   # session courante
> $env:NODE_OPTIONS="--use-system-ca"
> npm run tauri dev
> ```

Vérification des types (frontend) :

```bash
npm run check
```

## Build de production

```bash
npm run tauri build
```

Les artefacts sont générés dans `src-tauri/target/release/bundle/` :

- **Windows** : installeur NSIS `…/nsis/Reflet_<version>_x64-setup.exe`
- **Linux** : `.deb` / `.rpm`

Build portable (binaire seul, sans installeur) :

```bash
npm run tauri build -- --no-bundle
# binaire : src-tauri/target/release/reflet.exe
```

---

## Branches

- **`main`** — version stable / publiée.
- **`dev`** — intégration des nouveautés avant fusion dans `main`.

## Licence

MIT.
