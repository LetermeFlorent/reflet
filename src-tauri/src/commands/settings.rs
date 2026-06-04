use super::{apply_autostart, persist_and_notify};
use crate::config::Settings;
use crate::state::AppState;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, State};

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
    persist_and_notify(&app, &state)
}

#[tauri::command]
pub fn set_scheduler_running(app: AppHandle, state: State<AppState>, running: bool) -> Result<(), String> {
    state.scheduler_running.store(running, Ordering::SeqCst);
    {
        let mut cfg = state.config.lock().unwrap();
        cfg.settings.scheduler_running = running;
    }
    persist_and_notify(&app, &state)
}
