pub(super) use super::util::verbatim;
use crate::config::SyncPair;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

/// Entrée interne stockée dans l'archive : mtimes source au moment de la sauvegarde.
pub(super) const STATE_ENTRY: &str = ".reflet/state.json";

/// Ouvre en lecture SANS verrou exclusif (FILE_SHARE_READ|WRITE|DELETE) et via \\?\
/// pour gérer les noms finissant par « . »/espace et les chemins longs.
pub(super) fn open_shared(p: &Path) -> io::Result<File> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0x7)
            .open(verbatim(p))
    }
    #[cfg(not(windows))]
    {
        File::open(p)
    }
}

pub(super) fn crc32_of(p: &Path) -> io::Result<u32> {
    let mut f = open_shared(p)?;
    let mut h = crc32fast::Hasher::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize())
}

/// Nettoie un nom d'archive (caractères interdits Windows + points/espaces de fin).
pub(super) fn sanitize(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| if "\\/:*?\"<>|".contains(c) { '_' } else { c })
        .collect();
    let trimmed = cleaned.trim_end_matches(['.', ' ']).trim();
    if trimmed.is_empty() {
        "archive".to_string()
    } else {
        trimmed.to_string()
    }
}

pub(super) fn archive_path(pair: &SyncPair) -> PathBuf {
    let raw = if pair.compression.archive_name.trim().is_empty() {
        pair.name.as_str()
    } else {
        pair.compression.archive_name.as_str()
    };
    let ext = crate::compression::archive_extension(&pair.compression.method);
    Path::new(&pair.destination).join(format!("{}{ext}", sanitize(raw)))
}

/// Index lu depuis l'archive existante = « manifeste » sans re-scanner le contenu :
/// le répertoire central du zip donne crc+taille par entrée, `.reflet/state.json`
/// donne les mtimes source enregistrés à la dernière sauvegarde.
pub(super) struct Index {
    pub entries: HashMap<String, (u32, u64)>,
    pub mtimes: HashMap<String, i64>,
    /// false UNIQUEMENT si l'archive existe sur disque mais n'a pu être ouverte/lue
    /// (verrouillée, corrompue) — pour ne jamais l'écraser par erreur.
    pub readable: bool,
}

pub(super) fn read_index(arc: &Path) -> Index {
    let mut idx = Index {
        entries: HashMap::new(),
        mtimes: HashMap::new(),
        readable: true,
    };
    let exists = arc.exists();
    let Ok(file) = open_shared(arc) else {
        idx.readable = !exists;
        return idx;
    };
    let Ok(mut zip) = zip::ZipArchive::new(file) else {
        idx.readable = false;
        return idx;
    };
    // Noms lus depuis le répertoire central : disponibles même pour des entrées
    // chiffrées (AES) que `by_index` pourrait refuser de lire.
    let names: Vec<String> = zip.file_names().map(|s| s.to_string()).collect();
    for i in 0..zip.len() {
        let Ok(mut e) = zip.by_index(i) else {
            continue;
        };
        let name = e.name().to_string();
        if name == STATE_ENTRY {
            let mut s = String::new();
            if e.read_to_string(&mut s).is_ok() {
                if let Ok(m) = serde_json::from_str::<HashMap<String, i64>>(&s) {
                    idx.mtimes = m;
                }
            }
            continue;
        }
        if e.is_dir() {
            continue;
        }
        idx.entries.insert(name, (e.crc32(), e.size()));
    }
    // Archive chiffrée : `by_index` n'a rien renvoyé pour les entrées chiffrées, mais
    // l'anti-wipe a besoin de connaître leurs noms (comparaison par présence). On les
    // ajoute (crc/size inconnus = 0, ce qui force une recompression, cohérent).
    for name in names {
        if name == STATE_ENTRY || name.ends_with('/') {
            continue;
        }
        idx.entries.entry(name).or_insert((0, 0));
    }
    idx
}
