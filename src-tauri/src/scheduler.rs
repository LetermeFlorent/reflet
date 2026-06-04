use crate::config::{Settings, SyncPair};
use crate::state::{AppState, SyncRequest};
use chrono::Timelike;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};
use tauri::async_runtime::{channel, spawn, spawn_blocking, Sender};
use tauri::{AppHandle, Emitter, Manager};

pub const MIN_INTERVAL: u64 = 5;
const MAX_SLEEP: u64 = 1800;
const MIN_SLEEP: u64 = 3;

fn parse_hhmm(s: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 { return None; }
    let h: u32 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    if h > 23 || m > 59 { return None; }
    Some((h, m))
}

fn secs_until_schedule(times: &[String]) -> Option<u64> {
    let now = chrono::Local::now();
    let now_min = now.hour() as u64 * 60 + now.minute() as u64;
    let mut best: Option<u64> = None;
    for t in times {
        if let Some((h, m)) = parse_hhmm(t) {
            let target_min = h as u64 * 60 + m as u64;
            let diff = if target_min > now_min {
                target_min - now_min
            } else {
                target_min + 1440 - now_min
            };
            let secs = (diff * 60).saturating_sub(now.second() as u64);
            best = Some(best.map_or(secs, |b| b.min(secs)));
        }
    }
    best
}

fn is_schedule_due(times: &[String]) -> bool {
    let now = chrono::Local::now();
    let now_min = now.hour() as u64 * 60 + now.minute() as u64;
    for t in times {
        if let Some((h, m)) = parse_hhmm(t) {
            let target_min = h as u64 * 60 + m as u64;
            // Fenêtre = minute pile OU minute précédente (avec passage de minuit :
            // à 00h00, la minute précédente est 23h59 = 1439).
            let prev_min = if now_min == 0 { 1439 } else { now_min - 1 };
            if now_min == target_min || prev_min == target_min {
                let sec = now.second() as u64;
                if sec < 65 {
                    return true;
                }
            }
        }
    }
    false
}

pub fn start(app: AppHandle) -> Sender<SyncRequest> {
    let (tx, mut rx) = channel::<SyncRequest>(16);
    spawn(async move {
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

pub fn pair_interval(pair: &SyncPair, settings: &Settings) -> u64 {
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

fn compute_sleep(app: &AppHandle) -> Duration {
    let settings = current_settings(app);
    let now = Instant::now();
    let state = app.state::<AppState>();
    let cfg = state.config.lock().unwrap();
    let last = state.last_started.lock().unwrap();

    let mut min_remaining = MAX_SLEEP;
    for p in cfg.pairs.iter().filter(|p| p.enabled) {
        if !p.schedule_times.is_empty() {
            if let Some(secs) = secs_until_schedule(&p.schedule_times) {
                min_remaining = min_remaining.min(secs.max(MIN_SLEEP));
            }
        } else {
            let iv = pair_interval(p, &settings);
            let elapsed = last
                .get(&p.id)
                .map(|t| now.duration_since(*t).as_secs())
                .unwrap_or(0);
            min_remaining = min_remaining.min(iv.saturating_sub(elapsed));
        }
    }
    Duration::from_secs(min_remaining.clamp(MIN_SLEEP, MAX_SLEEP))
}

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
                if !p.schedule_times.is_empty() {
                    if !is_schedule_due(&p.schedule_times) {
                        return false;
                    }
                    match last.get(&p.id) {
                        Some(t) => now.duration_since(*t).as_secs() >= 60,
                        None => {
                            last.insert(p.id.clone(), now);
                            false
                        }
                    }
                } else {
                    let iv = pair_interval(p, &settings);
                    match last.get(&p.id) {
                        Some(t) => now.duration_since(*t).as_secs() >= iv,
                        None => {
                            last.insert(p.id.clone(), now);
                            false
                        }
                    }
                }
            })
            .cloned()
            .collect()
    };
    run_batch(app, due, settings).await;
}

async fn run_all(app: &AppHandle) {
    let pairs: Vec<SyncPair> = {
        let state = app.state::<AppState>();
        let cfg = state.config.lock().unwrap();
        cfg.pairs.iter().filter(|p| p.enabled).cloned().collect()
    };
    let settings = current_settings(app);
    run_batch(app, pairs, settings).await;
}

async fn run_pair(app: &AppHandle, id: &str) {
    let pair = {
        let state = app.state::<AppState>();
        let cfg = state.config.lock().unwrap();
        cfg.pairs.iter().find(|p| p.id == id).cloned()
    };
    let Some(pair) = pair else { return };
    let settings = current_settings(app);
    run_batch(app, vec![pair], settings).await;
}

/// File d'attente à concurrence bornée : chaque paire est dispatchée dans une tâche
/// détachée, le nombre de synchros simultanées étant limité par `sync_sem`. La boucle
/// du scheduler reste réactive pendant les synchros. L'anti-overlap (par paire ET par
/// destination) et la déduplication sont garantis dans `run_pair_task`.
async fn run_batch(app: &AppHandle, pairs: Vec<SyncPair>, settings: Settings) {
    for pair in pairs {
        let app = app.clone();
        let settings = settings.clone();
        spawn(async move {
            run_pair_task(&app, pair, settings).await;
        });
    }
}

fn norm_dest(p: &str) -> String {
    p.replace('/', "\\").trim_end_matches('\\').to_lowercase()
}

/// Deux destinations se chevauchent si elles sont égales ou que l'une contient l'autre.
fn dest_overlap(a: &str, b: &str) -> bool {
    a == b || a.starts_with(&format!("{b}\\")) || b.starts_with(&format!("{a}\\"))
}

async fn run_pair_task(app: &AppHandle, pair: SyncPair, settings: Settings) {
    let nd = norm_dest(&pair.destination);
    {
        let state = app.state::<AppState>();
        let mut active = state.active.lock().unwrap();
        // Déjà en file/en cours pour cette paire → on ne relance pas (dédup + anti-overlap).
        if active.contains_key(&pair.id) {
            return;
        }
        // Destination en conflit avec une synchro active → on réessaiera au prochain tick.
        if active.values().any(|d| dest_overlap(d, &nd)) {
            return;
        }
        let was_empty = active.is_empty();
        active.insert(pair.id.clone(), nd);
        drop(active);
        if was_empty {
            set_busy(app, true);
        }
    }

    // Limite la concurrence : attend un jeton si MAX_CONCURRENT_SYNCS sont déjà en cours.
    let permit = app.state::<AppState>().sync_sem.clone().acquire_owned().await.ok();
    app.state::<AppState>().mark_started(&pair.id);

    let app2 = app.clone();
    let pid = pair.id.clone();
    let _ = spawn_blocking(move || crate::sync::run_sync(&app2, pair, settings)).await;
    drop(permit);

    let state = app.state::<AppState>();
    let mut active = state.active.lock().unwrap();
    active.remove(&pid);
    let empty = active.is_empty();
    drop(active);
    if empty {
        set_busy(app, false);
    }
}
