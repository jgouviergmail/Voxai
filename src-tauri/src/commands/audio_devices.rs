use crate::audio::capture::{list_input_devices, InputDeviceInfo};
use crate::error::AppError;

#[tauri::command]
pub fn list_audio_devices() -> Result<Vec<InputDeviceInfo>, AppError> {
    list_input_devices()
}
