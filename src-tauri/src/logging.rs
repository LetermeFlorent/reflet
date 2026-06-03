
use tauri::{AppHandle, Manager};

pub fn init(app: &AppHandle) {
    let dir = app
        .path()
        .app_log_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("logs"));
    let _ = std::fs::create_dir_all(&dir);

    let file_appender = tracing_appender::rolling::daily(dir, "reflet.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    Box::leak(Box::new(guard));

    let _ = tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(false)
        .try_init();

    tracing::info!("Reflet démarré");
}
