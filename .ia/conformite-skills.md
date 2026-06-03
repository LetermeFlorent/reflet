# Conformité aux skills — reflet

> Rapport de conformité au référentiel **LetermeFlorent/SKILLS**.
> Projet **reflet** (Tauri 2 + SvelteKit), version 1.0.0. Date : **2026-06-03**.
> État établi à partir du code réel de l'arbre courant (lecture des fichiers, pas d'extrapolation).
> Le dépôt du référentiel SKILLS n'est **pas cloné localement** : les conventions ci-dessous sont
> reprises des intitulés fournis dans la consigne. **À confirmer** : se référer au texte exact des
> skills avant de figer une règle.

---

## Conforme / fait

Éléments en place, vérifiés dans le code.

- **Documentation `.ia/` créée.** Le dossier `.ia/` contient 9 documents :
  `architecture.md`, `config.md`, `design.md`, `langues.md`, `limites-code.md`,
  `performance.md`, `plan.md`, `ressources.md`, `resume.md`. Base documentaire interne fonctionnelle.
  - *Réserve* : `.ia/` est actuellement **non suivi par git** (`?? .ia/` dans `git status`). Voir
    la note « git » plus bas — à committer.

- **`.gitignore` propre.** (`.gitignore`) Couvre les artefacts lourds et sensibles :
  `node_modules`, `/build`, `/.svelte-kit`, `/package`, `/src-tauri/target`,
  `.env` et `.env.*` (avec exception `!.env.example`), `vite.config.*.timestamp-*`,
  fichiers OS/IDE (`Thumbs.db`, `.vscode`, `.idea`, `*.swp`, `.DS_Store`). Pas de fuite d'artefacts de build.

- **Secrets : OK.** Aucune clé/secret en dur trouvé dans le code source. L'application est
  **100 % locale, sans aucun appel réseau** (aucun client HTTP, pas d'URL distante dans `src-tauri/src`
  ni `src/`). Les `.env*` sont ignorés par git. Rien à exfiltrer côté config.

- **Dates en UTC / ISO 8601.** (`src-tauri/src/sync.rs`) Tous les horodatages passent par
  `now_iso()` = `chrono::Utc::now().to_rfc3339()` (UTC, format RFC 3339 / ISO 8601). Utilisé pour
  les entrées de journal (`LogEntry.at`) et `LastRun.at`. Pas d'heure locale ambiguë en persistance.

- **Logging via `tracing`.** (`src-tauri/src/logging.rs`) Initialisation `tracing_subscriber::fmt`
  + `tracing_appender::rolling::daily` (rotation journalière, fichier `reflet.log` dans
  `app_log_dir`). Les opérations de sync émettent `tracing::info!/warn!/error!` avec champs
  structurés (`pair`, `action`, `path`) — voir `log()` dans `sync.rs`.

