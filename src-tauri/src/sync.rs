use crate::config::{Settings, SyncPair};
use crate::state::{AppState, LogEntry};
use chrono::Utc;
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::UNIX_EPOCH;
use tauri::{AppHandle, Emitter, Manager};
use walkdir::WalkDir;

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
struct Entry {
    rel: String,
    is_dir: bool,
    size: u64,
    mtime: i64,
    abs: PathBuf,
}

struct CopyOp {
    rel: String,
    src_abs: PathBuf,
    dst_abs: PathBuf,
    size: u64,
    reason: String,
    is_new: bool,
}

struct DeleteOp {
    rel: String,
    abs: PathBuf,
    is_dir: bool,
}

struct ExecPlan {
    create_dirs: Vec<(String, PathBuf)>,
    copies: Vec<CopyOp>,
    deletes: Vec<DeleteOp>,
    total_bytes: u64,
    extra_entries: usize,
    dest_total: usize,
    delete_pct: u32,
    aborted_safety: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlanItem {
    pub rel: String,
    pub is_dir: bool,
    pub size: u64,
    pub reason: String,
}

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncPlan {
    pub to_create_dir: Vec<String>,
    pub to_copy: Vec<PlanItem>,
    pub to_overwrite: Vec<PlanItem>,
    pub to_delete: Vec<PlanItem>,
    pub total_bytes: u64,
    pub delete_pct: u32,
    pub aborted_safety: bool,
}

#[derive(Default, Clone, Copy)]
pub struct SyncOutcome {
    pub copied: u64,
    pub updated: u64,
    pub deleted: u64,
    pub errors: u64,
    pub aborted_safety: bool,
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn norm_key(rel: &str) -> String {
    if cfg!(windows) {
        rel.to_lowercase()
    } else {
        rel.to_string()
    }
}

fn norm_path(p: &str) -> String {
    let pb = PathBuf::from(p);
    let c = dunce::canonicalize(&pb).unwrap_or(pb);
    let s = c.to_string_lossy().to_string();
    if cfg!(windows) {
        s.to_lowercase()
    } else {
        s
    }
}

pub fn paths_overlap(a: &str, b: &str) -> bool {
    let (na, nb) = (norm_path(a), norm_path(b));
    if na == nb {
        return true;
    }
    let sep = std::path::MAIN_SEPARATOR.to_string();
    na.starts_with(&format!("{nb}{sep}")) || nb.starts_with(&format!("{na}{sep}"))
}

fn build_globset(patterns: &[String]) -> GlobSet {
    let mut b = GlobSetBuilder::new();
    for p in patterns {
        let p = p.trim();
        if p.is_empty() {
            continue;
        }
        if let Ok(g) = Glob::new(p) {
            b.add(g);
        }
    }
    b.build().unwrap_or_else(|_| GlobSet::empty())
}

fn is_reparse_point(entry: &walkdir::DirEntry) -> bool {
    if entry.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if let Ok(md) = entry.metadata() {
            return md.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0;
        }
    }
    false
}

fn walk_tree(root: &Path, ignore: &GlobSet) -> BTreeMap<String, Entry> {
    let mut map = BTreeMap::new();
    if !root.exists() {
        return map;
    }
    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let p = e.path();
            if p == root {
                return true;
            }
            if is_reparse_point(e) {
                return false;
            }
            match p.strip_prefix(root) {
                Ok(rel) => {
                    let rel_str = rel.to_string_lossy().replace('\\', "/");
                    !ignore.is_match(&rel_str)
                }
                Err(_) => false,
            }
        });

    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path == root {
            continue;
        }
        let ft = entry.file_type();
        if is_reparse_point(&entry) {
            continue;
        }
        let rel = match path.strip_prefix(root) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let is_dir = ft.is_dir();
        map.insert(
            norm_key(&rel),
            Entry {
                rel,
                is_dir,
                size: if is_dir { 0 } else { meta.len() },
                mtime,
                abs: path.to_path_buf(),
            },
        );
    }
    map
}

fn blake3_of(path: &Path) -> Option<[u8; 32]> {
    let bytes = std::fs::read(path).ok()?;
    Some(*blake3::hash(&bytes).as_bytes())
}

fn detect_changed(src: &Entry, dst: &Entry, settings: &Settings) -> (bool, String) {
    if src.size != dst.size {
        return (true, "size".into());
    }
    if (src.mtime - dst.mtime).abs() > settings.mtime_tolerance_sec {
        return (true, "mtime".into());
    }
    if settings.verify_by_content == "blake3" {
        match (blake3_of(&src.abs), blake3_of(&dst.abs)) {
            (Some(a), Some(b)) if a != b => return (true, "content".into()),
            _ => {}
        }
    }
    (false, String::new())
}

