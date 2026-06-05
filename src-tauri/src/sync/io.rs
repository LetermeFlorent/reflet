use super::util::verbatim;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_tmp(dst: &Path) -> PathBuf {
    let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let name = format!(".synctmp-{pid}-{n}");
    match dst.parent() {
        Some(p) => p.join(name),
        None => PathBuf::from(name),
    }
}

pub(super) fn copy_file_atomic(src: &Path, dst: &Path) -> std::io::Result<()> {
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(verbatim(parent))?;
    }
    let tmp = unique_tmp(dst);
    let (vsrc, vtmp, vdst) = (verbatim(src), verbatim(&tmp), verbatim(dst));
    std::fs::copy(&vsrc, &vtmp)?;
    if let Ok(meta) = std::fs::metadata(&vsrc) {
        let mt = filetime::FileTime::from_last_modification_time(&meta);
        let _ = filetime::set_file_mtime(&vtmp, mt);
    }
    match std::fs::rename(&vtmp, &vdst) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&vtmp);
            Err(e)
        }
    }
}

pub(super) fn safe_delete(abs: &Path, behavior: &str) -> Result<(), String> {
    if behavior == "trash" {
        if trash::delete(abs).is_ok() {
            return Ok(());
        }
        // La corbeille refuse les noms à point/espace de fin (« fichier introuvable »).
        // Repli : suppression définitive via chemin verbatim \\?\, sinon ces entrées
        // échouent à chaque passe et bloquent le nettoyage du miroir.
        remove_verbatim(abs)
    } else {
        remove_verbatim(abs)
    }
}

/// Suppression via chemin verbatim \\?\ (gère noms longs et points/espaces de fin).
fn remove_verbatim(abs: &Path) -> Result<(), String> {
    let v = verbatim(abs);
    let is_dir = std::fs::symlink_metadata(&v).map(|m| m.is_dir()).unwrap_or(false);
    if is_dir {
        std::fs::remove_dir_all(&v).map_err(|e| e.to_string())
    } else {
        std::fs::remove_file(&v).map_err(|e| e.to_string())
    }
}
