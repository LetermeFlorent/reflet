use crate::state::{AppState, LogEntry};
use chrono::Utc;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// Chemin « verbatim » Windows (`\\?\`) : gère chemins longs et noms finissant par un
/// point/espace. No-op hors Windows. Centralisé ici (utilisé par io.rs et archive_io.rs).
#[cfg(windows)]
pub(super) fn verbatim(p: &Path) -> PathBuf {
    let s = p.to_string_lossy().replace('/', "\\");
    if s.starts_with("\\\\") {
        PathBuf::from(s)
    } else {
        PathBuf::from(format!("\\\\?\\{s}"))
    }
}
#[cfg(not(windows))]
pub(super) fn verbatim(p: &Path) -> PathBuf {
    p.to_path_buf()
}

pub(super) fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

pub(super) fn log(
    app: &AppHandle,
    pair_id: &str,
    level: &str,
    action: &str,
    path: Option<String>,
    msg: String,
) {
    let p = path.clone().unwrap_or_default();
    let state = app.state::<AppState>();
    state.push_log(LogEntry {
        at: now_iso(),
        level: level.into(),
        pair_id: Some(pair_id.into()),
        action: action.into(),
        path,
        message: msg.clone(),
    });
    match level {
        "error" => tracing::error!(pair = pair_id, action, path = %p, "{msg}"),
        "warn" => tracing::warn!(pair = pair_id, action, path = %p, "{msg}"),
        _ => tracing::info!(pair = pair_id, action, path = %p, "{msg}"),
    }
}