fn build_plan(pair: &SyncPair, settings: &Settings) -> Result<ExecPlan, String> {
    let source = PathBuf::from(&pair.source);
    let dest = PathBuf::from(&pair.destination);
    let min_size = pair.min_file_size;
    let max_size = pair.max_file_size;

    if pair.source.trim().is_empty() || pair.destination.trim().is_empty() {
        return Err("Source ou destination vide".into());
    }
    if paths_overlap(&pair.source, &pair.destination) {
        return Err("Source et destination imbriquées (interdit)".into());
    }
    if !source.exists() {
        return Err(format!(
            "Source introuvable : {} (synchro annulée pour éviter d'effacer la destination)",
            pair.source
        ));
    }

    let mut patterns = settings.ignore_patterns.clone();
    patterns.extend(pair.ignore_patterns.clone());
    let ignore = build_globset(&patterns);

    let mut src_map = walk_tree(&source, &ignore);
    let mut dest_map = walk_tree(&dest, &ignore);

    if min_size > 0 || max_size > 0 {
        let filter_size = |e: &Entry| {
            if e.is_dir { return true; }
            if min_size > 0 && e.size < min_size { return false; }
            if max_size > 0 && e.size > max_size { return false; }
            true
        };
        src_map.retain(|_, e| filter_size(e));
        dest_map.retain(|_, e| filter_size(e));
    }

    let dest_total = dest_map.len();

    let mut create_dirs: Vec<(String, PathBuf)> = Vec::new();
    let mut copies: Vec<CopyOp> = Vec::new();
    let mut deletes: Vec<DeleteOp> = Vec::new();
    let mut total_bytes: u64 = 0;

    let dst_abs_of = |rel: &str| -> PathBuf { dest.join(rel) };

    for (k, se) in &src_map {
        match dest_map.get(k) {
            None => {
                if se.is_dir {
                    create_dirs.push((se.rel.clone(), dst_abs_of(&se.rel)));
                } else {
                    total_bytes += se.size;
                    copies.push(CopyOp {
                        rel: se.rel.clone(),
                        src_abs: se.abs.clone(),
                        dst_abs: dst_abs_of(&se.rel),
                        size: se.size,
                        reason: "new".into(),
                        is_new: true,
                    });
                }
            }
            Some(de) => {
                if se.is_dir && de.is_dir {
                } else if !se.is_dir && !de.is_dir {
                    let (changed, reason) = detect_changed(se, de, settings);
                    if changed {
                        total_bytes += se.size;
                        copies.push(CopyOp {
                            rel: se.rel.clone(),
                            src_abs: se.abs.clone(),
                            dst_abs: dst_abs_of(&se.rel),
                            size: se.size,
                            reason,
                            is_new: false,
                        });
                    }
                } else {
                    deletes.push(DeleteOp {
                        rel: de.rel.clone(),
                        abs: de.abs.clone(),
                        is_dir: de.is_dir,
                    });
                    if se.is_dir {
                        create_dirs.push((se.rel.clone(), dst_abs_of(&se.rel)));
                    } else {
                        total_bytes += se.size;
                        copies.push(CopyOp {
                            rel: se.rel.clone(),
                            src_abs: se.abs.clone(),
                            dst_abs: dst_abs_of(&se.rel),
                            size: se.size,
                            reason: "new".into(),
                            is_new: true,
                        });
                    }
                }
            }
        }
    }

    let mut extras: Vec<&Entry> = dest_map
        .iter()
        .filter(|(k, _)| !src_map.contains_key(*k))
        .map(|(_, e)| e)
        .collect();
    let extra_entries = extras.len();

    extras.sort_by(|a, b| a.rel.cmp(&b.rel));
    let mut last_root: Option<String> = None;
    for e in extras {
        let covered = match &last_root {
            Some(r) => e.rel.starts_with(&format!("{r}/")),
            None => false,
        };
        if covered {
            continue;
        }
        deletes.push(DeleteOp {
            rel: e.rel.clone(),
            abs: e.abs.clone(),
            is_dir: e.is_dir,
        });
        last_root = Some(e.rel.clone());
    }

    create_dirs.sort_by_key(|(rel, _)| rel.matches('/').count());

    let delete_pct = if dest_total > 0 {
        ((extra_entries * 100) / dest_total) as u32
    } else {
        0
    };
    let threshold = settings.delete_safety_threshold_pct;
    let aborted_safety = extra_entries > 0 && threshold < 100 && delete_pct > threshold;

    Ok(ExecPlan {
        create_dirs,
        copies,
        deletes,
        total_bytes,
        extra_entries,
        dest_total,
        delete_pct,
        aborted_safety,
    })
}

