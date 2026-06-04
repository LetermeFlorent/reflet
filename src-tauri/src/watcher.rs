use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use notify::{Event, RecursiveMode, Watcher};
use tauri::async_runtime::Sender;

use crate::state::SyncRequest;

pub struct WatcherManager {
    watchers: HashMap<String, notify::RecommendedWatcher>,
    debounce: Arc<Mutex<HashMap<String, Instant>>>,
    sync_tx: Sender<SyncRequest>,
}

impl WatcherManager {
    pub fn new(sync_tx: Sender<SyncRequest>) -> Self {
        Self {
            watchers: HashMap::new(),
            debounce: Arc::new(Mutex::new(HashMap::new())),
            sync_tx,
        }
    }

    pub fn start(&mut self, pair_id: &str, source: &str) {
        if self.watchers.contains_key(pair_id) {
            return;
        }
        let pair_id_owned = pair_id.to_string();
        let debounce = self.debounce.clone();
        let sync_tx = self.sync_tx.clone();

        let (event_tx, event_rx) = std::sync::mpsc::channel::<Result<Event, notify::Error>>();

        let mut watcher = match notify::RecommendedWatcher::new(event_tx, notify::Config::default())
        {
            Ok(w) => w,
            Err(e) => {
                tracing::warn!("Watcher init pour {pair_id_owned} : {e}");
                return;
            }
        };

        if let Err(e) = watcher.watch(Path::new(source), RecursiveMode::Recursive) {
            tracing::warn!("Watcher watch {source} : {e}");
            return;
        }

        let p_id = pair_id_owned.clone();
        let builder = std::thread::Builder::new().name(format!("watch-{p_id}"));
        let _ = builder.spawn(move || {
            for result in event_rx {
                if result.is_err() {
                    continue;
                }
                let now = Instant::now();
                let mut map = debounce.lock().unwrap_or_else(|e| e.into_inner());
                let should_trigger = match map.get(&p_id) {
                    Some(last) => now.duration_since(*last) > Duration::from_secs(3),
                    None => true,
                };
                if !should_trigger {
                    continue;
                }
                map.insert(p_id.clone(), now);
                drop(map);
                if let Err(e) = sync_tx.try_send(SyncRequest::Pair(p_id.clone())) {
                    tracing::debug!("watcher {p_id} : declenchement non transmis ({e})");
                }
            }
        });

        self.watchers.insert(pair_id_owned, watcher);
    }

    pub fn stop(&mut self, pair_id: &str) {
        self.watchers.remove(pair_id);
        self.debounce
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(pair_id);
    }

    pub fn stop_all(&mut self) {
        self.watchers.clear();
        self.debounce.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }
}
