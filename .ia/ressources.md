# Reflet — Ressources matérielles

Document de référence sur les besoins en ressources de **Reflet**, application
de bureau de synchronisation miroir unidirectionnelle (Tauri 2 + SvelteKit +
Rust). Rédigé à partir des fichiers réels du dépôt (version `1.0.0`).

> **Convention** : tout élément marqué **`A CONFIRMER par l'utilisateur`**
> correspond à une borne matérielle cible non encore validée. Les valeurs
> proposées sont des estimations raisonnables à valider par des tests réels.

---

## 1. Nature de l'application

Reflet est une application **de bureau légère**, **100 % locale**, sans aucun
appel réseau (hors accès aux dossiers réseau montés par l'OS). Son coût en
ressources se résume à trois postes :

1. **WebView2 / WebKitGTK** : le moteur de rendu de l'interface (fourni par
   l'OS, partagé avec le système), pas un Chromium embarqué.
2. **Le processus Rust** (backend Tauri) : moteur de synchronisation + planificateur.
3. **L'I/O disque** lors d'une passe de synchronisation (lecture source,
   écriture destination).

Constat clé tiré du code : **aucun GPU ni VRAM requis**. Il n'y a aucun calcul
lourd côté backend. Le seul traitement intensif possible est le **hachage
BLAKE3** des fichiers, et il est **optionnel** : par défaut `verify_by_content`
vaut `"off"` (cf. `src-tauri/src/config.rs`, `Settings::default`). La détection
de changement se fait alors par **taille** puis **date de modification**
(`src-tauri/src/sync.rs`, `detect_changed`), sans lecture du contenu.

---

## 2. Empreinte réelle observée (faits du dépôt)

| Élément | Valeur | Source |
|---|---|---|
| Binaire release `reflet.exe` | ~12,2 Mo | `src-tauri/target/release/reflet.exe` (mesuré) |
| Installeur NSIS | ~2,7 Mo | `src-tauri/target/release/bundle/` (mesuré) |
| Threads d'exécution | 1 worker de synchro unique | `scheduler.rs` : 1 seul `spawn_blocking`, anti-overlap par `sync_busy` |
| Concurrence de synchro | Aucune (paires traitées en série) | `scheduler.rs` : `run_due`/`run_all` itèrent séquentiellement |
| Journal en mémoire | plafonné à **3000 entrées** (`VecDeque`) | `state.rs` : `MAX_LOGS = 3000` |
| Runtime async | Tokio en config minimale (`time`, `sync`, `rt`, `macros`) | `src-tauri/Cargo.toml` |
| Persistance config | un seul `settings.json` (JSON) | `config.rs` : `%APPDATA%\com.reflet.desktop\settings.json` |
| Logs fichier | rotation **quotidienne** `reflet.log` | `logging.rs` : `tracing_appender::rolling::daily` |

### Implications mémoire
- L'arbre source **et** l'arbre destination sont chargés intégralement en RAM
  pendant la construction du plan : deux `BTreeMap<String, Entry>`
  (`sync.rs`, `walk_tree` + `build_plan`). La consommation RAM **croît avec le
  nombre d'entrées (fichiers + dossiers)** dans les arbres, pas avec leur
  taille totale en octets. Chaque `Entry` retient le chemin relatif, le chemin
  absolu, taille, mtime et un drapeau dossier.
- La liste des opérations (`copies`, `deletes`, `create_dirs`) est elle aussi
  matérialisée en mémoire avant exécution.
- Conséquence : le facteur de dimensionnement mémoire est le **nombre de
  fichiers à synchroniser**, pas leur volume.

### Implications CPU
- Mono-thread effectif pour la synchro : un seul fichier copié à la fois
  (`execute_plan` boucle séquentielle). Donc **1 cœur logique suffit**
  fonctionnellement ; les cœurs supplémentaires n'accélèrent pas une passe.
- Copie atomique via fichier temporaire puis `rename` (`copy_file_atomic`),
  donc pic d'usage **disque** plutôt que CPU.
- Si `verify_by_content = "blake3"` est activé, chaque fichier candidat est lu
  **intégralement deux fois** (source + destination) et haché : ce mode est le
  seul à pouvoir charger le CPU et l'I/O de façon notable.

### Implications disque
- **Espace temporaire** : la copie crée un fichier `.synctmp-<pid>-<n>` à côté
  du fichier de destination (`unique_tmp`). Il faut donc, côté destination,
  l'espace libre du **plus gros fichier en cours de copie** en plus du fichier
  final, transitoirement.
- Espace pour l'installeur + binaire : ordre de grandeur **< 50 Mo** une fois
  installé (hors WebView2 fourni par l'OS).

---

## 3. Bornes matérielles cibles (A CONFIRMER)

> Ces seuils sont des **propositions** cohérentes avec l'architecture observée.
> Aucun n'est garanti tant qu'il n'a pas été validé par l'utilisateur via des
> tests sur des configurations réelles.

### 3.1 RAM
- **Minimum proposé** : **4 Go** de RAM système — **`A CONFIRMER par l'utilisateur`**
  (contrainte dominée par WebView2 + l'OS, pas par Reflet lui-même).
- **Empreinte process Reflet au repos** : faible (quelques dizaines de Mo
  attendus, à mesurer) — **`A CONFIRMER par l'utilisateur`**.
- **Surcoût par passe de synchro** : proportionnel au nombre d'entrées des
  arbres source + destination. Borne haute du nombre de fichiers supportés
  sans dégradation — **`A CONFIRMER par l'utilisateur`** (ex. cible 100 000 /
  500 000 / 1 000 000 d'entrées à valider).

### 3.2 CPU / threads
- **Minimum proposé** : **CPU x86-64 (64 bits), 1 cœur logique** suffisant —
  **`A CONFIRMER par l'utilisateur`**.
- **Cible recommandée** : 2 cœurs (un pour l'UI/WebView, un pour la synchro) —
  **`A CONFIRMER par l'utilisateur`**.
- Note : la synchro étant séquentielle (1 worker), un CPU plus rapide réduit la
  durée d'une passe ; davantage de cœurs n'apporte pas de parallélisme de
  synchro dans l'implémentation actuelle.
- Architecture ARM64 Windows — **`A CONFIRMER par l'utilisateur`** (le binaire
  publié est `x64` d'après le nom d'installeur NSIS `..._x64-setup.exe`).

### 3.3 Disque (SSD / HDD)
- **Espace requis pour l'installation** : ordre de grandeur **< 50 Mo** ;
  borne exacte à figer — **`A CONFIRMER par l'utilisateur`**.
- **Espace temporaire côté destination** : prévoir au moins la taille du plus
  gros fichier copié (fichier `.synctmp` transitoire) — seuil de marge à fixer
  — **`A CONFIRMER par l'utilisateur`**.
- **SSD vs HDD** : Reflet fonctionne sur les deux ; le débit de copie et le
  temps de parcours d'arborescence dépendent fortement du support. SSD
  recommandé pour de très gros arbres — **`A CONFIRMER par l'utilisateur`**.
- **Croissance des logs** : `reflet.log` en rotation quotidienne, plafond de
  rétention / purge automatique — **`A CONFIRMER par l'utilisateur`** (le code
  ne supprime pas les anciens fichiers de log).

### 3.4 Système d'exploitation
- **Windows** : versions cibles supportées (ex. Windows 10 / Windows 11) —
  **`A CONFIRMER par l'utilisateur`**.
  - Dépendance **WebView2** : préinstallé sur Windows 11 (cf. README) ; sur
    Windows 10, présence/installation du runtime WebView2 à valider —
    **`A CONFIRMER par l'utilisateur`**.
  - Version minimale exacte de Windows 10 supportée — **`A CONFIRMER par l'utilisateur`**.
- **Linux** (le bundle cible aussi `.deb` / `.rpm`) : si Linux est dans le
  périmètre des cibles, distributions et versions supportées —
  **`A CONFIRMER par l'utilisateur`** (dépendances runtime : `libwebkit2gtk-4.1-0`,
  `libgtk-3-0`, `libayatana-appindicator3-1`, cf. `tauri.conf.json` et README).
- **Bits** : 64 bits requis (binaire x64).

### 3.5 Réseau (synchro de dossiers réseau)
- Reflet ne fait **aucun appel réseau applicatif** : il lit/écrit via le système
  de fichiers. La synchro « réseau » se fait donc sur des **chemins UNC** ou des
  **lecteurs réseau montés** par l'OS.
- Élément technique confirmé par le code : les chemins UNC (`\\serveur\partage`)
  sont gérés — `verbatim()` (`sync.rs`) préserve le préfixe `\\` au lieu d'y
  préfixer `\\?\`.
- **Bornes à confirmer** :
  - Débit / latence réseau minimal recommandé pour une synchro fluide —
    **`A CONFIRMER par l'utilisateur`**.
  - Comportement et tolérance en cas de partage réseau lent/intermittent
    (le code fait **un seul retry** après 60 ms en cas d'échec de copie,
    `execute_plan`) — adéquation à valider — **`A CONFIRMER par l'utilisateur`**.
  - Protocoles testés (SMB/CIFS, NFS monté, etc.) — **`A CONFIRMER par l'utilisateur`**.
  - Le parcours d'arborescence ignore les points de reparse / liens
    symboliques (`is_reparse_point`, `follow_links(false)`) : impact sur
    certains montages réseau à valider — **`A CONFIRMER par l'utilisateur`**.

---

## 4. Facteurs qui font varier la consommation

| Levier (réglage réel) | Effet sur les ressources | Réf. |
|---|---|---|
| `verify_by_content = "blake3"` | Lecture intégrale + hachage de chaque fichier candidat → I/O et CPU fortement accrus | `sync.rs` `detect_changed`, `blake3_of` |
| `interval_sec` / `interval_sec_override` | Fréquence des passes (donc fréquence des pics I/O). Borné : min 5 s, sommeil planificateur 3–1800 s | `scheduler.rs` (`MIN_INTERVAL`, `MIN_SLEEP`, `MAX_SLEEP`) |
| Nombre de paires activées | Passes séquentielles : plus de paires = plus longtemps occupé (pas plus de RAM simultanée) | `scheduler.rs` `run_all`/`run_due` |
| Taille des arborescences (nb de fichiers/dossiers) | RAM du plan (2 `BTreeMap`) et durée du parcours | `sync.rs` `walk_tree`, `build_plan` |
| `ignore_patterns` (glob) | Réduit le nombre d'entrées traitées → moins de RAM/CPU/I-O | `sync.rs` `build_globset`, `walk_tree` |
| `delete_behavior = "trash"` (défaut) | Suppression via corbeille (crate `trash`) : coût I/O dépend de l'implémentation OS de la corbeille | `sync.rs` `safe_delete` |

---

## 5. Synthèse

- Reflet est **léger** : binaire ~12 Mo, installeur ~2,7 Mo, **aucun GPU/VRAM**,
  synchro mono-worker.
- Le **vrai facteur de dimensionnement** est le **nombre de fichiers** à
  parcourir (RAM du plan), suivi du **support disque** et, si activé, du
  **mode BLAKE3** (CPU + I/O).
- Toutes les **bornes matérielles cibles** (RAM minimale, OS supportés,
  exigences réseau, espace disque, nombre maximal de fichiers) sont à ce stade
  des **propositions à valider** : elles sont marquées
  **`A CONFIRMER par l'utilisateur`** dans la section 3.
