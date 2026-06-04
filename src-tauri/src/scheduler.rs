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
            let secs = diff * 60 - now.second() as u64;
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
            if now_min == target_min || (now_min > 0 && now_min - 1 == target_min) {
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
