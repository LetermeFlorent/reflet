
use crate::config::Config;
use crate::watcher::WatcherManager;
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::async_runtime::Sender;
use tokio::sync::Semaphore;

pub const MAX_LOGS: usize = 300;
/// Nombre de paires synchronisées simultanément (file d'attente à concurrence bornée).
pub const MAX_CONCURRENT_SYNCS: usize = 2;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub at: String,
    pub level: String,
    pub pair_id: Option<String>,
    pub action: String,
    pub path: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum SyncRequest {
    Pair(String),
    All,
}

pub struct AppState {
    pub config: Mutex<Config>,
    pub logs: Mutex<VecDeque<LogEntry>>,
    pub statuses: Mutex<HashMap<String, String>>,
    pub scheduler_running: AtomicBool,
    pub really_quit: AtomicBool,
    pub sync_busy: Arc<AtomicBool>,
    pub sync_tx: Mutex<Option<Sender<SyncRequest>>>,
    pub last_started: Mutex<HashMap<String, Instant>>,
    pub watcher_manager: Mutex<Option<WatcherManager>>,
    /// Paires en file/en cours de synchro : id → destination normalisée (anti-overlap + dédup).
    pub active: Mutex<HashMap<String, String>>,
    /// Limite la concurrence des synchros de paires.
    pub sync_sem: Arc<Semaphore>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let scheduler_running = config.settings.scheduler_running;
        AppState {
            config: Mutex::new(config),
            logs: Mutex::new(VecDeque::with_capacity(256)),
            statuses: Mutex::new(HashMap::new()),
            scheduler_running: AtomicBool::new(scheduler_running),
            really_quit: AtomicBool::new(false),
            sync_busy: Arc::new(AtomicBool::new(false)),
            sync_tx: Mutex::new(None),
            last_started: Mutex::new(HashMap::new()),
            watcher_manager: Mutex::new(None),
            active: Mutex::new(HashMap::new()),
            sync_sem: Arc::new(Semaphore::new(MAX_CONCURRENT_SYNCS)),
        }
    }

    pub fn mark_started(&self, pair_id: &str) {
        self.last_started
            .lock()
            .unwrap()
            .insert(pair_id.to_string(), Instant::now());
    }

    pub fn push_log(&self, entry: LogEntry) {
        let mut logs = self.logs.lock().unwrap();
        if logs.len() >= MAX_LOGS {
            logs.pop_front();
        }
        logs.push_back(entry);
    }

    pub fn set_status(&self, pair_id: &str, status: &str) {
        self.statuses
            .lock()
            .unwrap()
            .insert(pair_id.to_string(), status.to_string());
    }
}
