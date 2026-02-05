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
}

pub fn create_injector() -> Box<dyn TextInjector> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsInjector::new())
    }
    #[cfg(target_os = "macos")]
    {
        todo!("macOS injector not yet implemented")
    }
    #[cfg(target_os = "linux")]
    {
        todo!("Linux injector not yet implemented")
    }
}
