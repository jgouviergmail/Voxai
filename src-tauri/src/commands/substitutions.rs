use tauri::State;

use crate::app_state::AppState;
use crate::config::persistence;
use crate::config::schema::SubstitutionRule;
use crate::error::AppError;

#[tauri::command]
pub fn get_substitutions(state: State<'_, AppState>) -> Result<Vec<SubstitutionRule>, AppError> {
    let config = state
        .config
        .read()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(config.postprocessing.substitutions.clone())
}

#[tauri::command]
pub fn add_substitution(
    rule: SubstitutionRule,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let mut config = state
        .config
        .write()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    config.postprocessing.substitutions.push(rule);
    persistence::save_and_notify(&config, &state.app_handle)?;
    Ok(())
}

#[tauri::command]
pub fn delete_substitution(index: usize, state: State<'_, AppState>) -> Result<(), AppError> {
    let mut config = state
        .config
        .write()
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if index >= config.postprocessing.substitutions.len() {
        return Err(AppError::Config(format!(
            "Substitution index {} out of range",
            index
        )));
    }

    config.postprocessing.substitutions.remove(index);
    persistence::save_and_notify(&config, &state.app_handle)?;
    Ok(())
}
