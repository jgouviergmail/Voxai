use std::fs;
use std::path::PathBuf;

use crate::config::schema::AppConfig;
use crate::error::AppError;

fn config_dir() -> Result<PathBuf, AppError> {
    let dir = dirs::config_dir()
        .ok_or_else(|| AppError::Config("Cannot determine config directory".to_string()))?
        .join("Voxai");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn config_path() -> Result<PathBuf, AppError> {
    Ok(config_dir()?.join("config.json"))
}

pub fn load_config() -> Result<AppConfig, AppError> {
    let path = config_path()?;
    if !path.exists() {
        let config = AppConfig::default();
        save_config(&config)?;
        return Ok(config);
    }
    let content = fs::read_to_string(&path)?;
    let config: AppConfig = match serde_json::from_str(&content) {
        Ok(c) => c,
        Err(e) => {
            log::warn!(
                "Config file corrupted or outdated ({}), resetting to defaults",
                e
            );
            let config = AppConfig::default();
            save_config(&config)?;
            config
        }
    };
    Ok(config)
}

pub fn save_config(config: &AppConfig) -> Result<(), AppError> {
    let path = config_path()?;
    let content = serde_json::to_string_pretty(config)?;
    fs::write(&path, content)?;
    Ok(())
}

/// Saves config to disk and emits `settings-updated` so the frontend re-fetches.
pub fn save_and_notify(config: &AppConfig, app_handle: &tauri::AppHandle) -> Result<(), AppError> {
    use tauri::Emitter;
    save_config(config)?;
    app_handle.emit(crate::events::EVENT_SETTINGS_UPDATED, ()).ok();
    Ok(())
}
