use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrayState {
    Idle,
    Recording,
    Processing,
}

pub fn build_tray(app: &AppHandle) -> Result<(), AppError> {
    let settings_item = MenuItemBuilder::with_id("settings", "Settings")
        .build(app)
        .map_err(|e| AppError::Internal(format!("Failed to build menu item: {}", e)))?;

    let quit_item = MenuItemBuilder::with_id("quit", "Quit")
        .build(app)
        .map_err(|e| AppError::Internal(format!("Failed to build menu item: {}", e)))?;

    let menu = MenuBuilder::new(app)
        .item(&settings_item)
        .separator()
        .item(&quit_item)
        .build()
        .map_err(|e| AppError::Internal(format!("Failed to build menu: {}", e)))?;

    let icon = load_tray_icon(TrayState::Idle)?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .menu(&menu)
        .tooltip("Voxai - Ready")
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "settings" => {
                if let Some(window) = app.get_webview_window("settings") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)
        .map_err(|e| AppError::Internal(format!("Failed to build tray icon: {}", e)))?;

    log::info!("System tray initialized");
    Ok(())
}

pub fn update_tray_icon(app: &AppHandle, state: TrayState) -> Result<(), AppError> {
    let icon = load_tray_icon(state)?;
    let tooltip = match state {
        TrayState::Idle => "Voxai - Ready",
        TrayState::Recording => "Voxai - Recording...",
        TrayState::Processing => "Voxai - Processing...",
    };

    if let Some(tray) = app.tray_by_id("main") {
        tray.set_icon(Some(icon))
            .map_err(|e| AppError::Internal(format!("Failed to set tray icon: {}", e)))?;
        tray.set_tooltip(Some(tooltip))
            .map_err(|e| AppError::Internal(format!("Failed to set tooltip: {}", e)))?;
    }

    Ok(())
}

fn load_tray_icon(state: TrayState) -> Result<Image<'static>, AppError> {
    let icon_bytes: &[u8] = match state {
        TrayState::Idle => include_bytes!("../../icons/tray-idle.png"),
        TrayState::Recording => include_bytes!("../../icons/tray-recording.png"),
        TrayState::Processing => include_bytes!("../../icons/tray-processing.png"),
    };

    Image::from_bytes(icon_bytes)
        .map_err(|e| AppError::Internal(format!("Failed to load tray icon: {}", e)))
}
