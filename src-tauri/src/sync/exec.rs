use super::io::{copy_file_atomic, safe_delete};
use super::types::{ExecPlan, SyncOutcome};
use super::util::log;
use crate::config::{Settings, SyncPair};
use serde_json::json;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use tauri::{AppHandle, Emitter};

/// Nombre de copies de fichiers menées en parallèle au sein d'une même paire
/// (les fichiers sont indépendants ; gros gain sur cibles réseau / nombreux fichiers).
const COPY_CONCURRENCY: usize = 4;

pub(super) fn execute_plan(
    app: &AppHandle,
    pair: &SyncPair,
    settings: &Settings,
    plan: &ExecPlan,
) -> SyncOutcome {
    let mut out = SyncOutcome {
        aborted_safety: plan.aborted_safety,
        ..Default::default()
    };

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

    // Copies en parallèle (bornées). Les dossiers sont déjà créés ci-dessus ; chaque
    // copie écrit un fichier distinct (tmp à compteur atomique unique) → indépendant.
    {
        let copied = AtomicU64::new(0);
        let updated = AtomicU64::new(0);
        let errors = AtomicU64::new(0);
        let done_a = AtomicUsize::new(done);
        let next = AtomicUsize::new(0);
        let n = plan.copies.len();
        let workers = COPY_CONCURRENCY.min(n).max(1);
        std::thread::scope(|s| {
            for _ in 0..workers {
                s.spawn(|| loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= n {
                        break;
                    }
                    let c = &plan.copies[i];
                    let mut res = copy_file_atomic(&c.src_abs, &c.dst_abs);
                    if res.is_err() {
                        std::thread::sleep(std::time::Duration::from_millis(60));
                        res = copy_file_atomic(&c.src_abs, &c.dst_abs);
                    }
                    match res {
                        Ok(()) => {
                            if c.is_new {
                                copied.fetch_add(1, Ordering::Relaxed);
                                log(app, &pair.id, "info", "copy", Some(c.rel.clone()), "copié".into());
                            } else {
                                updated.fetch_add(1, Ordering::Relaxed);
                                log(app, &pair.id, "info", "update", Some(c.rel.clone()), format!("mis à jour ({})", c.reason));
                            }
                        }
                        Err(e) => {
                            if e.kind() == std::io::ErrorKind::NotFound || !c.src_abs.exists() {
                                log(app, &pair.id, "warn", "skip", Some(c.rel.clone()), "ignoré (source introuvable)".into());
                            } else {
                                errors.fetch_add(1, Ordering::Relaxed);
                                log(app, &pair.id, "error", "error", Some(c.rel.clone()), format!("copie échec : {e}"));
                            }
                        }
                    }
                    let d = done_a.fetch_add(1, Ordering::Relaxed) + 1;
                    if d % step == 0 {
                        let _ = app.emit("sync:progress", json!({ "pairId": pair.id, "done": d, "total": total }));
                    }
                });
            }
        });
        out.copied += copied.load(Ordering::Relaxed);
        out.updated += updated.load(Ordering::Relaxed);
        out.errors += errors.load(Ordering::Relaxed);
        done = done_a.load(Ordering::Relaxed);
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
