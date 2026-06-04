use super::archive_io::{archive_path, crc32_of, open_shared, read_index, verbatim, Index, STATE_ENTRY};
use super::scan::{build_globset, walk_tree};
use super::types::{PlanItem, SyncOutcome, SyncPlan};
use super::util::log;
use crate::config::{Settings, SyncPair};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};
use zip::write::SimpleFileOptions;

/// Signature métadonnées d'un jeu de fichiers source (chemins triés + tailles + mtimes).
/// Sert à détecter qu'une archive de format externe est déjà à jour : même cohérence que
/// le fast-path taille+mtime du ZIP intégré (ne détecte pas une modif à taille+mtime
/// identiques — comportement assumé, identique au reste de l'app hors verify_by_content).
fn ext_signature(files: &[(String, PathBuf, u64, i64)]) -> String {
    let mut refs: Vec<&(String, PathBuf, u64, i64)> = files.iter().collect();
    refs.sort_by(|a, b| a.0.cmp(&b.0));
    let mut h = blake3::Hasher::new();
    for (rel, _, size, mtime) in refs {
        h.update(rel.as_bytes());
        h.update(&size.to_le_bytes());
        h.update(&mtime.to_le_bytes());
        h.update(b"\n");
    }
    h.finalize().to_hex().to_string()
}

/// Cache de signature (hors destination, pour ne pas interférer avec le nettoyage qui ne
/// garde que l'archive) : `app_config_dir/.ext-cache/{pair_id}.sig`.
fn ext_sig_path(app: &AppHandle, pair_id: &str) -> Option<PathBuf> {
    let dir = app.path().app_config_dir().ok()?.join(".ext-cache");
    Some(dir.join(format!("{pair_id}.sig")))
}
fn read_ext_sig(app: &AppHandle, pair_id: &str) -> Option<String> {
    std::fs::read_to_string(ext_sig_path(app, pair_id)?).ok()
}
fn write_ext_sig(app: &AppHandle, pair_id: &str, sig: &str) {
    if let Some(p) = ext_sig_path(app, pair_id) {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, sig);
    }
}

enum Change {
    New,
    Changed,
    Unchanged,
}

/// Codec interne de l'entrée selon la méthode choisie (le conteneur reste un ZIP).
fn zip_options(method: &str, level: u32) -> SimpleFileOptions {
    use zip::CompressionMethod as M;
    let base = SimpleFileOptions::default();
    match method {
        "store" => base.compression_method(M::Stored),
        "bzip2" => base
            .compression_method(M::Bzip2)
            .compression_level(Some(level.clamp(1, 9) as i64)),
        "zstd" => base
            .compression_method(M::Zstd)
            .compression_level(Some(level.clamp(1, 22) as i64)),
        _ => base
            .compression_method(M::Deflated)
            .compression_level(Some(level.clamp(0, 9) as i64)),
    }
}

/// Détection : fast-path par taille + mtime enregistré ; sinon on confirme par CRC.
/// `verify` (réglage verify_by_content) force le CRC même quand taille+mtime collent.
fn classify(rel: &str, abs: &Path, size: u64, mtime: i64, idx: &Index, tol: i64, verify: bool) -> Change {
    match idx.entries.get(rel) {
        None => Change::New,
        Some(&(crc, esize)) => {
            if !verify {
                let saved = idx.mtimes.get(rel).copied();
                if size == esize && saved.map(|m| (mtime - m).abs() <= tol).unwrap_or(false) {
                    return Change::Unchanged;
                }
            }
            match crc32_of(abs) {
                Ok(c) if c == crc && size == esize => Change::Unchanged,
                _ => Change::Changed,
            }
        }
    }
}

/// Recopie l'entrée précédente (octets compressés) depuis l'ancienne archive, pour
/// ne jamais perdre une version déjà sauvegardée quand la source est momentanément illisible.
fn carry_forward(
    zw: &mut zip::ZipWriter<io::BufWriter<File>>,
    old: Option<&mut zip::ZipArchive<File>>,
    rel: &str,
) -> bool {
    if let Some(z) = old {
        if let Ok(f) = z.by_name(rel) {
            return zw.raw_copy_file(f).is_ok();
        }
    }
    false
}

