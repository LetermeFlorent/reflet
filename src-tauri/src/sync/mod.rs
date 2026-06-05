mod archive;
mod archive_io;
mod backup;
mod diff;
mod exec;
mod io;
mod plan;
mod scan;
mod types;
mod util;

pub use archive::plan_archive;
pub use plan::dry_run;
pub use scan::paths_overlap;
pub use types::SyncPlan;

use crate::config::{Settings, SyncPair};
use crate::state::AppState;
use exec::execute_plan;
use plan::build_plan;
use serde_json::json;
use std::path::Path;
use tauri::{AppHandle, Emitter, Manager};
use types::SyncOutcome;
use util::{log, now_iso};

/// Chemin de l'archive unique d'une paire (mode compression), pour détecter les
/// collisions entre paires. None si la paire n'est pas en mode archive.
pub fn archive_target(pair: &SyncPair) -> Option<std::path::PathBuf> {
    if pair.compression.method != "off" && !pair.backup_mode {
        Some(archive_io::archive_path(pair))
    } else {
        None
    }
}

/// Retourne la raison si la source ou la destination d'une paire est introuvable.
fn missing_folder(pair: &SyncPair) -> Option<String> {
    if pair.source.trim().is_empty() || !Path::new(&pair.source).exists() {
        return Some(format!("dossier source introuvable ({})", pair.source));
    }
    if pair.destination.trim().is_empty() {
        return Some("destination non definie".into());
    }
    let dst = Path::new(&pair.destination);
    // Injoignable = ni la destination ni son dossier parent n'existent (lecteur debranche...).
    if !dst.exists() && !dst.parent().map(|p| p.exists()).unwrap_or(false) {
        return Some(format!("dossier destination injoignable ({})", pair.destination));
    }
    None
}

/// Desactive une paire (dossier manquant) pour eviter une boucle d'erreurs.
fn disable_pair(app: &AppHandle, pair: &SyncPair, reason: &str) {
    let state = app.state::<AppState>();
    {
        let mut cfg = state.config.lock().unwrap();
        if let Some(p) = cfg.pairs.iter_mut().find(|p| p.id == pair.id) {
            p.enabled = false;
        }
        let snapshot = cfg.clone();
        drop(cfg);
        let _ = crate::config::save(app, &snapshot);
    }
    if let Some(wm) = state.watcher_manager.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        wm.stop(&pair.id);
    }
    state.set_status(&pair.id, "disabled");
    let msg = format!("Paire « {} » desactivee : {reason}", pair.name);
    log(app, &pair.id, "warn", "skip", None, msg.clone());
    let _ = app.emit("pair:status", json!({ "pairId": pair.id, "status": "disabled" }));
    let _ = app.emit("app:error", json!({ "pairId": pair.id, "message": msg }));
    let _ = app.emit("state:changed", json!({}));
}

pub fn run_sync(app: &AppHandle, pair: SyncPair, settings: Settings) {
    let state = app.state::<AppState>();

    if let Some(reason) = missing_folder(&pair) {
        disable_pair(app, &pair, &reason);
        return;
    }

    state.set_status(&pair.id, "syncing");
    let _ = app.emit("pair:status", json!({ "pairId": pair.id, "status": "syncing" }));

    let outcome = if pair.backup_mode {
        backup::run_backup(app, &pair, &settings)
    } else if pair.compression.method != "off" {
        archive::run_archive_sync(app, &pair, &settings)
    } else {
        match build_plan(&pair, &settings) {
            Ok(plan) => execute_plan(app, &pair, &settings, &plan),
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
        }
    };

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
