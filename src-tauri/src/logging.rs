use std::fs;
use std::path::PathBuf;

/// Maximum log file size before rotation (10 MB).
const MAX_LOG_SIZE: u64 = 10 * 1024 * 1024;

/// Returns the Voxai log directory (~/.config/Voxai/ on Windows = %APPDATA%/Voxai/).
fn log_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("Voxai"))
}

/// Returns the main log file path.
pub fn log_path() -> Option<PathBuf> {
    log_dir().map(|d| d.join("voxai.log"))
}

/// Rotate log files: voxai.log → voxai.log.prev (keeps 1 previous session).
/// If the current log exceeds MAX_LOG_SIZE, it is rotated on startup.
fn rotate_logs(log_file: &PathBuf) {
    if let Ok(meta) = fs::metadata(log_file) {
        if meta.len() > MAX_LOG_SIZE {
            let prev = log_file.with_extension("log.prev");
            let _ = fs::remove_file(&prev);
            let _ = fs::rename(log_file, &prev);
        }
    }
}

/// Initialize the production logging system.
///
/// - **File logging**: all levels ≥ INFO written to `%APPDATA%/Voxai/voxai.log`
///   with timestamps, level, module target. Voxai modules logged at DEBUG.
/// - **Log rotation**: if the log exceeds 10 MB at startup, it's renamed to `.prev`.
/// - **Console output**: in debug builds only, for developer convenience.
/// - **Graceful fallback**: if file logging fails, falls back to stderr-only.
///
/// Uses the `log` facade — all existing `log::info!`, `log::warn!`, `log::error!`
/// calls work unchanged.
pub fn setup_logging() {
    let file_dispatch = log_dir().and_then(|dir| {
        fs::create_dir_all(&dir).ok()?;
        let log_file = dir.join("voxai.log");
        rotate_logs(&log_file);

        let file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .ok()?;

        Some(
            fern::Dispatch::new()
                .level(log::LevelFilter::Info)
                .level_for("voxai_lib", log::LevelFilter::Debug)
                .chain(file),
        )
    });

    let mut dispatch = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {:5} {}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                record.level(),
                record.target(),
                message
            ))
        })
        // Filter noisy third-party crates
        .level(log::LevelFilter::Warn)
        .level_for("voxai_lib", log::LevelFilter::Debug)
        .level_for("voxai", log::LevelFilter::Debug);

    // File output (always, if available)
    if let Some(file_dispatch) = file_dispatch {
        dispatch = dispatch.chain(file_dispatch);
    }

    // Console output in debug builds only (release has no console on Windows)
    #[cfg(debug_assertions)]
    {
        dispatch = dispatch.chain(
            fern::Dispatch::new()
                .level(log::LevelFilter::Info)
                .level_for("voxai_lib", log::LevelFilter::Debug)
                .chain(std::io::stderr()),
        );
    }

    if let Err(e) = dispatch.apply() {
        eprintln!("Failed to initialize logging: {e}");
    }
}