fn plan_to_dto(plan: &ExecPlan) -> SyncPlan {
    let mut to_copy = Vec::new();
    let mut to_overwrite = Vec::new();
    for c in &plan.copies {
        let item = PlanItem {
            rel: c.rel.clone(),
            is_dir: false,
            size: c.size,
            reason: c.reason.clone(),
        };
        if c.is_new {
            to_copy.push(item);
        } else {
            to_overwrite.push(item);
        }
    }
    let to_delete = plan
        .deletes
        .iter()
        .map(|d| PlanItem {
            rel: d.rel.clone(),
            is_dir: d.is_dir,
            size: 0,
            reason: "extra".into(),
        })
        .collect();
    SyncPlan {
        to_create_dir: plan.create_dirs.iter().map(|(r, _)| r.clone()).collect(),
        to_copy,
        to_overwrite,
        to_delete,
        total_bytes: plan.total_bytes,
        delete_pct: plan.delete_pct,
        aborted_safety: plan.aborted_safety,
    }
}

pub fn dry_run(pair: &SyncPair, settings: &Settings) -> Result<SyncPlan, String> {
    let plan = build_plan(pair, settings)?;
    Ok(plan_to_dto(&plan))
}

fn unique_tmp(dst: &Path) -> PathBuf {
    let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let name = format!(".synctmp-{pid}-{n}");
    match dst.parent() {
        Some(p) => p.join(name),
        None => PathBuf::from(name),
    }
}

#[cfg(windows)]
fn verbatim(p: &Path) -> PathBuf {
    let s = p.to_string_lossy().replace('/', "\\");
    if s.starts_with("\\\\") {
        PathBuf::from(s)
    } else {
        PathBuf::from(format!("\\\\?\\{}", s))
    }
}
#[cfg(not(windows))]
fn verbatim(p: &Path) -> PathBuf {
    p.to_path_buf()
}