fn source_files(pair: &SyncPair, settings: &Settings) -> Vec<(String, PathBuf, u64, i64)> {
    let mut patterns = settings.ignore_patterns.clone();
    patterns.extend(pair.ignore_patterns.clone());
    let ignore = build_globset(&patterns);
    let map = walk_tree(Path::new(&pair.source), &ignore);
    let (min, max) = (pair.min_file_size, pair.max_file_size);
    map.into_values()
        .filter(|e| !e.is_dir)
        .filter(|e| (min == 0 || e.size >= min) && (max == 0 || e.size <= max))
        .map(|e| (e.rel, e.abs, e.size, e.mtime))
        .collect()
}

fn item(rel: &str, size: u64, reason: &str) -> PlanItem {
    PlanItem {
        rel: rel.to_string(),
        is_dir: false,
        size,
        reason: reason.to_string(),
    }
}

/// Ajoute au plan les fichiers de la destination que le nettoyage supprimerait (tout sauf l'archive).
fn push_dest_cleanup(plan: &mut SyncPlan, dest: &Path, arc: &Path) {
    let keep = arc.file_name();
    if let Ok(rd) = std::fs::read_dir(verbatim(dest)) {
        for ent in rd.flatten() {
            if Some(ent.file_name().as_os_str()) == keep {
                continue;
            }
            plan.to_delete.push(item(&ent.file_name().to_string_lossy(), 0, "nettoyage destination"));
        }
    }
}

/// Aperçu (dry-run) : ce que la prochaine sauvegarde changerait dans l'archive ET
/// ce que le nettoyage retirerait de la destination, plus l'état de sécurité.
pub fn plan_archive(pair: &SyncPair, settings: &Settings) -> SyncPlan {
    let arc = archive_path(pair);
    let dest = Path::new(&pair.destination);

    // Formats externes : reconstruction complète, on liste tout + le nettoyage destination.
    if !crate::compression::is_builtin(&pair.compression.method) {
        let mut plan = SyncPlan::default();
        for (rel, _, size, _) in source_files(pair, settings) {
            plan.to_overwrite.push(item(&rel, size, "archive (reconstruction complète)"));
        }
        push_dest_cleanup(&mut plan, dest, &arc);
        plan.total_bytes = plan.to_overwrite.iter().map(|i| i.size).sum();
        return plan;
    }

    let idx = read_index(&arc);
    let files = source_files(pair, settings);
    let tol = settings.mtime_tolerance_sec;
    let verify = settings.verify_by_content != "off";
    let mut plan = SyncPlan::default();
    let mut present = HashSet::new();
    for (rel, abs, size, mtime) in &files {
        present.insert(rel.clone());
        match classify(rel, abs, *size, *mtime, &idx, tol, verify) {
            Change::New => plan.to_copy.push(item(rel, *size, "nouveau")),
            Change::Changed => plan.to_overwrite.push(item(rel, *size, "modifié")),
            Change::Unchanged => {}
        }
    }
    for name in idx.entries.keys() {
        if !present.contains(name) {
            plan.to_delete.push(item(name, 0, "retiré de l'archive"));
        }
    }
    push_dest_cleanup(&mut plan, dest, &arc);
    let prev = idx.entries.len();
    if prev > 0 {
        let removed = idx.entries.keys().filter(|k| !present.contains(*k)).count();
        plan.delete_pct = (removed as u64 * 100 / prev as u64) as u32;
        let t = settings.delete_safety_threshold_pct;
        plan.aborted_safety = t < 100 && plan.delete_pct > t;
    }
    plan.total_bytes = plan.to_copy.iter().chain(&plan.to_overwrite).map(|i| i.size).sum();
    plan
}

