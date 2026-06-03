mod commands;
mod compression;
mod config;
mod lifecycle;
mod logging;
mod scheduler;
mod state;
mod sync;
mod tray;
mod watcher;

use state::AppState;
use std::sync::atomic::Ordering;
use tauri::Manager;
use watcher::WatcherManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }));
    }

    builder = builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init());

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ));
    }

    builder
        .setup(|app| {
            let handle = app.handle().clone();
            logging::init(&handle);

            let cfg = config::load(&handle);
            app.manage(AppState::new(cfg));

            let tx = scheduler::start(handle.clone());
            {
                let state = handle.state::<AppState>();
                *state.sync_tx.lock().unwrap() = Some(tx.clone());
                let mut wm = WatcherManager::new(tx);
                let cfg = state.config.lock().unwrap();
                for p in &cfg.pairs {
                    if p.enabled && p.watch_realtime {
                        wm.start(&p.id, &p.source);
                    }
                }
                *state.watcher_manager.lock().unwrap() = Some(wm);
            }

            tray::build(&handle)?;

            let enable_autostart = handle
                .state::<AppState>()
                .config
                .lock()
                .unwrap()
                .settings
                .autostart;
            commands::apply_autostart(&handle, enable_autostart);

            let autostarted = std::env::args().any(|a| a == "--autostart");
            let start_min = handle
                .state::<AppState>()
                .config
                .lock()
                .unwrap()
                .settings
                .start_minimized;
            if autostarted && start_min {
                if let Some(w) = handle.get_webview_window("main") {
                    let _ = w.hide();
                }
            }

            Ok(())
        })
        .on_window_event(lifecycle::on_window_event)
        .invoke_handler(tauri::generate_handler![
            commands::get_app_state,
            commands::get_settings,
            commands::update_settings,
            commands::set_scheduler_running,
            commands::add_pair,
            commands::update_pair,
            commands::delete_pair,
            commands::set_pair_enabled,
            commands::set_pair_watch_realtime,
            commands::reorder_pairs,
            commands::sync_now,
            commands::sync_all,
            commands::dry_run,
            commands::get_logs,
            commands::clear_logs,
            commands::show_window,
            commands::hide_window,
            commands::quit_app,
            commands::detect_compression_methods
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                let state = app.state::<AppState>();
                if !state.really_quit.load(Ordering::SeqCst) {
                    api.prevent_exit();
                }
            }
        });
}
