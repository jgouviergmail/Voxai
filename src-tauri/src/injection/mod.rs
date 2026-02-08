use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[cfg(target_os = "windows")]
pub mod windows;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionOptions {
    pub auto_enter: bool,
    pub clipboard_restore: bool,
}

impl Default for InjectionOptions {
    fn default() -> Self {
        Self {
            auto_enter: false,
            clipboard_restore: true,
        }
    }
}

/// Trait for injecting text into the focused application.
/// All methods are sync — clipboard and key simulation are synchronous operations.
pub trait TextInjector: Send + Sync {
    fn inject(&self, text: &str, options: &InjectionOptions) -> Result<(), AppError>;

    /// Copy the currently selected text via Ctrl+C.
    /// Returns (copied_text, saved_clipboard) where saved_clipboard is the previous content.
    fn copy_selection(&self) -> Result<(String, Option<String>), AppError> {
        Err(AppError::Internal(
            "copy_selection not supported on this platform".into(),
        ))
    }

    /// Replace the current selection with `text` via Ctrl+V, then restore saved clipboard.
    fn replace_selection(&self, _text: &str, _saved: Option<String>) -> Result<(), AppError> {
        Err(AppError::Internal(
            "replace_selection not supported on this platform".into(),
        ))
    }

    /// Inject text at cursor via clipboard+paste, WITHOUT Enter and WITHOUT clipboard restore.
    /// Used by streaming mode — clipboard is saved/restored at session level, not per-segment.
    fn inject_no_enter(&self, text: &str) -> Result<(), AppError> {
        self.inject(
            text,
            &InjectionOptions {
                auto_enter: false,
                clipboard_restore: false,
            },
        )
    }
}

pub fn create_injector(is_simulating: Arc<AtomicBool>) -> Box<dyn TextInjector> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsInjector::new(is_simulating))
    }
    #[cfg(target_os = "macos")]
    {
        let _ = is_simulating;
        todo!("macOS injector not yet implemented")
    }
    #[cfg(target_os = "linux")]
    {
        let _ = is_simulating;
        todo!("Linux injector not yet implemented")
    }
}
