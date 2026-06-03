
use crate::state::AppState;
use std::sync::atomic::Ordering;
use tauri::{Manager, Window, WindowEvent};

pub fn on_window_event(window: &Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        let app = window.app_handle();
        let state = app.state::<AppState>();
        if !state.really_quit.load(Ordering::SeqCst) {
            api.prevent_close();
            let _ = window.hide();
        }
    }
}
