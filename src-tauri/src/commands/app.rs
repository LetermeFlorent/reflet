use crate::compression::{self, CompressionMethod};
use crate::state::AppState;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager, State};

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
                let iv = crate::scheduler::pair_interval(p, &cfg.settings);
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
pub async fn detect_compression_methods() -> Vec<CompressionMethod> {
    // detect_methods() lance des sous-process (has_binary --help) → bloquant. On l'exécute
    // hors du thread principal pour ne pas figer l'UI au démarrage (loader animé, fenêtre
    // déplaçable) pendant la détection des outils externes.
    tauri::async_runtime::spawn_blocking(compression::detect_methods)
        .await
        .unwrap_or_default()
}

/// Ouvre une URL dans le navigateur par défaut du système.
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("URL invalide".into());
    }
    #[cfg(target_os = "windows")]
    let res = std::process::Command::new("cmd").args(["/C", "start", "", &url]).spawn();
    #[cfg(target_os = "macos")]
    let res = std::process::Command::new("open").arg(&url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let res = std::process::Command::new("xdg-open").arg(&url).spawn();
    res.map(|_| ()).map_err(|e| format!("Ouverture du lien échouée : {e}"))
}