pub fn run_archive_sync(app: &AppHandle, pair: &SyncPair, settings: &Settings) -> SyncOutcome {
    let mut out = SyncOutcome::default();
    let dest = Path::new(&pair.destination);
    if let Err(e) = std::fs::create_dir_all(verbatim(dest)) {
        out.errors += 1;
        log(app, &pair.id, "error", "error", None, format!("création destination échec : {e}"));
        return out;
    }
    let arc = archive_path(pair);

    // Formats externes (7z, tar.*) : reconstruction complète via l'outil.
    if !crate::compression::is_builtin(&pair.compression.method) {
        return run_external(app, pair, settings, dest, &arc);
    }

    let arc_exists = arc.exists();
    let idx = read_index(&arc);

    // Une archive présente mais illisible (verrouillée/corrompue) ne doit JAMAIS être
    // écrasée : on annule plutôt que de remplacer une sauvegarde par une vide.
    if arc_exists && !idx.readable {
        out.aborted_safety = true;
        let msg = "Archive existante illisible (verrouillée ou corrompue) — synchro annulée pour ne pas l'écraser.".to_string();
        log(app, &pair.id, "error", "error", None, msg.clone());
        let _ = app.emit("app:error", json!({ "pairId": pair.id, "message": msg }));
        return out;
    }

    let files = source_files(pair, settings);
    let tol = settings.mtime_tolerance_sec;
    let verify = settings.verify_by_content != "off";
    let total = files.len();

    let present: HashSet<String> = files.iter().map(|(rel, ..)| rel.clone()).collect();

    // Sécurité anti-wipe : si la source paraît vidée vs l'archive existante (lecteur
    // démonté, dossier temporairement vide…), on n'écrase pas l'archive ni la destination.
    let prev = idx.entries.len();
    if prev > 0 {
        let removed = idx.entries.keys().filter(|k| !present.contains(*k)).count();
        let pct = (removed as u64 * 100 / prev as u64) as u32;
        let threshold = settings.delete_safety_threshold_pct;
        if threshold < 100 && pct > threshold {
            out.aborted_safety = true;
            let msg = format!(
                "Sauvegarde RETENUE : {pct}% du contenu de l'archive ({removed}/{prev}) disparaîtrait, seuil {threshold}%. Vérifie la source puis relance.",
            );
            log(app, &pair.id, "warn", "skip", None, msg.clone());
            let _ = app.emit("app:error", json!({ "pairId": pair.id, "message": msg }));
            return out;
        }
    }

    let _ = app.emit(
        "sync:started",
        json!({ "pairId": pair.id, "name": pair.name, "totalFiles": total, "totalBytes": 0 }),
    );

    let tmp = dest.join(format!(".reflet-arc-{}.tmp", std::process::id()));
    let outfile = match File::create(verbatim(&tmp)) {
        Ok(f) => f,
        Err(e) => {
            out.errors += 1;
            log(app, &pair.id, "error", "error", None, format!("création archive échec : {e}"));
            return out;
        }
    };
    let mut zw = zip::ZipWriter::new(io::BufWriter::new(outfile));
    let pwd = pair.compression.password.as_deref().map(str::trim).filter(|p| !p.is_empty());
    let base_opts = zip_options(&pair.compression.method, pair.compression.level);
    let opts = match pwd {
        Some(p) => base_opts.with_aes_encryption(zip::AesMode::Aes256, p),
        None => base_opts,
    };

    // Archive chiffrée : on ne recopie PAS les octets bruts de l'ancienne archive
    // (raw_copy_file recopierait des entrées chiffrées telles quelles, incohérent
    // si le mot de passe change). On rechiffre donc tout le contenu à chaque passe.
    let mut old = if pwd.is_some() {
        None
    } else {
        open_shared(&arc).ok().and_then(|f| zip::ZipArchive::new(f).ok())
    };
    let mut new_mtimes: HashMap<String, i64> = HashMap::new();
    let step = (total / 50).max(1);

    for (i, (rel, abs, size, mtime)) in files.iter().enumerate() {
        let change = classify(rel, abs, *size, *mtime, &idx, tol, verify);
        let mut wrote = false;

        if matches!(change, Change::Unchanged) && carry_forward(&mut zw, old.as_mut(), rel) {
            new_mtimes.insert(rel.clone(), *mtime);
            wrote = true;
        }

        if !wrote {
            match open_shared(abs) {
                Ok(mut rf) => {
                    let ok = zw.start_file(rel.as_str(), opts).is_ok() && io::copy(&mut rf, &mut zw).is_ok();
                    if ok {
                        if matches!(change, Change::Changed) {
                            out.updated += 1;
                            log(app, &pair.id, "info", "update", Some(rel.clone()), "mis à jour dans l'archive".into());
                        } else {
                            out.copied += 1;
                            log(app, &pair.id, "info", "copy", Some(rel.clone()), "ajouté à l'archive".into());
                        }
                        new_mtimes.insert(rel.clone(), *mtime);
                    } else {
                        // Écriture partielle : on retire l'entrée tronquée, puis on tente de
                        // conserver la version précédente plutôt que de la perdre.
                        let _ = zw.abort_file();
                        out.errors += 1;
                        if carry_forward(&mut zw, old.as_mut(), rel) {
                            new_mtimes.insert(rel.clone(), *mtime);
                            log(app, &pair.id, "error", "error", Some(rel.clone()), "écriture échec — version précédente conservée".into());
                        } else {
                            log(app, &pair.id, "error", "error", Some(rel.clone()), "écriture dans l'archive échec".into());
                        }
                    }
                }
                Err(_) if !abs.exists() => {
                    log(app, &pair.id, "warn", "skip", Some(rel.clone()), "ignoré (source introuvable)".into());
                }
                Err(e) => {
                    if carry_forward(&mut zw, old.as_mut(), rel) {
                        new_mtimes.insert(rel.clone(), *mtime);
                        log(app, &pair.id, "warn", "skip", Some(rel.clone()), "source illisible — version précédente conservée".into());
                    } else {
                        out.errors += 1;
                        log(app, &pair.id, "error", "error", Some(rel.clone()), format!("lecture source échec : {e}"));
                    }
                }
            }
        }

        if (i + 1) % step == 0 {
            let _ = app.emit("sync:progress", json!({ "pairId": pair.id, "done": i + 1, "total": total }));
        }
    }

    for name in idx.entries.keys() {
        if !present.contains(name) {
            out.deleted += 1;
            log(app, &pair.id, "info", "delete", Some(name.clone()), "retiré de l'archive".into());
        }
    }

    if let Ok(js) = serde_json::to_string(&new_mtimes) {
        let so = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        // Chiffré comme le reste de l'archive pour ne pas exposer l'index des chemins en clair.
        let so = match pwd {
            Some(p) => so.with_aes_encryption(zip::AesMode::Aes256, p),
            None => so,
        };
        if zw.start_file(STATE_ENTRY, so).is_ok() {
            let _ = zw.write_all(js.as_bytes());
        }
    }

    drop(old);

    if let Err(e) = zw.finish() {
        out.errors += 1;
        log(app, &pair.id, "error", "error", None, format!("finalisation archive échec : {e}"));
        let _ = std::fs::remove_file(verbatim(&tmp));
        return out;
    }
    if let Err(e) = std::fs::rename(verbatim(&tmp), verbatim(&arc)) {
        out.errors += 1;
        log(app, &pair.id, "error", "error", None, format!("remplacement archive échec : {e}"));
        let _ = std::fs::remove_file(verbatim(&tmp));
        return out;
    }

    let _ = app.emit("sync:progress", json!({ "pairId": pair.id, "done": total, "total": total }));

    // Ne nettoyer la destination que si TOUT a été archivé sans erreur, sinon on
    // risque de supprimer la copie de secours d'un fichier qui n'a pas pu être archivé.
    if out.errors == 0 {
        cleanup_dest(app, &pair.id, dest, &arc, settings, &mut out);
    } else {
        log(app, &pair.id, "warn", "skip", None, "nettoyage destination ignoré (des fichiers ont échoué) — copies existantes conservées".into());
    }
    out
}

