use super::send_request;
use crate::state::{AppState, SyncRequest};
use crate::sync::{self, SyncPlan};
use tauri::State;

#[tauri::command]
pub fn sync_now(state: State<AppState>, id: String) -> Result<(), String> {
    send_request(&state, SyncRequest::Pair(id))
}

#[tauri::command]
pub fn sync_all(state: State<AppState>) -> Result<(), String> {
    send_request(&state, SyncRequest::All)
}

#[tauri::command]
pub async fn dry_run(state: State<'_, AppState>, id: String) -> Result<SyncPlan, String> {
    let (pair, settings) = {
        let cfg = state.config.lock().unwrap();
        let pair = cfg
            .pairs
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .ok_or_else(|| "Paire introuvable".to_string())?;
        (pair, cfg.settings.clone())
    };
    let archive_mode = pair.compression.method != "off" && !pair.backup_mode;
    tauri::async_runtime::spawn_blocking(move || {
        if archive_mode {
            Ok(sync::plan_archive(&pair, &settings))
        } else {
            sync::dry_run(&pair, &settings)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
