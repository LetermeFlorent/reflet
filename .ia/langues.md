# Langues — reflet

> Document de référence sur l'internationalisation (i18n) de Reflet.
> Date : 2026-06-03. Version actuelle : 1.0.0.
> État vérifié à partir du code réel (frontend SvelteKit + backend Rust/Tauri).

## Résumé

- **Langue par défaut : français (`fr`).**
- **Repli (fallback) : `fr`.**
- **i18n NON externalisée à ce jour** : tous les textes visibles par l'utilisateur sont codés **en dur** (français) dans les fichiers source. Il n'existe ni librairie i18n, ni fichiers de traduction, ni sélecteur de langue.
- **L'externalisation i18n est un chantier backloggé** (non commencé, non planifié à date).

## Langues à supporter

> **À CONFIRMER par l'utilisateur.**

À ce jour, seul le **français** est présent. La liste des langues cibles (ex. anglais, néerlandais, etc.) n'est **pas décidée**. Aucune autre langue n'est implémentée ni amorcée.

- Langue actuelle (seule existante) : **français (`fr`)**.
- Langues additionnelles souhaitées : **à confirmer** (aucune pour l'instant).
- **Repli en cas de clé manquante / langue non disponible : `fr`.**

## État actuel : textes codés en dur

Aucune chaîne n'est externalisée. Les textes français sont écrits directement dans le code, des deux côtés de la frontière IPC.

### Frontend (SvelteKit / TypeScript)

- **`src/app.html`** : attribut de langue figé `<html lang="fr">`.
- **Composants `.svelte`** : libellés d'interface en dur. Exemples vérifiés :
  - `src/routes/+page.svelte` : « Tableau de bord », « Mettre en pause » / « Reprendre », « Tout synchroniser », « + Ajouter », « Aucune paire de synchronisation », « Chargement… », messages `toast` (« Paire supprimée », « Synchro lancée : … »), texte de confirmation de suppression.
  - `src/routes/settings/+page.svelte` : tous les titres de sections et libellés (« Réglages », « Synchronisation », « Sécurité des suppressions », « Notifications », « Démarrage », « Affichage », « Exclusions globales »), descriptions et options des `Select` (« Corbeille (recommandé) », « Permanent », « Taille + date (rapide) », etc.), avertissement mode permanent.
  - Autres : `src/routes/logs/+page.svelte`, et les composants de `src/lib/components/*.svelte` (`PairCard`, `PairEditModal`, `DryRunModal`, `IntervalPicker`, `StatusBadge`, etc.).
- **`src/lib/format.ts`** : formatage localisé en dur.
  - Date/heure : `d.toLocaleString("fr-FR", …)` (locale `fr-FR` figée).
  - Unités d'octets : `["o", "Ko", "Mo", "Go", "To"]` (françaises).
  - Intervalles : suffixes `j` / `h` / `min` / `s` (français).

### Backend (Rust / Tauri)

Des chaînes destinées à l'utilisateur sont aussi en français dans le code Rust :

- **`src-tauri/src/tray.rs`** : menu de la barre système (« Afficher Reflet », « Synchroniser tout », « Quitter ») et tooltip « Reflet ».
- **`src-tauri/src/sync.rs`** : titres et corps des **notifications système** (« Reflet — erreur », « Reflet — synchro terminée », « Reflet — synchro avec erreurs », et le corps `"{name} : {n} copiés, {n} màj, {n} supprimés[, {n} erreurs]"`).
- **`src-tauri/src/commands.rs`** : messages d'erreur renvoyés à l'UI (ex. « Source et destination requises », « Paire introuvable »).

### Persistance / configuration

- **`src-tauri/src/config.rs`** — la structure `Settings` **ne contient aucun champ de langue/locale** (`language`, `locale`, …). Le choix de langue n'est donc ni stocké ni configurable aujourd'hui.
- **`package.json`** : aucune dépendance i18n (pas de `svelte-i18n`, `typesafe-i18n`, `paraglide`, `@formatjs/intl`, etc.).

## Chantier backloggé : externalisation i18n

> Cette section décrit un travail **non commencé**. Le périmètre exact reste **à confirmer par l'utilisateur**.

Pistes (indicatives, à valider) pour une future externalisation :

1. **Choisir les langues cibles** — à confirmer (voir « Langues à supporter »).
2. **Frontend** : extraire les chaînes `.svelte` vers des catalogues de traduction (clé → texte), choisir une librairie i18n Svelte adaptée à l'`adapter-static` (build statique, 100 % local, sans réseau).
3. **`format.ts`** : rendre la locale dynamique (date, octets, intervalles) au lieu de `fr-FR` figé ; conserver `fr` comme repli.
4. **`app.html`** : rendre `lang` dynamique selon la langue choisie.
5. **Backend Rust** : externaliser les chaînes utilisateur (tray, notifications, erreurs de `commands.rs`) — décider qui porte la traduction (Rust ou frontend qui mappe les codes d'erreur).
6. **Config** : ajouter un champ de langue dans `Settings` (`config.rs`) + un sélecteur dans Réglages, persisté dans `settings.json` ; **repli `fr`** si la valeur est absente ou inconnue.
7. **Contrainte** : l'application est **100 % locale, aucun appel réseau** — toute solution i18n doit fonctionner hors-ligne et être embarquée dans le bundle.

## Points à confirmer (récapitulatif)

- [ ] **Langues à supporter** (en plus de `fr`) — non décidé.
- [ ] Priorité / planification du chantier d'externalisation i18n — backloggé, non planifié.
- [ ] Répartition de la traduction des chaînes backend (Rust) vs frontend (Svelte).

> Tant que rien n'est confirmé : **langue par défaut `fr`**, **repli `fr`**, **aucune i18n externalisée**.
