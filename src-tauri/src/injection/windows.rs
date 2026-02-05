use std::thread;
use std::time::Duration;

use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

use super::{InjectionOptions, TextInjector};
use crate::error::AppError;

pub struct WindowsInjector;

impl WindowsInjector {
    pub fn new() -> Self {
        Self
    }
}

impl TextInjector for WindowsInjector {
    fn inject(&self, text: &str, options: &InjectionOptions) -> Result<(), AppError> {
        let mut clipboard = Clipboard::new()?;

        // Save current clipboard content for restore
        let saved_text = if options.clipboard_restore {
            clipboard.get_text().ok()
        } else {
            None
        };

        // Set text to clipboard
        clipboard
            .set_text(text)
            .map_err(|e| AppError::Injection(format!("Failed to set clipboard: {}", e)))?;

        // Small delay to ensure clipboard is ready
        thread::sleep(Duration::from_millis(50));

        // Simulate Ctrl+V
        let mut enigo = Enigo::new(&Settings::default())
            .map_err(|e| AppError::Injection(format!("Failed to create enigo: {}", e)))?;

        enigo
            .key(Key::Control, Direction::Press)
            .map_err(|e| AppError::Injection(format!("Key press error: {}", e)))?;
        enigo
            .key(Key::Unicode('v'), Direction::Click)
            .map_err(|e| AppError::Injection(format!("Key press error: {}", e)))?;
        enigo
            .key(Key::Control, Direction::Release)
            .map_err(|e| AppError::Injection(format!("Key release error: {}", e)))?;

        // Wait for paste to complete
        thread::sleep(Duration::from_millis(100));

        // Auto-enter if configured
        if options.auto_enter {
            enigo
                .key(Key::Return, Direction::Click)
                .map_err(|e| AppError::Injection(format!("Key press error: {}", e)))?;
            thread::sleep(Duration::from_millis(50));
        }

        // Restore clipboard
        if let Some(saved) = saved_text {
            thread::sleep(Duration::from_millis(50));
            let _ = clipboard.set_text(&saved);
        }

        log::info!("Text injected: {} chars", text.len());
        Ok(())
    }
}
