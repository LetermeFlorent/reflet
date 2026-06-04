use super::io::copy_file_atomic;
use super::scan::{build_globset, walk_tree};
use super::types::SyncOutcome;
use super::util::log;
use crate::config::{Settings, SyncPair};
use chrono::Local;
use serde_json::json;
use std::path::Path;
use tauri::{AppHandle, Emitter};

/// Mode sauvegarde : copie l'arbre source complet dans un sous-dossier horodaté
/// de la destination (aucune détection de différences, aucune suppression).
pub(super) fn run_backup(app: &AppHandle, pair: &SyncPair, settings: &Settings) -> SyncOutcome {
    let mut out = SyncOutcome::default();
    let stamp = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let root = Path::new(&pair.destination).join(format!("backup-{stamp}"));

    let mut patterns = settings.ignore_patterns.clone();
    patterns.extend(pair.ignore_patterns.clone());
    let ignore = build_globset(&patterns);
    let entries = walk_tree(Path::new(&pair.source), &ignore);

    let total = entries.len();
    let _ = app.emit(
        "sync:started",
        json!({ "pairId": pair.id, "name": pair.name, "totalFiles": total, "totalBytes": 0 }),
    );

    let mut done = 0usize;
    let step = (total / 50).max(1);
    for e in entries.values() {
        let dst = root.join(&e.rel);
        if e.is_dir {
            if let Err(err) = std::fs::create_dir_all(&dst) {
                out.errors += 1;
                log(app, &pair.id, "error", "error", Some(e.rel.clone()), format!("mkdir échec : {err}"));
            }
        } else {
            match copy_file_atomic(&e.abs, &dst) {
                Ok(()) => {
                    out.copied += 1;
                    log(app, &pair.id, "info", "copy", Some(e.rel.clone()), "sauvegardé".into());
                }
                Err(err) => {
                    out.errors += 1;
                    log(app, &pair.id, "error", "error", Some(e.rel.clone()), format!("copie échec : {err}"));
                }
            }
        }
        done += 1;
        if done % step == 0 {
            let _ = app.emit("sync:progress", json!({ "pairId": pair.id, "done": done, "total": total }));
        }
    }
    let _ = app.emit("sync:progress", json!({ "pairId": pair.id, "done": done, "total": total }));
    log(
        app,
        &pair.id,
        "info",
        "info",
        None,
        format!("Sauvegarde créée : backup-{stamp} ({} fichiers)", out.copied),
    );
    out
}
