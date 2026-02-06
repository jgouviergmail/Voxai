use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

use super::{InjectionOptions, TextInjector};
use crate::error::AppError;

pub struct WindowsInjector {
    is_simulating: Arc<AtomicBool>,
}

impl WindowsInjector {
    pub fn new(is_simulating: Arc<AtomicBool>) -> Self {
        Self { is_simulating }
    }

    /// Simulate a key combo (e.g. Ctrl+C, Ctrl+V) with the is_simulating guard.
    fn simulate_combo(&self, char_key: char) -> Result<(), AppError> {
        self.is_simulating.store(true, Ordering::SeqCst);
        let result = (|| {
            let mut enigo = Enigo::new(&Settings::default())
                .map_err(|e| AppError::Injection(format!("Failed to create enigo: {}", e)))?;
            enigo
                .key(Key::Control, Direction::Press)
                .map_err(|e| AppError::Injection(format!("Key press error: {}", e)))?;
            enigo
                .key(Key::Unicode(char_key), Direction::Click)
                .map_err(|e| AppError::Injection(format!("Key press error: {}", e)))?;
            enigo
                .key(Key::Control, Direction::Release)
                .map_err(|e| AppError::Injection(format!("Key release error: {}", e)))?;
            Ok(())
        })();
        self.is_simulating.store(false, Ordering::SeqCst);
        result
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

        // Simulate Ctrl+V (with is_simulating guard)
        self.simulate_combo('v')?;

        // Wait for paste to complete
        thread::sleep(Duration::from_millis(100));

        // Auto-enter if configured
        if options.auto_enter {
            self.is_simulating.store(true, Ordering::SeqCst);
            let enter_result: Result<(), AppError> = (|| {
                let mut enigo = Enigo::new(&Settings::default())
                    .map_err(|e| AppError::Injection(format!("Failed to create enigo: {}", e)))?;
                enigo
                    .key(Key::Return, Direction::Click)
                    .map_err(|e| AppError::Injection(format!("Key press error: {}", e)))?;
                Ok(())
            })();
            self.is_simulating.store(false, Ordering::SeqCst);
            enter_result?;
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

    fn copy_selection(&self) -> Result<(String, Option<String>), AppError> {
        let mut clipboard = Clipboard::new()?;

        // Save current clipboard content
        let saved = clipboard.get_text().ok();

        // Clear clipboard so we can detect if Ctrl+C actually copied something
        let _ = clipboard.clear();
        thread::sleep(Duration::from_millis(50));

        // Simulate Ctrl+C
        self.simulate_combo('c')?;

        // Wait for copy to take effect
        thread::sleep(Duration::from_millis(150));

        // Read what was copied
        let copied = clipboard.get_text().ok().filter(|t| !t.is_empty());

        match copied {
            Some(text) => {
                log::info!("Copied selection: {} chars", text.len());
                Ok((text, saved))
            }
            None => Err(AppError::Injection("No text selected or copy failed".into())),
        }
    }

    fn replace_selection(&self, text: &str, saved: Option<String>) -> Result<(), AppError> {
        let mut clipboard = Clipboard::new()?;

        // Set result to clipboard
        clipboard
            .set_text(text)
            .map_err(|e| AppError::Injection(format!("Failed to set clipboard: {}", e)))?;
        thread::sleep(Duration::from_millis(50));

        // Simulate Ctrl+V to replace selection
        self.simulate_combo('v')?;

        // Wait for paste
        thread::sleep(Duration::from_millis(150));

        // Restore original clipboard content
        if let Some(saved_text) = saved {
            thread::sleep(Duration::from_millis(50));
            let _ = clipboard.set_text(&saved_text);
        }

        log::info!("Selection replaced: {} chars", text.len());
        Ok(())
    }
}