- **Sécurités de suppression en place.** Vérifiées dans `sync.rs` / `config.rs` :
  - **Corbeille par défaut** : `delete_behavior: "trash"` (crate `trash`) ; mode « permanent » optionnel.
  - **Seuil anti-wipe** : `delete_safety_threshold_pct` (50 % par défaut) ; si le % d'éléments à
    supprimer dépasse le seuil, les suppressions sont **retenues** et journalisées (`aborted_safety`),
    les copies/maj se font quand même. Seuil à 100 % = protection désactivée.
  - **Source introuvable** ⇒ synchro **annulée** (jamais de vidage de destination).
  - **Copie atomique** : `copy_file_atomic` (fichier temporaire `.synctmp-…` puis `rename`, nettoyage
    du tmp en cas d'échec, retry après 60 ms), préfixe `\\?\` Windows (chemins longs).
  - **Dry-run** : commande `dry_run` qui calcule le plan sans rien écrire.
  - Liens symboliques / points de jonction **non suivis** (`is_reparse_point`).

- **Commits sur `dev`, pas `main`.** ⚠️ **NON vérifié comme conforme à date — À CONFIRMER / corriger.**
  État réel mesuré (`git`) :
  - Branches : `dev` et `main` pointent **sur le même commit** (`b147b42`). Aucun commit propre à `dev`
    (`git log dev --not main` est vide).
  - La **branche courante est `main`** (pas `dev`).
  - Le travail en cours (dont **`.ia/` et `.github/`**) est **non committé** et présent sur `main`
    (working tree modifié : `config.rs` + plusieurs `.svelte`/`.ts`, plus `?? .ia/`, `?? .github/`).
  - **Conclusion** : la règle « commits sur `dev`, pas sur `main` » n'est **pas** respectée en l'état.
    Action attendue (à confirmer) : se placer sur `dev`, y committer le travail (`.ia/`, `.github/`,
    modifs en cours), garder `main` propre / réservé aux releases.

---

## Convention adoptée (à appliquer en continu)

Règles de travail retenues pour la suite. Aucune action ponctuelle : ce sont des réflexes à tenir
sur chaque contribution. **À confirmer** au regard du texte exact des skills correspondants.

- **affiner-prompt** : reformuler / cadrer la demande avant d'agir ; lever les ambiguïtés et marquer
  explicitement ce qui doit être confirmé par l'utilisateur (déjà appliqué dans les docs `.ia/`).

- **nommage anglais** : nommer en anglais les identifiants de code (fonctions, variables, types,
  modules). Constat actuel : le **code** Rust/TS est déjà majoritairement en anglais
  (`walk_tree`, `build_plan`, `execute_plan`, `SyncPair`, `Settings`…). À maintenir.
  *Distinct des textes UI* (français, voir backlog i18n) et des **messages d'erreur** Rust
  (actuellement en français, ex. « Paire introuvable » — relève de i18n / messages-utilisateur).

- **complexité** : garder les fonctions simples, éviter l'imbrication et les responsabilités
  multiples ; préférer des fonctions courtes à intention unique.

- **fonctions modulaires** : découper par responsabilité, frontières nettes
  (UI Svelte → commandes Tauri → cœur sync, jamais l'inverse — déjà respecté). À préserver lors des
  futurs découpages (voir backlog R1).

- **code-mort** : pas de code/commentaires/imports inutilisés. Constat : le dépôt a déjà fait l'objet
  d'un nettoyage (commit « suppression de tous les commentaires du code + nettoyage repo »). À tenir
  à jour à chaque modification.

- **messages-utilisateur** : soigner les messages destinés à l'utilisateur (erreurs, notifications,
  toasts), clairs et actionnables. Aujourd'hui en français en dur (tray, notifications, erreurs IPC) ;
  leur externalisation rejoint le chantier i18n du backlog.

- **vérifier-avant-affirmer** : ne rien affirmer sans l'avoir vérifié (build, tests, comportement).
  Convention clé tant qu'il n'existe pas de filet de tests : toute affirmation de conformité
  fonctionnelle doit s'appuyer sur une exécution réelle, pas sur une lecture seule.

---

## Backlog (chantiers, par priorité)

Dette identifiée, **non corrigée immédiatement**. Principe directeur : **ne pas refactorer à froid un
moteur qui supprime des fichiers, sans filet de tests** — le risque de régression (perte de données en
destination) l'emporte sur le bénéfice. Le premier item est donc un **prérequis** de tous les autres.

### 1. Infra de tests (Vitest + `cargo test`) — **PRÉALABLE**

- **Quoi** : monter un filet de tests avant tout refactor.
  - Rust : `cargo test` sur le cœur (`build_plan`, `detect_changed`, `paths_overlap`, copie atomique,
    `safe_delete`, seuil anti-wipe) avec arbres temporaires.
  - Frontend : Vitest sur `format.ts`, le store réactif, et les mappings IPC (`ipc.ts`).
- **Pourquoi pas fait maintenant** : c'est justement le **préalable**. Tant qu'il n'existe pas, tous
  les chantiers ci-dessous (découpage, suppression des `.unwrap()`, i18n) se feraient **sans
  non-régression** sur un moteur destructif. Risque : casser silencieusement la sync miroir
  (suppressions en destination). Ordre imposé : **tests d'abord**, refactor ensuite.
- *À confirmer* : périmètre minimal de couverture exigé, frameworks (Vitest confirmé ? `cargo test`
  natif ou via harnais ?).

### 2. Découpage des fichiers > 150 lignes (règle R1)

- **Quoi** : ramener sous le seuil les fichiers dépassant 150 lignes. Recensés (voir
  `.ia/limites-code.md` pour les mesures détaillées) : `sync.rs`, `commands.rs`,
  `settings/+page.svelte`, `PairCard.svelte`, `+page.svelte`, `DryRunModal.svelte`
  (+ `PairEditModal.svelte` à arbitrer). Pistes : sous-dossier `sync/` (walk/plan/exec),
  regroupement des commandes IPC par domaine, extraction de sous-composants Svelte.
  - *À confirmer* : comptage en lignes brutes vs LOC ; périmètre exact (cf. notes de `limites-code.md`).
- **Pourquoi pas fait maintenant** : découper `sync.rs` (le moteur destructif) **sans tests** = risque
  élevé de régression non détectée. Conditionné à l'item 1.

### 3. Suppression des ~30 `.unwrap()` Rust (échec propre + trace)

- **Quoi** : ~**32** `.unwrap()` mesurés dans `src-tauri/src` (`commands.rs` 16, `scheduler.rs` 9,
  `lib.rs` 3, `state.rs` 3, `sync.rs` 1), majoritairement des `mutex.lock().unwrap()` (+ un `.expect(...)`
  au build Tauri dans `lib.rs`). Remplacer par une gestion d'erreur propre : log `tracing` + remontée/
  dégradation contrôlée au lieu d'un `panic!` (notamment sur Mutex empoisonné).
  - *À confirmer* : politique en cas de Mutex empoisonné (récupérer le garde via `into_inner` ?
    redémarrer le worker ? remonter une erreur IPC ?).
- **Pourquoi pas fait maintenant** : changer le comportement d'erreur du cœur et du scheduler peut
  altérer la sémantique (ce qui paniquait/abandonnait se mettrait à continuer). **Sans tests**, on ne
  peut pas garantir l'absence de régression. Conditionné à l'item 1.

### 4. i18n — externaliser les textes

- **Quoi** : aujourd'hui **tout est en français codé en dur** (aucune lib i18n — cf. `.ia/langues.md`).
  Frontend (`.svelte`, `format.ts` avec `fr-FR` et unités `o/Ko/Mo`, `app.html lang="fr"`) et backend
  (`tray.rs`, notifications de `sync.rs`, erreurs de `commands.rs`). Externaliser vers des catalogues,
  rendre la locale dynamique, ajouter un champ langue dans `Settings`, repli `fr`.
  - *À confirmer* : **langues cibles** (aucune décidée), répartition traduction Rust vs Svelte,
    priorité du chantier. Contrainte : solution 100 % offline (adapter-static, pas de réseau).
- **Pourquoi pas fait maintenant** : chantier large touchant les deux côtés de l'IPC et la persistance
  (`Settings`). Sans tests ni périmètre de langues confirmé, l'externalisation risque de casser des
  libellés/formats sans détection. Conditionné à l'item 1 et à une décision produit.

### 5. Commentaires / doc en anglais

- **Quoi** : aligner commentaires et documentation de code sur l'anglais (le nommage l'est déjà).
  Constat : le code a été dépouillé de ses commentaires (commit de nettoyage) — il y a donc **peu de
  commentaires** à convertir aujourd'hui, mais la règle s'applique à toute doc/commentaire **futur**.
  - *À confirmer* : la règle vise-t-elle aussi les docs `.ia/` (actuellement en français, volontairement) ?
    Hypothèse : non — `.ia/` est de la doc projet FR, distincte des commentaires de code.
