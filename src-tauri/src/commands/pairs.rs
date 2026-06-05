use super::{notify_state_changed, persist_and_notify, reconcile_watcher};
use crate::config::{self, SyncPair};
use crate::state::AppState;
use crate::sync;
use serde::Deserialize;
use tauri::{AppHandle, State};

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
    #[serde(default)]
    pub backup_mode: bool,
}

fn default_true() -> bool {
    true
}

/// Filtre de taille incohérent (min > max non nul) = aucun fichier ne passe jamais,
/// silencieusement. On le refuse à l'enregistrement plutôt que de laisser une paire muette.
fn validate_size_filter(min: u64, max: u64) -> Result<(), String> {
    if max > 0 && min > max {
        return Err("Taille minimale supérieure à la taille maximale.".into());
    }
    Ok(())
}

/// Empêche deux paires en mode archive de viser le même fichier .zip (elles
/// s'effaceraient mutuellement au nettoyage). Retourne le nom de la paire en conflit.
fn archive_conflict(state: &AppState, self_id: &str, pair: &SyncPair) -> Option<String> {
    let target = sync::archive_target(pair)?;
    let cfg = state.config.lock().unwrap();
    cfg.pairs
        .iter()
        .find(|p| p.id != self_id && sync::archive_target(p).as_deref() == Some(target.as_path()))
        .map(|p| p.name.clone())
}

#[tauri::command]
pub fn add_pair(app: AppHandle, state: State<AppState>, new: NewPair) -> Result<String, String> {
    if new.source.trim().is_empty() || new.destination.trim().is_empty() {
        return Err("Source et destination requises".into());
    }
    if sync::paths_overlap(&new.source, &new.destination) {
        return Err("Source et destination imbriquées (interdit)".into());
    }
    validate_size_filter(new.min_file_size, new.max_file_size)?;
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
        backup_mode: new.backup_mode,
        last_run: None,
    };
    if let Some(other) = archive_conflict(&state, &id, &pair) {
        return Err(format!(
            "Conflit d'archive avec « {other} » (même fichier .zip de destination). Change le nom de l'archive ou la destination."
        ));
    }
    let source = pair.source.clone();
    state.config.lock().unwrap().pairs.push(pair);
    reconcile_watcher(&state, &id, enabled, watch_realtime, Some(&source));
    persist_and_notify(&app, &state)?;
    Ok(id)
}

#[tauri::command]
pub fn update_pair(app: AppHandle, state: State<AppState>, pair: SyncPair) -> Result<(), String> {
    if sync::paths_overlap(&pair.source, &pair.destination) {
        return Err("Source et destination imbriquées (interdit)".into());
    }
    validate_size_filter(pair.min_file_size, pair.max_file_size)?;
    if let Some(other) = archive_conflict(&state, &pair.id, &pair) {
        return Err(format!(
            "Conflit d'archive avec « {other} » (même fichier .zip de destination). Change le nom de l'archive ou la destination."
        ));
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
                existing.backup_mode = pair.backup_mode;
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
    persist_and_notify(&app, &state)
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
    persist_and_notify(&app, &state)
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
    reconcile_watcher(&state, &id, enabled, watch, source.as_deref());
    persist_and_notify(&app, &state)
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
    reconcile_watcher(&state, &id, enabled, watch, source.as_deref());
    persist_and_notify(&app, &state)
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
