# Analyse Performance & Sécurité — Reflet

> App desktop **Tauri 2** (Rust) + **SvelteKit**. Synchro miroir unidirectionnelle, **100 % locale** (aucun réseau, mono-utilisateur).
> Date : **2026-06-04**. Audit multi-agents (8 auditeurs parallèles → synthèse → vérification adversariale).
> Périmètre lu : `src-tauri/src/**` (sync, scheduler, watcher, commands, config, compression) + `src/**` (store, composants).

## Méthodologie

49 findings bruts → **38 consolidés** (dédup) → **12 findings sécurité vérifiés** de façon adversariale →
**2 confirmés haute-confiance**, **10 abaissés / rejetés** (faux positifs en contexte local mono-utilisateur).
Le modèle de menace écarte les classes web (XSS réseau, CSRF, multi-tenant) : l'app est locale, sans serveur.

## État des corrections (2026-06-04)

**Appliqué** ✅ : P1 (reframe + skip-si-inchangé), P4, P5, P10, P11, P12, P16, P22, P23, S3, S4.
**Écarté** (inhérent au but de l'app, ou « fix » qui casserait une fonctionnalité / régression sans tests) :
- **P3** — `verify_by_content` re-hache tout : c'est le *but* du mode (détecter une modif à taille+mtime identiques). Sauter le hash = neutraliser la fonction.
- **P7** — le ZIP réécrit le conteneur entier : nature du format zip (déjà mitigé par `carry_forward`).
- **P6** — plafond `read_index` : marquer une archive comme illisible **abandonnerait** une vraie grosse sauvegarde (>seuil) → pire que le DoS auto-infligé.
- **P2, P9, P14, P17, P19, P20, P21, P13/P15** — architecturaux / micro-perf / churn sans bénéfice net.
**Reporté** (changement à part entière, à valider) :
- **S1** (mdp → keyring/DPAPI) : touche le round-trip IPC du secret + migration ; risque de régression sur la fonctionnalité fraîche (mdp perdu = archives chiffrées cassées). À faire en changement dédié.
- **S2** (timeout outils externes) : un timeout fixe **tuerait** une grosse compression légitime ; nécessite un réglage configurable + UI.
- **S5** (`cookie<0.7` npm) : bump `@sveltejs/kit` = churn lockfile, hors de ce lot.

---

## 1. Sécurité

| # | Sévérité | Problème | Emplacement | Impact | Recommandation |
|---|----------|----------|-------------|--------|----------------|
| S1 | **Moyen** | Mot de passe de compression stocké **en clair** dans `settings.json` | `config.rs:167-173`, `compression.rs:27` | `compression.password` sérialisé tel quel sur disque. Fuite possible via sync cloud (OneDrive), sauvegardes, malware sous l'identité user. **Pas d'élévation de privilège** (`%APPDATA%\Roaming` protégé par ACL profil ; un admin contourne toute ACL de toute façon). Secret protège une archive de fichiers déjà en clair côté source. | Idéal : credential store OS (`keyring`/DPAPI) ou champ chiffré (`aes-gcm` + clé au keyring), `zeroize` en mémoire. A minima : `#[serde(skip)]` + saisie à la volée. ⚠ L'exposition argv (7z/rar) subsisterait même avec keyring. |
| S2 | **Moyen** | Aucun **timeout** sur les outils externes (gel possible) | `compression.rs:44`, `build_external` | `Command::output()` sans deadline. Un NAS/disque réseau qui se déconnecte pendant la compression fige le worker scheduler (unique, séquentiel) → toutes les synchros suivantes bloquées. UI reste réactive (`spawn_blocking`). C'est de la **fiabilité**, pas une faille exploitable. | `spawn()` + `wait-timeout` + kill au dépassement. Timeout **généreux/configurable** (ne pas tuer une grosse compression légitime). |
| S3 | Faible | `delete_pct` : multiplication `u32` sans garde d'overflow | `plan.rs:136-140`, `archive.rs:148,196` | Théorique (>42M entrées). Division par zéro déjà gardée. | `(extra as u64).saturating_mul(100) / (total as u64).max(1)`. |
| S4 | Faible | `has_binary()` exécute un binaire sans liste blanche explicite | `compression.rs:43-45` | Noms codés en dur aujourd'hui (pas de risque réel) ; deviendrait exploitable si la détection devenait dynamique/configurable. | Liste blanche explicite ou crate `which` ; documenter l'invariant « noms codés en dur ». |
| S5 | Faible | Dép. npm transitive `cookie <0.7.0` (via `@sveltejs/kit`) | `package.json` (GHSA-pxg6-pf52-xh8x) | Impact quasi nul (app desktop sans serveur HTTP), mais présent dans la supply chain. | MAJ `@sveltejs/kit` (≥2.62) / `npm audit fix` + `npm audit` en CI. |

### Faux positifs notables (vérifiés, écartés)

- **MDP exposé au frontend via `get_app_state`** — *by design* : le WebView est le propriétaire légitime du secret (préremplit le champ d'édition). Le masquer **casserait** l'édition.
- **Permissions `settings.json`** — `icacls` mesuré : `SYSTEM/Admins/<user>` uniquement, **aucun `Users/Everyone`**. Un autre utilisateur standard ne peut pas lire. Prémisse du finding démentie.
- **Injection shell `open_url` (`cmd /C start`)** — mécanisme réel mais source **100 % fermée** (11 URLs codées en dur, sans métacaractère). Non exploitable ; à durcir en defense-in-depth (`opener`/`ShellExecuteW`).
- **TOCTOU / symlink à la suppression** — `remove_dir_all` moderne est conscient des reparse points (post-CVE-2022-21658) ; `walk_tree` filtre déjà les jonctions. Pas de wipe récursif. Résidu mineur : `cleanup_dest` ne filtre pas les reparse points.
- **CSP `null`** — aucun sink HTML (`{@html}`, `innerHTML`…), Svelte 5 échappe tout, aucun contenu distant. Durcissement low-priority.
- **`^` semver non épinglé** — `package-lock.json` committé + `npm ci` en CI → déjà reproductible.
- **MDP « jamais appliqué »** — *obsolète* : le chiffrement est désormais câblé (AES-256 ZIP intégré + `-p`/`-hp` 7z/rar).

---

## 2. Performance

| # | Sévérité | Problème | Emplacement | Impact | Recommandation |
|---|----------|----------|-------------|--------|----------------|
| P1 | **Moyen** ✅ | Formats externes : rebuild complet **même quand la source n'a pas changé** (scheduler auto) | `archive.rs` (`run_external`) | Le full-rebuild « une archive = miroir de la source » est le **design assumé** (le ZIP intégré est la voie incrémentale, documenté dans l'UI). Le vrai gaspillage = rebuild **inconditionnel** même sans changement → recompresse des Go pour 0 delta à chaque tick. | **Corrigé** : signature métadonnées (chemins+tailles+mtimes via blake3) mise en cache hors destination ; si l'archive existe et la signature est inchangée → reconstruction **ignorée** (« déjà à jour »). « Rendre 7z incrémental » = écarté (contre la nature du format). |
| P2 | **Élevé** | **Double parcours FS** (dry-run + exécution) | `archive.rs:117,126,184` | `plan_archive()` et `run_archive_sync()` appellent chacun `walk_tree()` → 2× les `stat()` sur gros arbres. | Mémoriser/transmettre le scan du plan vers l'exécution, ou cache invalidé par mtime du dossier. |
| P3 | **Élevé** | `verify_by_content` court-circuite le fast-path → CRC/blake3 sur **tous** les fichiers | `archive.rs:40-56`, `archive_io.rs:41-52` | Dès `verify != off`, relecture intégrale de chaque fichier inchangé, y compris en dry-run. Des dizaines de Go relus à chaque run. | Ne hacher que si taille+mtime diffèrent (ou collision) ; mode échantillonnage ; documenter le coût. |
| P4 | **Élevé** | Débounce watcher **n'avance pas** le timestamp quand busy | `watcher.rs:67-79` | Pendant une synchro, les events font `continue` sans `map.insert(now)` → re-déclenchement immédiat en fin de synchro = synchros parasites en boucle. | Toujours avancer le timestamp (même busy), ou débounce sur dernier event reçu. |
| P5 | **Élevé** | Fuite d'écouteurs `window` dans `Select.svelte` | `Select.svelte:60-65,82-84` | `$effect` attachent scroll/resize/mousedown/keydown ; réouvertures répétées peuvent accumuler des listeners → re-renders multipliés. | Cleanup du `$effect` retirant exactement les mêmes références (`removeEventListener` même fn + flag capture). |
| P6 | Moyen | `read_index` sans plafond d'entrées (DoS mémoire sur archive forgée) | `archive_io.rs:105-122` | 2 `HashMap` remplies sans borne ; archive à millions d'entrées → explosion mémoire (dry-run + synchro). | `MAX_ARCHIVE_ENTRIES` (ex. 1M), abandon si dépassé ; pré-allouer borné. |
| P7 | Moyen | Recompression des inchangés (ZIP intégré, `carry_forward` partiel) | `archive.rs:226-284` | Conteneur entier réécrit dans un `.tmp` à chaque run ; New/Changed ou échec carry_forward → recompression. ~9,9 Go réécrits pour 1 % de delta sur 10 Go. *(NB : avec mot de passe c'est un full-rebuild assumé.)* | Maximiser le carry_forward brut ; logguer le ratio recompressé. |
| P8 | Moyen | Scan FS inutile en dry-run (formats externes) | `archive.rs:115-123` | `walk_tree` complet pour afficher « tout sera réarchivé » (résultat connu). | Court-circuiter : mode « reconstruction complète » sans scan. |
| P9 | Moyen | Allocations `String` redondantes (walk + index) | `scan.rs`, `archive.rs:189` | Chemins clonés en `String` plusieurs fois (BTreeMap → HashSet → map). Dizaines de milliers d'allocs évitables sur 10k+ fichiers. | `Cow<str>`/références ; construire directement les structures cibles. |
| P10 | Moyen | Fuite possible de thread watcher si `spawn()` échoue en silence | `watcher.rs:49-52,83` | `builder.spawn()` ignoré (l.56) : un échec laisserait un watcher actif sans handler. Le reste de l'init est correct. | Vérifier le résultat de `spawn()` ; ne pas insérer le watcher sinon ; tester enable/disable répété. |
| P11 | Moyen | Calcul du sleep horaire imprécis (soustraction relative) | `scheduler.rs:34` | `diff*60 - now.second()` : granularité minute + clamp peut désaligner le réveil. | Raisonner en secondes époque absolues (`target_epoch - now_epoch`). |
| P12 | Moyen | Tâches planifiées à **00h00** mal détectées (wrap minuit) | `scheduler.rs:47-51` | Fenêtre « minute précédente » n'enveloppe pas 23h59→00h00. | Ajouter `now_min==0 && target_min==1439`, ou secondes époque. |
| P13 | Moyen | Pas de back-pressure / plafond haut sur le scheduler | `scheduler.rs:9,58-89` | `MIN_INTERVAL=5s` ; si une synchro dure plus que l'intervalle, le tick suivant part dès la fin. Canal capacité 16 empile des requêtes. | `MIN_INTERVAL` défaut ↑ (60s) ; back-pressure (doubler si durée>intervalle) ; coalescer les `SyncRequest::Pair`. |
| P14 | Moyen | Aucune incrémentalité disque : `walk_tree` re-stat tout à chaque synchro | `scan.rs`, `plan.rs:30-31`, `archive.rs:184` | `stat()` sur tous les fichiers même watcher actif. Sur sources To, plusieurs minutes. `state.json` mtimes non exploités pour pré-filtrer. | Exploiter `notify` pour re-scanner les sous-arbres modifiés (repli full-scan si incertain). |
| P15 | Moyen | `run_batch` : canal peut empiler des `SyncRequest` redondantes | `scheduler.rs:58-89,197-207` | Exécution **réellement séquentielle** (pas de parallélisme — finding initial « Critique » corrigé). Résidu : pas de déduplication des requêtes en attente. | Dédupliquer/coalescer les `SyncRequest::Pair` pour un même id ; vérifier `last_started`. |
| P16 | Moyen | `refresh()` (getAppState complet) en cascade sans débounce | `store.svelte.ts:99,121-136` | `state:changed`/`sync:finished` → `getAppState()` complet à répétition lors de rafales d'events. | Débouncer `refresh()` (10-50 ms), ou MAJ fine des champs depuis le payload. |
| P17 | Moyen | `filteredPairs` dérivé sans mémoïsation | `routes/+page.svelte:19-28` | `toLowerCase()+filter()` à chaque frappe et à chaque mutation de `store.pairs`. Micro-lags avec 50+ paires ; le drag-drop re-filtre. | Mémoïser `(query, résultat)` ; découpler réordonnancement du filtrage. |
| P18 | Moyen | État local `IntervalPicker` non resynchronisé avec les props | `IntervalPicker.svelte:24-27` | `isDefault/unit/value` initialisés une fois ; un changement de `seconds` par le parent ne se reflète pas (valeur périmée). | `$derived.by()` sur les props, ou `$effect` de resync. |
| P19 | Faible | Buffer CRC 64 Ko réalloué à chaque fichier | `archive_io.rs:41-52` | Pas de réutilisation lors du hachage en série. Mineur. | Buffer réutilisable (thread-local/paramètre). |
| P20 | Faible | `crc32fast` peut-être sans SIMD selon le profil de build | `Cargo.toml:45-46` | Si SSE4.2 non activé → chemin logiciel plus lent sur gros volumes. | Vérifier les opts CPU en release ; benchmarker ; documenter. |
| P21 | Faible | Timers scheduler (3s) et watcher (3s) non coordonnés | `scheduler.rs:11,132`, `watcher.rs:64` | Deux timers 3s en parallèle → petits pics CPU au repos. Impact desktop marginal. | Coordonner via le canal IPC, ou accepter+documenter. |
| P22 | Faible | `setTimeout` des toasts non nettoyé | `store.svelte.ts:81-85` | Timers orphelins après fermeture manuelle (dismiss idempotent → impact mineur). | Stocker les ids et `clearTimeout()`. |
| P23 | Faible | `onscroll` de `VirtualList` sans `{ passive: true }` | `VirtualList.svelte:26-31` | MAJ d'état synchrone à chaque event scroll → fluidité réduite sur longs journaux. | Listener `passive` + coalescer via `requestAnimationFrame`. |

---

## 3. Priorités suggérées

1. **P1/P3/P14** — incrémentalité réelle (le plus gros levier perf sur grosses sources).
2. **P4** — corriger le débounce busy (bug de boucle de synchro temps réel).
3. **S1** — sortir le mot de passe du `settings.json` en clair (keyring/DPAPI) si le chiffrement vise une vraie confidentialité.
4. **S2** — timeout configurable sur les outils externes (fiabilité des destinations réseau).
5. **P5/P16** — fuites/cascades frontend (stabilité UI longue durée).

---

*Audit établi à partir des sources réelles (2026-06-04). Les sévérités sécurité ont été ajustées après vérification adversariale ; voir « Faux positifs » pour les findings écartés en contexte local mono-utilisateur.*
