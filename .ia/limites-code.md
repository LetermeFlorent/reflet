# Limites de code — Ledger & Exceptions

> Projet **reflet** (Tauri 2 + SvelteKit). Document interne `.ia/`.
> Dernière mise à jour : **2026-06-04**. Mesures (lignes brutes) sur l'arbre courant.

## 1. Règles

| # | Règle | Seuil | Périmètre |
|---|-------|-------|-----------|
| R1 | Lignes par fichier de code main | **≤ 150 lignes** | `.rs`, `.svelte`, `.ts` du code source |
| R2 | Fichiers par dossier | **≤ 8 fichiers** | dossiers du code source |

**Décision utilisateur (2026-06-04) : découpage « par responsabilité ».**
On découpe là où ça clarifie (extraction de vrais composants / modules à responsabilité unique).
Un conteneur cohésif (formulaire, page, algorithme d'un seul tenant) peut rester un peu > 150
lignes si ça se lit mieux d'un bloc — l'exception est alors **consignée ici**. La cible réelle
est « un fichier = une responsabilité », pas la course au nombre de lignes.

## 2. Découpages réalisés (2026-06-04)

| Avant | Lignes | Après | Statut |
|-------|-------:|-------|--------|
| `sync.rs` | 720 | `sync/{mod,types,scan,diff,plan,io,exec,util,backup}.rs` | ✅ éclaté par responsabilité (déplacement pur, 0 changement de logique, `cargo check` OK) |
| `commands.rs` | 390 | `commands/{mod,app,pairs,settings,sync,logs}.rs` | ✅ éclaté par domaine IPC |
| `PairEditModal.svelte` | 427 | + `ScheduleTimesEditor`, `CompressionSettings`, `ColorPicker` | ✅ 3 sous-composants réutilisables extraits (reste 277) |
| `PairCard.svelte` | 309 | + `CardHeader`, `CardStatusLine`, `SyncButton` | ✅ **122** (sous le seuil) |

## 3. Non-conformités R1 restantes — exceptions cohésives assumées

Chacun fait **une seule chose** ; le dépassement vient d'un bloc cohérent qu'éclater nuirait à la lisibilité.

| Fichier | Lignes | Responsabilité unique | Pourquoi on garde d'un bloc |
|---------|-------:|------------------------|------------------------------|
| `PairEditModal.svelte` | 277 | Formulaire d'édition d'une paire | 11 champs + save ; la logique métier (planif/compression/couleur) est déjà sortie en composants. |
| `routes/+page.svelte` | 245 | Tableau de bord (liste paires + DnD) | Vue conteneur. |
| `routes/compression/+page.svelte` | 239 | Page « formats de compression » | Vue conteneur. |
| `compression.rs` | 234 | Détection + exécution compression | Candidat à `compression/{detect,compress,types}.rs` si besoin (non urgent). |
| `routes/settings/+page.svelte` | 225 | Page Réglages (auto-save) | Vue conteneur. |
| `sync/plan.rs` | 215 | `build_plan` : calcul du plan miroir | **Algorithme d'un seul tenant** ; le découper sans tests = risque de régression. |
| `commands/pairs.rs` | 206 | Commandes IPC du domaine « paires » | 5 commandes + `NewPair`, même domaine. |
| `scheduler.rs` | 196 | Boucle de planification / worker | Cohésif. |
| `routes/+layout.svelte` | 194 | Shell appli (nav + thème) | Vue conteneur. |
| `Select.svelte` | 171 | Composant listbox custom | Un seul composant. |
| `routes/logs/+page.svelte` | 156 | Page Journal | Vue conteneur. |
| `DryRunModal.svelte` | 152 | Modale d'aperçu dry-run | Dépassement marginal (+2). |

## 4. R2 (fichiers / dossier) — sciemment relâché

Le choix « par responsabilité » multiplie les petits fichiers à but unique, ce qui pousse
certains dossiers au-delà de 8 :

| Dossier | Fichiers | Note |
|---------|---------:|------|
| `src-tauri/src/sync` | 9 | Préférence : 9 modules nets à responsabilité unique plutôt qu'un `sync.rs` de 720 lignes. |
| `src/lib/components` | > 8 | Idem : chaque composant fait une chose. Un regroupement par famille (`components/card/…`) reste possible si la racine devient illisible. |

## 5. Conditions de levée des exceptions restantes

1. un filet de tests couvre le périmètre (moteur sync, IPC) ;
2. le découpage est appliqué sans régression (miroir unidirectionnel, sécurités de suppression,
   dry-run, copie atomique, mode sauvegarde) ;
3. ce ledger est mis à jour.

---
*Mesures du 2026-06-04 (lignes brutes). Les fichiers du § 3 sont des exceptions cohésives validées par le choix « par responsabilité », pas de la dette ignorée.*
