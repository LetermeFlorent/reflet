mod app;
mod logs;
mod pairs;
mod settings;
mod sync;

pub use app::*;
pub use logs::*;
pub use pairs::*;
pub use settings::*;
pub use sync::*;

use crate::config;
use crate::state::{AppState, SyncRequest};
use tauri::{AppHandle, Emitter, Manager};

pub(super) fn persist(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let snapshot = state.config.lock().unwrap().clone();
    config::save(app, &snapshot)
}

pub(super) fn notify_state_changed(app: &AppHandle) {
    let _ = app.emit("state:changed", serde_json::json!({}));
}

pub(super) fn persist_and_notify(app: &AppHandle, state: &AppState) -> Result<(), String> {
    persist(app, state)?;
    notify_state_changed(app);
    Ok(())
}

/// Démarre ou arrête le watcher temps-réel d'une paire selon (enabled && watch).
pub(super) fn reconcile_watcher(state: &AppState, id: &str, enabled: bool, watch: bool, source: Option<&str>) {
    if let Some(wm) = state.watcher_manager.lock().unwrap().as_mut() {
        if enabled && watch {
            if let Some(src) = source {
                wm.start(id, src);
            }
        } else {
            wm.stop(id);
        }
    }
}

pub fn apply_autostart(app: &AppHandle, enable: bool) {
    #[cfg(desktop)]
    {
        use tauri_plugin_autostart::ManagerExt;
        let mgr = app.autolaunch();
        let _ = if enable { mgr.enable() } else { mgr.disable() };
    }
    #[cfg(not(desktop))]
    let _ = (app, enable);
}

pub(super) fn send_request(state: &AppState, req: SyncRequest) -> Result<(), String> {
    let guard = state.sync_tx.lock().unwrap();
    match guard.as_ref() {
        Some(tx) => tx.try_send(req).map_err(|e| e.to_string()),
        None => Err("worker de synchronisation non démarré".into()),
    }
}

pub fn trigger_all(app: &AppHandle) {
    let state = app.state::<AppState>();
    let _ = send_request(&state, SyncRequest::All);
}
