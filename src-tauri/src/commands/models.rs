use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::State;

use crate::app_state::AppState;
use crate::error::AppError;
use crate::models::{downloader, registry};

#[tauri::command]
pub fn list_models(state: State<'_, AppState>) -> Result<Vec<ModelInfo>, AppError> {
    let cache = state
        .model_cache
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let downloaded = cache.list_downloaded();

    let models: Vec<ModelInfo> = registry::MODEL_CATALOG
        .iter()
        .map(|def| ModelInfo {
            id: def.id.to_string(),
            name: def.name.to_string(),
            size_mb: def.size_mb,
            description: def.description.to_string(),
            downloaded: downloaded.contains(&def.id.to_string()),
        })
        .collect();

    Ok(models)
}

#[tauri::command]
pub async fn download_model(
    app: tauri::AppHandle,
    model_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    // Create cancel token and register it
    let cancel_token = Arc::new(AtomicBool::new(false));
    {
        let mut downloads = state
            .active_downloads
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        if downloads.contains_key(&model_id) {
            return Err(AppError::Model(format!(
                "Download already in progress for {}",
                model_id
            )));
        }
        downloads.insert(model_id.clone(), Arc::clone(&cancel_token));
    }

    let result = downloader::download_model(&app, &model_id, cancel_token).await;

    // Always remove from active downloads
    {
        let mut downloads = state
            .active_downloads
            .lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        downloads.remove(&model_id);
    }

    let path = result?;

    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let mut cache = state
        .model_cache
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    cache.register_download(&model_id, size)?;

    Ok(())
}

#[tauri::command]
pub fn cancel_download(model_id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let downloads = state
        .active_downloads
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if let Some(token) = downloads.get(&model_id) {
        token.store(true, Ordering::Relaxed);
        log::info!("Cancellation requested for download: {}", model_id);
    }
    Ok(())
}

#[tauri::command]
pub fn delete_model(model_id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let mut cache = state
        .model_cache
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    cache.remove_model(&model_id)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub size_mb: u64,
    pub description: String,
    pub downloaded: bool,
}