fn copy_file_atomic(src: &Path, dst: &Path) -> std::io::Result<()> {
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

fn safe_delete(abs: &Path, behavior: &str) -> Result<(), String> {
    if behavior == "trash" {
        trash::delete(abs).map_err(|e| e.to_string())
    } else if abs.is_dir() {
        std::fs::remove_dir_all(abs).map_err(|e| e.to_string())
    } else {
        std::fs::remove_file(abs).map_err(|e| e.to_string())
    }
}

fn log(app: &AppHandle, pair_id: &str, level: &str, action: &str, path: Option<String>, msg: String) {
    let p = path.clone().unwrap_or_default();
    let state = app.state::<AppState>();
    state.push_log(LogEntry {
        at: now_iso(),
        level: level.into(),
        pair_id: Some(pair_id.into()),
        action: action.into(),
        path,
        message: msg.clone(),
    });
    match level {
        "error" => tracing::error!(pair = pair_id, action, path = %p, "{msg}"),
        "warn" => tracing::warn!(pair = pair_id, action, path = %p, "{msg}"),
        _ => tracing::info!(pair = pair_id, action, path = %p, "{msg}"),
    }
}

fn execute_plan(app: &AppHandle, pair: &SyncPair, settings: &Settings, plan: &ExecPlan) -> SyncOutcome {
    let mut out = SyncOutcome::default();
    out.aborted_safety = plan.aborted_safety;

    let total = plan.copies.len() + plan.create_dirs.len() + plan.deletes.len();
    let _ = app.emit(
        "sync:started",
        json!({
            "pairId": pair.id,
            "name": pair.name,
            "totalFiles": plan.copies.len(),
            "totalBytes": plan.total_bytes,
        }),
    );

    let mut done = 0usize;
    let step = (total / 50).max(1);
    let emit_progress = |app: &AppHandle, done: usize| {
        let _ = app.emit(
            "sync:progress",
            json!({ "pairId": pair.id, "done": done, "total": total }),
        );
    };

    for (rel, abs) in &plan.create_dirs {
        if let Err(e) = std::fs::create_dir_all(abs) {
            out.errors += 1;
            log(app, &pair.id, "error", "error", Some(rel.clone()), format!("mkdir échec : {e}"));
        }
        done += 1;
        if done % step == 0 {
            emit_progress(app, done);
        }
    }

    for c in &plan.copies {
        let mut res = copy_file_atomic(&c.src_abs, &c.dst_abs);
        if res.is_err() {
            std::thread::sleep(std::time::Duration::from_millis(60));
            res = copy_file_atomic(&c.src_abs, &c.dst_abs);
        }
        match res {
            Ok(()) => {
                if c.is_new {
                    out.copied += 1;
                    log(app, &pair.id, "info", "copy", Some(c.rel.clone()), "copié".into());
                } else {
                    out.updated += 1;
                    log(app, &pair.id, "info", "update", Some(c.rel.clone()), format!("mis à jour ({})", c.reason));
                }
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound || !c.src_abs.exists() {
                    log(app, &pair.id, "warn", "skip", Some(c.rel.clone()), "ignoré (source introuvable)".into());
                } else {
                    out.errors += 1;
                    log(app, &pair.id, "error", "error", Some(c.rel.clone()), format!("copie échec : {e}"));
                }
            }
        }
        done += 1;
        if done % step == 0 {
            emit_progress(app, done);
        }
    }

    if plan.aborted_safety {
        log(
            app,
            &pair.id,
            "warn",
            "skip",
            None,
            format!(
                "Suppressions RETENUES : {}% de la destination ({}/{} entrées) serait supprimé, seuil = {}%. Vérifie la source puis relance.",
                plan.delete_pct, plan.extra_entries, plan.dest_total, settings.delete_safety_threshold_pct
            ),
        );
        let _ = app.emit(
            "app:error",
            json!({
                "pairId": pair.id,
                "message": format!("Suppressions retenues (seuil sécurité {}% dépassé : {}%)", settings.delete_safety_threshold_pct, plan.delete_pct)
            }),
        );
    } else {
        for d in &plan.deletes {
            match safe_delete(&d.abs, &settings.delete_behavior) {
                Ok(()) => {
                    out.deleted += 1;
                    log(app, &pair.id, "info", "delete", Some(d.rel.clone()), "supprimé".into());
                }
                Err(e) => {
                    out.errors += 1;
                    log(app, &pair.id, "error", "error", Some(d.rel.clone()), format!("suppression échec : {e}"));
                }
            }
            done += 1;
            if done % step == 0 {
                emit_progress(app, done);
            }
        }
    }

    emit_progress(app, done);
    out
}

pub fn run_sync(app: &AppHandle, pair: SyncPair, settings: Settings) {
    let state = app.state::<AppState>();
    state.set_status(&pair.id, "syncing");
    let _ = app.emit("pair:status", json!({ "pairId": pair.id, "status": "syncing" }));

    let plan = match build_plan(&pair, &settings) {
        Ok(p) => p,
        Err(e) => {
            log(app, &pair.id, "error", "error", None, e.clone());
            state.set_status(&pair.id, "error");
            let _ = app.emit("pair:status", json!({ "pairId": pair.id, "status": "error" }));
            let _ = app.emit("app:error", json!({ "pairId": pair.id, "message": e.clone() }));
            update_last_run(app, &pair.id, "error", &SyncOutcome::default());
            if settings.notify_pc && pair.notify_pc {
                notify(app, "Reflet — erreur", &format!("{} : {}", pair.name, e));
            }
            return;
        }
    };

    let outcome = execute_plan(app, &pair, &settings, &plan);

    let ok = outcome.errors == 0 && !outcome.aborted_safety;
    let run_status = if ok { "ok" } else { "error" };
    update_last_run(app, &pair.id, run_status, &outcome);

    let rt_status = if ok { "idle" } else { "error" };
    state.set_status(&pair.id, rt_status);

    let _ = app.emit(
        "sync:finished",
        json!({
            "pairId": pair.id,
            "name": pair.name,
            "copied": outcome.copied,
            "updated": outcome.updated,
            "deleted": outcome.deleted,
            "errors": outcome.errors,
            "abortedSafety": outcome.aborted_safety,
            "status": run_status,
        }),
    );
    let _ = app.emit("pair:status", json!({ "pairId": pair.id, "status": rt_status }));
    let _ = app.emit("state:changed", json!({}));

    let body = format!(
        "{} : {} copiés, {} màj, {} supprimés{}",
        pair.name,
        outcome.copied,
        outcome.updated,
        outcome.deleted,
        if outcome.errors > 0 { format!(", {} erreurs", outcome.errors) } else { String::new() }
    );
    if settings.notify_pc && pair.notify_pc {
        notify(app, if ok { "Reflet — synchro terminée" } else { "Reflet — synchro avec erreurs" }, &body);
    }
}

fn update_last_run(app: &AppHandle, pair_id: &str, status: &str, out: &SyncOutcome) {
    let state = app.state::<AppState>();
    let mut cfg = state.config.lock().unwrap();
    if let Some(p) = cfg.pairs.iter_mut().find(|p| p.id == pair_id) {
        p.last_run = Some(crate::config::LastRun {
            at: now_iso(),
            status: status.into(),
            copied: out.copied,
            updated: out.updated,
            deleted: out.deleted,
            errors: out.errors,
        });
    }
    let snapshot = cfg.clone();
    drop(cfg);
    let _ = crate::config::save(app, &snapshot);
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    use tauri_plugin_notification::NotificationExt;
    let _ = app.notification().builder().title(title).body(body).show();
}
