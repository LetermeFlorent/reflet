use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LastRun {
    pub at: String,
    pub status: String,
    pub copied: u64,
    pub updated: u64,
    pub deleted: u64,
    pub errors: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPair {
    pub id: String,
    pub name: String,
    pub source: String,
    pub destination: String,
    #[serde(default = "default_true")]
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
    pub last_run: Option<LastRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub interval_sec: u64,
    pub delete_behavior: String,
    pub autostart: bool,
    pub start_minimized: bool,
    pub confirm_deletes_with_dry_run: bool,
    pub ignore_patterns: Vec<String>,
    pub verify_by_content: String,
    pub mtime_tolerance_sec: i64,
    pub delete_safety_threshold_pct: u32,
    pub scheduler_running: bool,
    #[serde(default)]
    pub notify_pc: bool,
    #[serde(default)]
    pub notify_app: bool,
    #[serde(default = "default_true")]
    pub compact_cards: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            interval_sec: 900,
            delete_behavior: "trash".into(),
            autostart: false,
            start_minimized: false,
            confirm_deletes_with_dry_run: true,
            ignore_patterns: vec![
                "**/*.tmp".into(),
                "**/~$*".into(),
                "**/Thumbs.db".into(),
                "**/.DS_Store".into(),
                "**/.git/**".into(),
            ],
            verify_by_content: "off".into(),
            mtime_tolerance_sec: 2,
            delete_safety_threshold_pct: 50,
            scheduler_running: true,
            notify_pc: false,
            notify_app: false,
            compact_cards: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub pairs: Vec<SyncPair>,
}

pub fn config_path(app: &AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    dir.join("settings.json")
}

pub fn load(app: &AppHandle) -> Config {
    let path = config_path(app);
    match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
            tracing::warn!("settings.json illisible ({e}), valeurs par défaut");
            Config::default()
        }),
        Err(_) => Config::default(),
    }
}

pub fn save(app: &AppHandle, cfg: &Config) -> Result<(), String> {
    let path = config_path(app);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}
