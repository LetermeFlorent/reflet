# Performance — Reflet

> Document base sur le code reel (`src-tauri/src/sync.rs`, `scheduler.rs`, `config.rs`, `Cargo.toml`).
> Reflet 1.0.0 — app desktop locale, sync miroir unidirectionnelle. Aucun appel reseau, aucun GPU.
> Date : 2026-06-03.

## 1. Resume

Reflet est une application **I/O-bound** : l'essentiel du temps est passe en lecture/ecriture
disque (parcours d'arborescence, copie de fichiers, suppressions). Le seul calcul CPU
potentiellement lourd est le **hachage blake3** de verification par contenu, qui est :

- **optionnel** (desactive par defaut),
- **mono-thread** aujourd'hui,
- domine par le cout I/O (le fichier est entierement lu en memoire avant hachage).

**GPU : non applicable.** Aucune charge de calcul ne justifie une acceleration materielle.

## 2. Ou se trouve le cout

### 2.1 Parcours des arborescences (`walk_tree`, sync.rs:142)

A chaque cycle de synchro, Reflet parcourt **integralement** la source ET la destination
(`build_plan`, sync.rs:247-248) pour construire deux `BTreeMap`. Cout proportionnel au nombre
d'entrees (fichiers + dossiers) de chaque cote.

- Cout : 1 `stat`/`metadata` par entree (sync.rs:180), I/O-bound, mono-thread.
- Sur de tres grandes arborescences (centaines de milliers d'entrees), c'est le poste
  dominant si la verification par contenu est desactivee.
- Pas de cache/index persistant entre cycles : tout est re-parcouru a chaque tick du
  scheduler (intervalle par defaut 900 s, config.rs:65).

### 2.2 Detection de changement (`detect_changed`, sync.rs:210)

Strategie en cascade, du moins cher au plus cher :

1. comparaison de **taille** (gratuit, deja en memoire) ;
2. comparaison **mtime** avec tolerance (`mtime_tolerance_sec`, defaut 2 s, config.rs:78) ;
3. **seulement si** `verify_by_content == "blake3"` : hachage des deux fichiers (source +
   destination).

Tant que `verify_by_content` reste a `"off"` (valeur par defaut, config.rs:77), **aucun
hachage n'est effectue** : la detection se fait uniquement sur taille + mtime, quasi
gratuite.

### 2.3 Hachage blake3 (`blake3_of`, sync.rs:205)

```rust
fn blake3_of(path: &Path) -> Option<[u8; 32]> {
    let bytes = std::fs::read(path).ok()?;        // lit TOUT le fichier en RAM
    Some(*blake3::hash(&bytes).as_bytes())          // hash mono-thread
}
```

Points a noter, tels qu'implementes :

- **Lecture complete en memoire** via `std::fs::read` avant hachage. Pour de gros fichiers,
  cela alloue autant de RAM que la taille du fichier et ne stream pas.
- **Mono-thread** : `blake3::hash` sur tranche unique, sans rayon ni mmap.
- Declenche **deux** lectures completes (source + destination) par fichier suspect, et
  uniquement quand taille identique mais doute (etape 3 de la cascade).
- `blake3 = "1"` dans Cargo.toml (ligne 32), **features par defaut** : ni `rayon` ni `mmap`
  ne sont actives.

### 2.4 Execution du plan (`execute_plan`, sync.rs:475)

- Copies atomiques sequentielles (`for c in &plan.copies`, sync.rs:510) via
  `copy_file_atomic` (copie vers `.synctmp-*` puis `rename`, sync.rs:427). I/O-bound.
- En cas d'echec de copie : 1 retry apres pause de 60 ms (sync.rs:513).
- Suppressions sequentielles, par defaut vers la **corbeille** (crate `trash`, sync.rs:449) ;
  la corbeille est plus lente qu'un `remove` direct mais c'est un choix de securite assume.
- Progression emise tous les `total/50` items (sync.rs:491) pour limiter le trafic IPC.

### 2.5 Modele de concurrence (scheduler.rs)

- **Un seul worker** : `run_sync` tourne dans `spawn_blocking` (scheduler.rs:151), une paire
  a la fois. Les paires dues sont traitees **en serie** (`for pair in due`, scheduler.rs:110).
- Anti-overlap garanti par ce worker unique + flag `sync_busy`.
- `dry_run` passe aussi par `spawn_blocking` (commands.rs:258), donc ne bloque pas l'UI.

> Consequence : la parallelisation **entre paires** n'existe pas (choix de design : worker
> unique, anti-overlap). Toute parallelisation eventuelle se ferait **a l'interieur** d'une
> synchro (hachage et/ou copies).

## 3. Caracteristiques

| Charge                       | Type        | Thread        | GPU |
|------------------------------|-------------|---------------|-----|
| Parcours arbo (`walk_tree`)  | I/O-bound   | mono          | N/A |
| Detection taille/mtime       | CPU trivial | mono          | N/A |
| Hachage blake3 (optionnel)   | CPU + I/O   | mono          | N/A |
| Copies de fichiers           | I/O-bound   | mono (serie)  | N/A |
| Suppressions (corbeille)     | I/O-bound   | mono (serie)  | N/A |

## 4. Pistes d'optimisation (A CONFIRMER avant toute implementation)

> Aucune de ces pistes n'est implementee. Elles sont listees pour decision utilisateur.
> Reflet etant un outil de **sync miroir avec garanties de securite**, la priorite reste
> la correction (pas d'effacement intempestif) avant la vitesse.

### 4.1 Parallelisation CPU du hachage (threads / rayon) — **A CONFIRMER**

Question centrale posee : **autoriser la parallelisation CPU (threads / rayon) du hachage ?**

Deux niveaux possibles, independants :

- **(a) Multi-thread d'un seul gros fichier** : activer la feature `rayon` de blake3
  (`blake3 = { version = "1", features = ["rayon"] }`) + `update_rayon`/`Hasher`. Gain reel
  surtout au-dela de ~1 Mo par fichier, et surtout si le disque (NVMe) n'est pas le goulot.
- **(b) Hacher plusieurs fichiers en parallele** : utiliser un pool (rayon `par_iter` sur
  les `CopyOp` candidats au hachage). Gain si beaucoup de fichiers a verifier en meme temps.

**A CONFIRMER avec l'utilisateur :**
- Autoriser la parallelisation CPU du hachage ? (oui/non)
- Si oui : niveau (a), (b), ou les deux ?
- Plafonner le nombre de threads (ex. `num_cpus - 1`) pour ne pas saturer la machine
  pendant l'usage normal ?
- Conserver le comportement mono-thread par defaut et n'activer le parallelisme que via
  un reglage opt-in dans `settings.json` ?

> Remarque : la verification par contenu est **desactivee par defaut**. La parallelisation
> du hachage ne profite donc qu'aux utilisateurs ayant explicitement active `verify_by_content
> = "blake3"`. A pondererer selon la frequence reelle de ce mode.

### 4.2 Hachage en streaming au lieu de `fs::read` complet — **A CONFIRMER**

Remplacer `std::fs::read` (sync.rs:206) par une lecture par blocs (`std::io::Read` ->
`Hasher::update`) eviterait d'allouer la taille entiere du fichier en RAM. Pertinent pour
de tres gros fichiers. **A CONFIRMER** : utile au vu des tailles de fichiers reellement
synchronisees ? (sinon, complexite inutile)

### 4.3 Parallelisation des copies I/O — **A CONFIRMER, prudence**

Copier plusieurs fichiers en parallele peut accelerer sur NVMe, mais :
- sur HDD ou reseau (SMB), le parallelisme degrade souvent le debit ;
- complique la progression et la gestion d'erreurs/retry (actuellement serie, sync.rs:510).

**A CONFIRMER** : souhaite-t-on tenter cela ? Si oui, le rendre conditionnel au type de
volume serait prudent. Recommandation par defaut : **ne pas** paralleliser les copies.

### 4.4 Index incremental entre cycles — **A CONFIRMER, gros chantier**

Aujourd'hui, source + destination sont re-parcourues integralement a chaque cycle. Un index
persistant (mtime/size par chemin) reduirait le cout du `walk_tree` sur tres grandes arbos,
au prix d'une complexite et de risques de desynchronisation de l'index. **A CONFIRMER** :
hors scope 1.0.0 sauf besoin avere sur de tres gros volumes.

## 5. Recommandations immediates (sans changement de code)

- Laisser `verify_by_content = "off"` (defaut) sauf besoin reel : c'est le levier de perf
  le plus important deja en place.
- Ajuster `mtime_tolerance_sec` (defaut 2 s) plutot que d'activer blake3 si le souci vient
  de horodatages legerement differents apres copie.
- Allonger `interval_sec` (defaut 900 s) sur de gros volumes pour espacer les re-parcours
  complets.

## 6. Points en attente de confirmation (recapitulatif)

| # | Sujet | Statut |
|---|-------|--------|
| 1 | Parallelisation CPU du hachage (threads/rayon), niveau (a)/(b) | **A CONFIRMER** |
| 2 | Plafond de threads (ex. num_cpus-1) | **A CONFIRMER** |
| 3 | Parallelisme opt-in via settings.json | **A CONFIRMER** |
| 4 | Hachage en streaming vs `fs::read` complet | **A CONFIRMER** |
| 5 | Parallelisation des copies I/O (deconseillee par defaut) | **A CONFIRMER** |
| 6 | Index incremental entre cycles | **A CONFIRMER** |
