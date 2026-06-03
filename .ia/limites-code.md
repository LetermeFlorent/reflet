# Limites de code — Ledger & Exceptions

> Projet **reflet** (Tauri 2 + SvelteKit). Document interne `.ia/`.
> Dernière mise à jour : **2026-06-03**. Données mesurées sur l'arbre courant.

## 1. Règles fixées

| # | Règle | Seuil | Périmètre |
|---|-------|-------|-----------|
| R1 | Lignes par fichier écrit « main » (code applicatif rédigé à la main) | **≤ 150 lignes** | `.rs`, `.svelte`, `.ts` du code source |
| R2 | Fichiers par dossier | **≤ 8 fichiers** | dossiers du code source |

**À confirmer par l'utilisateur :**
- Le décompte de lignes inclut-il commentaires + lignes vides, ou seulement les lignes de code (LOC) ? (Les chiffres ci-dessous sont des **lignes brutes**, tout compris.)
- « Fichier écrit main » = exclut-on explicitement les fichiers générés / vendor / config (ex. `Cargo.lock`, `package-lock.json`, dossiers `node_modules`, `target`, `.svelte-kit`) ? Hypothèse retenue : **oui, exclus**.
- R2 (8 fichiers/dossier) : compte-t-on uniquement les fichiers, ou aussi les sous-dossiers ?

## 2. Statut global

> **Exceptions tolérées, découpage planifié (nécessite un filet de tests d'abord).**

Aucune des non-conformités ci-dessous n'est corrigée immédiatement : refactor reporté tant qu'il n'existe pas de tests de non-régression couvrant le moteur de sync et l'IPC. Le risque d'un découpage à froid (sans filet) est jugé supérieur au bénéfice. Ce fichier sert de **backlog** de dette à résorber.

## 3. Non-conformités R1 (> 150 lignes)

Mesures réelles (lignes brutes) :

| Fichier | Lignes | Dépassement | Nature |
|---------|-------:|------------:|--------|
| `src-tauri/src/sync.rs` | **609** | +459 | Moteur miroir : ~30 fonctions (`walk_tree`, `build_plan`, `execute_plan`, `copy_file_atomic`, `safe_delete`, blake3, DTO `SyncPlan`/`PlanItem`/`SyncOutcome`, `run_sync`, notifications). |
| `src-tauri/src/commands.rs` | **264** | +114 | Frontière IPC : **16** commandes `#[tauri::command]`. |
| `src/routes/settings/+page.svelte` | **264** | +114 | Page Réglages (template + script + style colocalisés). |
| `src/lib/components/PairCard.svelte` | **261** | +111 | Carte d'une paire de sync (état, actions, progression). |
| `src/routes/+page.svelte` | **172** | +22 | Dashboard. **Non listé dans la demande initiale mais réellement > 150** — à confirmer s'il entre dans le backlog. |
| `src/lib/components/DryRunModal.svelte` | **152** | +2 | Modale d'aperçu dry-run. Dépassement **marginal** (2 lignes). |

Pour info, fichiers proches du seuil mais **conformes** (≤ 150) : `PairEditModal.svelte` (192 ❗ en réalité > 150 — voir note), `scheduler.rs` (140), `logs/+page.svelte` (135), `Select.svelte` (127), `store.svelte.ts` (121), `config.rs` (113).

> **Note / à confirmer :** `src/lib/components/PairEditModal.svelte` mesure **192 lignes** et dépasse donc aussi R1, bien qu'il ne figure pas dans la liste de la demande. À arbitrer : l'ajouter au backlog ?

## 4. Non-conformités R2 (> 8 fichiers / dossier)

| Dossier | Fichiers | Dépassement | Détail |
|---------|---------:|------------:|--------|
| `src-tauri/src` | **10** | +2 | `main.rs`, `lib.rs`, `commands.rs`, `sync.rs`, `scheduler.rs`, `config.rs`, `state.rs`, `tray.rs`, `lifecycle.rs`, `logging.rs`. |
| `src/lib/components` | **9** | +1 | `DryRunModal`, `IntervalPicker`, `PairCard`, `PairEditModal`, `ProgressBar`, `Select`, `StatusBadge`, `Switch`, `VirtualList`. |

## 5. Pistes de découpage envisagées (backlog, non engagé)

> Indicatif — **à valider** avant tout refactor. Prérequis commun : **filet de tests d'abord**.

- **`sync.rs`** : extraire en sous-modules logiques, p. ex. `sync/walk.rs` (parcours + ignore globs), `sync/plan.rs` (`build_plan` + DTO), `sync/exec.rs` (`execute_plan`, copie atomique, suppression corbeille). Attention : R2 se durcirait sur `src-tauri/src` (déjà à 10) — privilégier un sous-dossier `sync/`.
- **`commands.rs`** : regrouper les 16 commandes par domaine (paires / sync / settings / logs) en sous-modules réexportés.
- **`settings/+page.svelte` & `PairCard.svelte` & `DryRunModal.svelte`** : externaliser sous-composants et logique vers `src/lib` ; surveiller R2 sur `src/lib/components` (déjà à 9 → un sous-dossier par famille de composants peut être nécessaire).
- **`src-tauri/src` (10 fichiers)** : un sous-dossier `sync/` absorberait l'éclatement de `sync.rs` sans aggraver le compte racine.

## 6. Conditions de levée des exceptions

Une exception est résorbée quand :
1. un filet de tests couvre le périmètre concerné (moteur sync, IPC) ;
2. le découpage est appliqué sans régression fonctionnelle (sync miroir unidirectionnelle, sécurités suppression : corbeille, seuil anti-wipe, dry-run, copie atomique) ;
3. ce ledger est mis à jour (ligne retirée du backlog).

---
*Mesures effectuées le 2026-06-03 via comptage de lignes brutes. Les points marqués « à confirmer » attendent une décision de l'utilisateur.*