/// Formats externes : on reconstruit l'archive entière via l'outil (pas d'incrémental),
/// dans un fichier temporaire, puis remplacement atomique + nettoyage gardé.
fn run_external(app: &AppHandle, pair: &SyncPair, settings: &Settings, dest: &Path, arc: &Path) -> SyncOutcome {
    let mut out = SyncOutcome::default();
    let files = source_files(pair, settings);
    let total = files.len();
    let arc_exists = arc.exists();

    if total == 0 && arc_exists {
        out.aborted_safety = true;
        let msg = "Source vide alors qu'une archive existe — sauvegarde retenue (vérifie la source).".to_string();
        log(app, &pair.id, "warn", "skip", None, msg.clone());
        let _ = app.emit("app:error", json!({ "pairId": pair.id, "message": msg }));
        return out;
    }

    // Évite de TOUT recompresser quand la source n'a pas bougé (les formats externes
    // reconstruisent l'archive entière ; sur le scheduler auto ça recompresserait des Go
    // pour 0 changement). On compare une signature métadonnées au dernier build réussi.
    let sig = ext_signature(&files);
    if arc_exists && read_ext_sig(app, &pair.id).as_deref() == Some(sig.as_str()) {
        log(app, &pair.id, "info", "skip", None,
            "archive externe déjà à jour (aucun changement détecté) — reconstruction ignorée".into());
        cleanup_dest(app, &pair.id, dest, arc, settings, &mut out);
        return out;
    }

    let _ = app.emit(
        "sync:started",
        json!({ "pairId": pair.id, "name": pair.name, "totalFiles": total, "totalBytes": 0 }),
    );

    let tmp = dest.join(format!(".reflet-arc-{}.tmp", std::process::id()));
    let _ = std::fs::remove_file(verbatim(&tmp));
    let pwd = pair.compression.password.as_deref().map(str::trim).filter(|p| !p.is_empty());
    if let Err(e) = crate::compression::build_external(
        &pair.compression.method,
        pair.compression.level,
        pwd,
        Path::new(&pair.source),
        &tmp,
    ) {
        out.errors += 1;
        log(app, &pair.id, "error", "error", None, format!("compression échec : {e}"));
        let _ = std::fs::remove_file(verbatim(&tmp));
        return out;
    }
    if let Err(e) = std::fs::rename(verbatim(&tmp), verbatim(arc)) {
        out.errors += 1;
        log(app, &pair.id, "error", "error", None, format!("remplacement archive échec : {e}"));
        let _ = std::fs::remove_file(verbatim(&tmp));
        return out;
    }
    write_ext_sig(app, &pair.id, &sig);

    out.copied = total as u64;
    log(app, &pair.id, "info", "copy", None, format!("archive {} recréée ({total} fichiers)", pair.compression.method));
    let _ = app.emit("sync:progress", json!({ "pairId": pair.id, "done": total, "total": total }));

    cleanup_dest(app, &pair.id, dest, arc, settings, &mut out);
    out
}

