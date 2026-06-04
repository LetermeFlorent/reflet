use crate::state::{AppState, LogEntry};
use tauri::State;

#[tauri::command]
pub fn get_logs(state: State<AppState>) -> Vec<LogEntry> {
    state.logs.lock().unwrap().iter().cloned().collect()
}

#[tauri::command]
pub fn clear_logs(state: State<AppState>) {
    state.logs.lock().unwrap().clear();
}
