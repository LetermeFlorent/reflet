//! Worker de synchronisation unique : minuteur PAR PAIRE + requêtes « sync now ».
//!
//! Un seul worker traite une requête à la fois => aucun chevauchement de passes.
//! Chaque paire a son propre intervalle (interval_sec_override), sinon hérite de
//! l'intervalle global des réglages.

use crate::config::{Settings, SyncPair};
use crate::state::{AppState, SyncRequest};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::async_runtime::{channel, spawn, spawn_blocking, Sender};
use tauri::{AppHandle, Emitter, Manager};

const MIN_INTERVAL: u64 = 5;
const MAX_SLEEP: u64 = 1800;
const MIN_SLEEP: u64 = 3;

pub fn start(app: AppHandle) -> Sender<SyncRequest> {
    let (tx, mut rx) = channel::<SyncRequest>(16);
    spawn(async move {
        // Initialise les timers des paires existantes (=> 1er run auto après leur intervalle).
        {
            let state = app.state::<AppState>();
            let cfg = state.config.lock().unwrap();
            let mut last = state.last_started.lock().unwrap();
            let now = Instant::now();
            for p in &cfg.pairs {
                last.entry(p.id.clone()).or_insert(now);
            }
        }

        loop {
            let sleep_for = compute_sleep(&app);
            tokio::select! {
                _ = tokio::time::sleep(sleep_for) => {
                    let running = app.state::<AppState>().scheduler_running.load(Ordering::SeqCst);
                    if running {
                        run_due(&app).await;
                    }
                }
                msg = rx.recv() => {
                    match msg {
                        Some(SyncRequest::All) => run_all(&app).await,
                        Some(SyncRequest::Pair(id)) => run_pair(&app, &id).await,
                        None => break,
                    }
                }
            }
        }
    });
    tx
}

fn current_settings(app: &AppHandle) -> Settings {
    app.state::<AppState>().config.lock().unwrap().settings.clone()
}

fn pair_interval(pair: &SyncPair, settings: &Settings) -> u64 {
    match pair.interval_sec_override {
        Some(v) if v >= MIN_INTERVAL => v,
        _ => settings.interval_sec.max(MIN_INTERVAL),
    }
}

fn set_busy(app: &AppHandle, busy: bool) {
    let state = app.state::<AppState>();
    state.sync_busy.store(busy, Ordering::SeqCst);
    let _ = app.emit("sync:busy", serde_json::json!({ "busy": busy }));
}

/// Temps à dormir avant la prochaine paire due.
fn compute_sleep(app: &AppHandle) -> Duration {
    let settings = current_settings(app);
    let now = Instant::now();
    let state = app.state::<AppState>();
    let cfg = state.config.lock().unwrap();
    let last = state.last_started.lock().unwrap();

    let mut min_remaining = MAX_SLEEP;
    for p in cfg.pairs.iter().filter(|p| p.enabled) {
        let iv = pair_interval(p, &settings);
        let elapsed = last
            .get(&p.id)
            .map(|t| now.duration_since(*t).as_secs())
            .unwrap_or(0);
        min_remaining = min_remaining.min(iv.saturating_sub(elapsed));
    }
    Duration::from_secs(min_remaining.clamp(MIN_SLEEP, MAX_SLEEP))
}

/// Lance les paires dont l'intervalle est écoulé.
async fn run_due(app: &AppHandle) {
    let settings = current_settings(app);
    let now = Instant::now();
    let due: Vec<SyncPair> = {
        let state = app.state::<AppState>();
        let cfg = state.config.lock().unwrap();
        let mut last = state.last_started.lock().unwrap();
        cfg.pairs
            .iter()
            .filter(|p| p.enabled)
            .filter(|p| {
                let iv = pair_interval(p, &settings);
                match last.get(&p.id) {
                    Some(t) => now.duration_since(*t).as_secs() >= iv,
                    None => {
                        // Nouvelle paire : initialise, pas due tout de suite.
                        last.insert(p.id.clone(), now);
                        false
                    }
                }
            })
            .cloned()
            .collect()
    };
    if due.is_empty() {
        return;
    }
    set_busy(app, true);
    for pair in due {
        app.state::<AppState>().mark_started(&pair.id);
        run_one(app, pair, settings.clone()).await;
    }
    set_busy(app, false);
}

async fn run_all(app: &AppHandle) {
    let pairs: Vec<SyncPair> = {
        let state = app.state::<AppState>();
        let cfg = state.config.lock().unwrap();
        cfg.pairs.iter().filter(|p| p.enabled).cloned().collect()
    };
    if pairs.is_empty() {
        return;
    }
    let settings = current_settings(app);
    set_busy(app, true);
    for pair in pairs {
        app.state::<AppState>().mark_started(&pair.id);
        run_one(app, pair, settings.clone()).await;
    }
    set_busy(app, false);
}

async fn run_pair(app: &AppHandle, id: &str) {
    let pair = {
        let state = app.state::<AppState>();
        let cfg = state.config.lock().unwrap();
        cfg.pairs.iter().find(|p| p.id == id).cloned()
    };
    let Some(pair) = pair else { return };
    let settings = current_settings(app);
    app.state::<AppState>().mark_started(&pair.id);
    set_busy(app, true);
    run_one(app, pair, settings).await;
    set_busy(app, false);
}

async fn run_one(app: &AppHandle, pair: SyncPair, settings: Settings) {
    let app2 = app.clone();
    let _ = spawn_blocking(move || crate::sync::run_sync(&app2, pair, settings)).await;
}