- **Pourquoi pas fait maintenant** : faible volume de commentaires existants ; à traiter au fil de
  l'eau lors des items 2/3 plutôt qu'en passe dédiée. Pas de risque technique, mais sans intérêt à
  isoler maintenant.

### 6. Hook PostToolUse de vérification (`.claude/settings.json`)

- **Quoi** : ajouter un hook PostToolUse (dans un `.claude/settings.json` de projet, **absent à ce
  jour**) qui vérifie automatiquement après édition (ex. `cargo check`/`clippy`, `svelte-check`,
  `tsc`, lint) pour matérialiser le réflexe « vérifier-avant-affirmer ».
  - *À confirmer* : commandes exactes à lancer dans le hook, et OS/shell cible (Windows PowerShell ici).
- **Pourquoi pas fait maintenant** : un hook qui lance `cargo check` / `svelte-check` n'a de valeur que
  s'il existe une **base de tests/vérifs fiable** (item 1) ; mis en place trop tôt, il bruiterait sans
  garde-fou réel. À cadrer avec l'utilisateur (skill `update-config`).

### 7. Persistance : `schemaVersion` + écriture atomique + backup

- **Quoi** : durcir `config.rs::save`. Aujourd'hui : `serde_json::to_string_pretty` puis
  `std::fs::write` **direct** (non atomique : une coupure en plein write peut corrompre/tronquer
  `settings.json`), **aucun `schemaVersion`** dans `Config`/`Settings`, **aucun backup** avant écrasement.
  Cible : champ `schemaVersion` (migration future), écriture **atomique** (tmp + rename, comme
  `copy_file_atomic`), et **backup** du précédent `settings.json` avant remplacement.
  - *À confirmer* : nombre de backups à conserver, emplacement, stratégie de migration de schéma.
- **Pourquoi pas fait maintenant** : `save` est appelé par quasiment toutes les commandes IPC et par
  `update_last_run` à chaque fin de sync. Modifier le chemin d'écriture de la config sans tests
  (item 1) risque de corrompre la persistance de **tous** les utilisateurs. Conditionné à l'item 1.

---

*Rapport établi le 2026-06-03 à partir du code source réel. Les points marqués « À confirmer » /
« ⚠️ » attendent une décision ou une vérification de l'utilisateur. Le référentiel SKILLS n'étant pas
disponible localement, les intitulés de conventions sont repris de la consigne : à recouper avec le
texte exact des skills.*
