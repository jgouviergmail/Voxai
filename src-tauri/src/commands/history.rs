use tauri::State;

use crate::app_state::AppState;
use crate::error::AppError;
use crate::history::store::HistoryEntry;

#[tauri::command]
pub fn get_history(state: State<'_, AppState>) -> Result<Vec<HistoryEntry>, AppError> {
    let history = state
        .history
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(history.get_all().to_vec())
}

#[tauri::command]
pub fn clear_history(state: State<'_, AppState>) -> Result<(), AppError> {
    let mut history = state
        .history
        .lock()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    history.clear()?;
    log::info!("History cleared");
    Ok(())
}