/// La destination ne doit contenir QUE l'archive : on supprime le reste (anciens .zip
/// par-fichier, copies brutes pré-archive…). Garde-fou anti-wipe : si plus de `seuil %`
/// des entrées seraient supprimées (destination mal pointée), on RETIENT le nettoyage.
fn cleanup_dest(app: &AppHandle, pair_id: &str, dest: &Path, arc: &Path, settings: &Settings, out: &mut SyncOutcome) {
    let keep = arc.file_name();
    let entries: Vec<PathBuf> = match std::fs::read_dir(verbatim(dest)) {
        Ok(rd) => rd.flatten().map(|e| e.path()).collect(),
        Err(e) => {
            out.errors += 1;
            log(app, pair_id, "warn", "skip", None, format!("nettoyage destination ignoré : lecture impossible ({e})"));
            return;
        }
    };
    let total = entries.len().max(1);
    let to_delete: Vec<&PathBuf> = entries
        .iter()
        .filter(|p| p.file_name() != keep)
        .collect();
    if to_delete.is_empty() {
        return;
    }
    let pct = (to_delete.len() as u64 * 100 / total as u64) as u32;
    let threshold = settings.delete_safety_threshold_pct;
    if threshold < 100 && pct > threshold {
        out.aborted_safety = true;
        let msg = format!(
            "Nettoyage destination RETENU : {pct}% des fichiers ({}/{total}) seraient supprimés, seuil {threshold}%. Augmente le seuil (Réglages) ou vide le dossier toi-même si c'est voulu.",
            to_delete.len()
        );
        log(app, pair_id, "warn", "skip", None, msg.clone());
        let _ = app.emit("app:error", json!({ "pairId": pair_id, "message": msg }));
        return;
    }
    for p in to_delete {
        match remove_with_fallback(p, &settings.delete_behavior) {
            Ok(()) => out.deleted += 1,
            Err(e) => {
                out.errors += 1;
                log(app, pair_id, "error", "error", Some(p.to_string_lossy().to_string()), format!("nettoyage destination échec : {e}"));
            }
        }
    }
}

/// Supprime via la corbeille ; en repli (noms à point/espace final que la corbeille
/// refuse), suppression définitive via chemin verbatim \\?\.
fn remove_with_fallback(p: &Path, behavior: &str) -> Result<(), String> {
    if super::io::safe_delete(p, behavior).is_ok() {
        return Ok(());
    }
    let v = verbatim(p);
    let is_dir = std::fs::symlink_metadata(&v).map(|m| m.is_dir()).unwrap_or(false);
    if is_dir {
        std::fs::remove_dir_all(&v).map_err(|e| e.to_string())
    } else {
        std::fs::remove_file(&v).map_err(|e| e.to_string())
    }
}
