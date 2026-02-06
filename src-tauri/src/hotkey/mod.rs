pub mod keyboard_hook;

#[derive(Debug, Clone)]
pub enum HotkeyEvent {
    RecordStart,
    RecordStop,
    TextProcess,
}

/// Re-export start_listener from keyboard_hook
pub use keyboard_hook::start_listener;
