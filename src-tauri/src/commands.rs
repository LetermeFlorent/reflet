
use crate::compression::{self, CompressionMethod};
use crate::config::{self, Settings, SyncPair};
use crate::state::{AppState, LogEntry, SyncRequest};
use crate::sync::{self, SyncPlan};
use serde::Deserialize;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, State};


fn persist(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let snapshot = state.config.lock().unwrap().clone();
    config::save(app, &snapshot)
}

fn notify_state_changed(app: &AppHandle) {
    let _ = app.emit("state:changed", serde_json::json!({}));
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

fn send_request(state: &AppState, req: SyncRequest) -> Result<(), String> {
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


const MIN_INTERVAL: u64 = 5;

#[tauri::command]
pub fn get_app_state(state: State<AppState>) -> serde_json::Value {
    let cfg = state.config.lock().unwrap();
    let statuses = state.statuses.lock().unwrap();
    let last_started = state.last_started.lock().unwrap();
    let scheduler_running = state.scheduler_running.load(Ordering::SeqCst);
    let now = std::time::Instant::now();
    let pairs: Vec<serde_json::Value> = cfg
        .pairs
        .iter()
        .map(|p| {
            let st = if !p.enabled {
                "disabled".to_string()
            } else {
                statuses.get(&p.id).cloned().unwrap_or_else(|| "idle".into())
            };
            let next_run_sec: Option<u64> = if p.enabled && scheduler_running {
                let iv = match p.interval_sec_override {
                    Some(v) if v >= MIN_INTERVAL => v,
                    _ => cfg.settings.interval_sec.max(MIN_INTERVAL),
                };
                let elapsed = last_started
                    .get(&p.id)
                    .map(|t| now.duration_since(*t).as_secs())
                    .unwrap_or(0);
                Some(iv.saturating_sub(elapsed))
            } else {
                None
            };
            let mut v = serde_json::to_value(p).unwrap_or(serde_json::Value::Null);
            if let Some(obj) = v.as_object_mut() {
                obj.insert("status".into(), serde_json::Value::String(st));
                obj.insert(
                    "nextRunSec".into(),
                    match next_run_sec {
                        Some(s) => serde_json::Value::from(s),
                        None => serde_json::Value::Null,
                    },
                );
            }
            v
        })
        .collect();
    serde_json::json!({
        "settings": cfg.settings,
        "pairs": pairs,
        "schedulerRunning": scheduler_running,
        "syncBusy": state.sync_busy.load(Ordering::SeqCst),
    })
}


#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.config.lock().unwrap().settings.clone()
}

#[tauri::command]
pub fn update_settings(app: AppHandle, state: State<AppState>, settings: Settings) -> Result<(), String> {
    {
        let mut cfg = state.config.lock().unwrap();
        cfg.settings = settings.clone();
    }
    state
        .scheduler_running
        .store(settings.scheduler_running, Ordering::SeqCst);
    apply_autostart(&app, settings.autostart);
    persist(&app, &state)?;
    notify_state_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn set_scheduler_running(app: AppHandle, state: State<AppState>, running: bool) -> Result<(), String> {
    state.scheduler_running.store(running, Ordering::SeqCst);
    {
        let mut cfg = state.config.lock().unwrap();
        cfg.settings.scheduler_running = running;
    }
    persist(&app, &state)?;
    notify_state_changed(&app);
    Ok(())
}


#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewPair {
    pub name: String,
    pub source: String,
    pub destination: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub interval_sec_override: Option<u64>,
    #[serde(default = "default_true")]
    pub notify_pc: bool,
    #[serde(default = "default_true")]
    pub notify_app: bool,
    #[serde(default)]
    pub ignore_patterns: Vec<String>,
    #[serde(default)]
    pub watch_realtime: bool,
    #[serde(default)]
    pub schedule_times: Vec<String>,
    #[serde(default)]
    pub min_file_size: u64,
    #[serde(default)]
    pub max_file_size: u64,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub compression: crate::compression::CompressionConfig,
}

fn default_true() -> bool {
    true
}

#[tauri::command]
pub fn add_pair(app: AppHandle, state: State<AppState>, new: NewPair) -> Result<String, String> {
    if new.source.trim().is_empty() || new.destination.trim().is_empty() {
        return Err("Source et destination requises".into());
    }
    if sync::paths_overlap(&new.source, &new.destination) {
        return Err("Source et destination imbriquées (interdit)".into());
    }
    let id = uuid::Uuid::new_v4().to_string();
    let source = new.source.clone();
    let watch_realtime = new.watch_realtime;
    let enabled = new.enabled;
    let pair = SyncPair {
        id: id.clone(),
        name: if new.name.trim().is_empty() {
            source.clone()
        } else {
            new.name
        },
        source,
        destination: new.destination,
        enabled,
        interval_sec_override: new.interval_sec_override,
        notify_pc: new.notify_pc,
        notify_app: new.notify_app,
        ignore_patterns: new.ignore_patterns,
        watch_realtime,
        schedule_times: new.schedule_times.clone(),
        min_file_size: new.min_file_size,
        max_file_size: new.max_file_size,
        color: new.color.clone(),
        compression: new.compression,
        last_run: None,
    };
    let source = pair.source.clone();
    state.config.lock().unwrap().pairs.push(pair);
    if enabled && watch_realtime {
        if let Some(wm) = state.watcher_manager.lock().unwrap().as_mut() {
            wm.start(&id, &source);
        }
    }
    persist(&app, &state)?;
    notify_state_changed(&app);
    Ok(id)
}

#[tauri::command]
pub fn update_pair(app: AppHandle, state: State<AppState>, pair: SyncPair) -> Result<(), String> {
    if sync::paths_overlap(&pair.source, &pair.destination) {
        return Err("Source et destination imbriquées (interdit)".into());
    }
    let old_source: Option<String>;
    let old_enabled: bool;
    let old_watch: bool;
    let id = pair.id.clone();
    let new_source = pair.source.clone();
    let new_enabled = pair.enabled;
    let new_watch = pair.watch_realtime;
    {
        let mut cfg = state.config.lock().unwrap();
        match cfg.pairs.iter_mut().find(|p| p.id == pair.id) {
            Some(existing) => {
                old_source = Some(existing.source.clone());
                old_enabled = existing.enabled;
                old_watch = existing.watch_realtime;
                existing.name = pair.name;
                existing.source = pair.source;
                existing.destination = pair.destination;
                existing.enabled = pair.enabled;
                existing.interval_sec_override = pair.interval_sec_override;
                existing.notify_pc = pair.notify_pc;
                existing.notify_app = pair.notify_app;
                existing.ignore_patterns = pair.ignore_patterns;
                existing.watch_realtime = pair.watch_realtime;
                existing.schedule_times = pair.schedule_times;
                existing.min_file_size = pair.min_file_size;
                existing.max_file_size = pair.max_file_size;
                existing.color = pair.color;
                existing.compression = pair.compression;
            }
            None => return Err("Paire introuvable".into()),
        }
    }
    if let Some(wm) = state.watcher_manager.lock().unwrap().as_mut() {
        let was_watching = old_enabled && old_watch;
        let will_watch = new_enabled && new_watch;
        if was_watching && !will_watch {
            wm.stop(&id);
        } else if !was_watching && will_watch {
            wm.start(&id, &new_source);
        } else if was_watching && will_watch && old_source != Some(new_source.clone()) {
            wm.stop(&id);
            wm.start(&id, &new_source);
        }
    }
    persist(&app, &state)?;
    notify_state_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_pair(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    {
        let mut cfg = state.config.lock().unwrap();
        cfg.pairs.retain(|p| p.id != id);
    }
    state.statuses.lock().unwrap().remove(&id);
    if let Some(wm) = state.watcher_manager.lock().unwrap().as_mut() {
        wm.stop(&id);
    }
    persist(&app, &state)?;
    notify_state_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn set_pair_enabled(app: AppHandle, state: State<AppState>, id: String, enabled: bool) -> Result<(), String> {
    let source: Option<String>;
    let watch: bool;
    {
        let mut cfg = state.config.lock().unwrap();
        let pair = cfg.pairs.iter_mut().find(|p| p.id == id);
        match pair {
            Some(p) => {
                p.enabled = enabled;
                source = Some(p.source.clone());
                watch = p.watch_realtime;
            }
            None => return Err("Paire introuvable".into()),
        }
    }
    if let Some(wm) = state.watcher_manager.lock().unwrap().as_mut() {
        if enabled && watch {
            if let Some(src) = &source {
                wm.start(&id, src);
            }
        } else {
            wm.stop(&id);
        }
    }
    persist(&app, &state)?;
    notify_state_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn set_pair_watch_realtime(app: AppHandle, state: State<AppState>, id: String, watch: bool) -> Result<(), String> {
    let enabled: bool;
    let source: Option<String>;
    {
        let mut cfg = state.config.lock().unwrap();
        let pair = cfg.pairs.iter_mut().find(|p| p.id == id);
        match pair {
            Some(p) => {
                p.watch_realtime = watch;
                enabled = p.enabled;
                source = Some(p.source.clone());
            }
            None => return Err("Paire introuvable".into()),
        }
    }
    if let Some(wm) = state.watcher_manager.lock().unwrap().as_mut() {
        if enabled && watch {
            if let Some(src) = &source {
                wm.start(&id, src);
            }
        } else {
            wm.stop(&id);
        }
    }
    persist(&app, &state)?;
    notify_state_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn reorder_pairs(app: AppHandle, state: State<AppState>, ordered_ids: Vec<String>) -> Result<(), String> {
    let mut cfg = state.config.lock().unwrap();
    let pairs = std::mem::take(&mut cfg.pairs);
    let pair_map: std::collections::HashMap<&str, SyncPair> = pairs.iter().map(|p| (p.id.as_str(), p.clone())).collect();
    let mut reordered: Vec<SyncPair> = Vec::with_capacity(ordered_ids.len());
    for id in &ordered_ids {
        if let Some(p) = pair_map.get(id.as_str()) {
            reordered.push(p.clone());
        }
    }
    for p in pairs {
        if !ordered_ids.contains(&p.id) {
            reordered.push(p);
        }
    }
    cfg.pairs = reordered;
    let snapshot = cfg.clone();
    drop(cfg);
    config::save(&app, &snapshot)?;
    notify_state_changed(&app);
    Ok(())
}


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
    tauri::async_runtime::spawn_blocking(move || sync::dry_run(&pair, &settings))
        .await
        .map_err(|e| e.to_string())?
}


#[tauri::command]
pub fn get_logs(state: State<AppState>) -> Vec<LogEntry> {
    state.logs.lock().unwrap().iter().cloned().collect()
}

#[tauri::command]
pub fn clear_logs(state: State<AppState>) {
    state.logs.lock().unwrap().clear();
}


#[tauri::command]
pub fn show_window(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

#[tauri::command]
pub fn hide_window(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    let state = app.state::<AppState>();
    state.really_quit.store(true, Ordering::SeqCst);
    app.exit(0);
}

#[tauri::command]
pub fn detect_compression_methods() -> Vec<CompressionMethod> {
    compression::detect_methods()
}
