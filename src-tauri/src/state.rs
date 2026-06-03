//! État applicatif partagé (config, logs en mémoire, statuts runtime, canal sync).

use crate::config::Config;
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use std::time::Instant;
use tauri::async_runtime::Sender;

pub const MAX_LOGS: usize = 3000;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub at: String,
    pub level: String,           // info | warn | error
    pub pair_id: Option<String>, // paire concernée
    pub action: String,          // copy | update | delete | skip | error | info
    pub path: Option<String>,
    pub message: String,
}

/// Requête envoyée au worker de synchronisation.
#[derive(Debug, Clone)]
pub enum SyncRequest {
    /// Synchroniser une paire précise immédiatement.
    Pair(String),
    /// Synchroniser toutes les paires actives immédiatement.
    All,
}

pub struct AppState {
    pub config: Mutex<Config>,
    pub logs: Mutex<VecDeque<LogEntry>>,
    /// pair_id -> statut courant (idle | syncing | error | disabled)
    pub statuses: Mutex<HashMap<String, String>>,
    pub scheduler_running: AtomicBool,
    pub really_quit: AtomicBool,
    pub sync_busy: AtomicBool,
    pub sync_tx: Mutex<Option<Sender<SyncRequest>>>,
    /// pair_id -> instant du dernier déclenchement (pour l'intervalle par paire).
    pub last_started: Mutex<HashMap<String, Instant>>,
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
            sync_busy: AtomicBool::new(false),
            sync_tx: Mutex::new(None),
            last_started: Mutex::new(HashMap::new()),
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
