use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, RwLock};
use std::thread;

use rdev::{grab, Event, EventType, Key};

use super::HotkeyEvent;
use crate::config::schema::HotkeyConfig;

/// Starts the global keyboard hook in a dedicated thread.
///
/// Uses `rdev::grab` (low-level hook) to **suppress** hotkey key events so they
/// never reach the target application — this prevents e.g. Space being typed
/// into the focused window when the push-to-talk hotkey is Shift+Space.
///
/// `is_simulating` skips normal processing during simulated keystrokes (Ctrl+V
/// via enigo), but we still detect the push-to-talk key release during
/// simulation so that RecordStop is never missed.
pub fn start_listener(
    hotkey: Arc<RwLock<HotkeyConfig>>,
    text_hotkey: Arc<RwLock<Option<HotkeyConfig>>>,
    is_simulating: Arc<AtomicBool>,
) -> mpsc::Receiver<HotkeyEvent> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let modifiers = ModifierState::default();
        // Tracks whether push-to-talk recording is active so we can detect the
        // main-key release regardless of modifier state (e.g. user releases
        // Shift before Space).
        let recording_active = Cell::new(false);

        grab(move |event: Event| -> Option<Event> {
            // --- While simulating keystrokes (Ctrl+V via enigo) ---
            // Skip normal hotkey matching but still detect the push-to-talk
            // key release so RecordStop is never lost.  Modifier state is NOT
            // updated here to avoid corruption from simulated Ctrl presses
            // (the simulation window is <200ms; state self-corrects on the
            // next real modifier event).
            if is_simulating.load(Ordering::SeqCst) {
                match event.event_type {
                    EventType::KeyRelease(key) => {
                        if recording_active.get() {
                            if let Ok(config) = hotkey.read() {
                                if key_matches_config_key(&key, &config.key) {
                                    recording_active.set(false);
                                    let _ = tx.send(HotkeyEvent::RecordStop);
                                    return None; // suppress
                                }
                            }
                        }
                    }
                    EventType::KeyPress(key) => {
                        // Suppress repeated hotkey presses during simulation
                        if recording_active.get() {
                            if let Ok(config) = hotkey.read() {
                                if key_matches_config_key(&key, &config.key) {
                                    return None; // suppress repeat
                                }
                            }
                        }
                    }
                    _ => {}
                }
                return Some(event); // pass through all other events
            }

            // --- Normal processing ---
            match event.event_type {
                EventType::KeyPress(key) => {
                    update_modifier_state(&modifiers, &key, true);

                    // Push-to-talk hotkey — suppress key from reaching app
                    if let Ok(config) = hotkey.read() {
                        if matches_hotkey(&key, &modifiers, &config) {
                            if !recording_active.get() {
                                recording_active.set(true);
                                let _ = tx.send(HotkeyEvent::RecordStart);
                            }
                            return None; // suppress (including repeats)
                        }
                    }

                    // Text processing hotkey (fire on press, no release needed)
                    if let Ok(guard) = text_hotkey.read() {
                        if let Some(ref text_cfg) = *guard {
                            if matches_hotkey(&key, &modifiers, text_cfg) {
                                let _ = tx.send(HotkeyEvent::TextProcess);
                                return None; // suppress
                            }
                        }
                    }
                }
                EventType::KeyRelease(key) => {
                    // Check for hotkey release BEFORE updating modifier state
                    // so Shift is still seen as pressed when releasing Space.
                    // Only check main key — ignore modifiers so that releasing
                    // Shift before Space still triggers stop.
                    if recording_active.get() {
                        if let Ok(config) = hotkey.read() {
                            if key_matches_config_key(&key, &config.key) {
                                recording_active.set(false);
                                let _ = tx.send(HotkeyEvent::RecordStop);
                                update_modifier_state(&modifiers, &key, false);
                                return None; // suppress
                            }
                        }
                    }

                    update_modifier_state(&modifiers, &key, false);
                }
                _ => {}
            }

            Some(event) // pass through all non-hotkey events
        })
        .expect("Failed to start keyboard hook");
    });

    log::info!("Hotkey listener started");
    rx
}

#[derive(Default)]
struct ModifierState {
    ctrl: Cell<bool>,
    shift: Cell<bool>,
    alt: Cell<bool>,
    meta: Cell<bool>,
}

fn update_modifier_state(state: &ModifierState, key: &Key, pressed: bool) {
    match key {
        Key::ControlLeft | Key::ControlRight => state.ctrl.set(pressed),
        Key::ShiftLeft | Key::ShiftRight => state.shift.set(pressed),
        Key::Alt | Key::AltGr => state.alt.set(pressed),
        Key::MetaLeft | Key::MetaRight => state.meta.set(pressed),
        _ => {}
    }
}

/// Check whether `key` matches the config key name, ignoring modifiers.
/// Used for detecting the push-to-talk main-key release regardless of
/// modifier state (the user may release Shift before Space).
fn key_matches_config_key(key: &Key, config_key: &str) -> bool {
    match key {
        Key::Space => config_key == "Space",
        Key::F1 => config_key == "F1",
        Key::F2 => config_key == "F2",
        Key::F3 => config_key == "F3",
        Key::F4 => config_key == "F4",
        Key::F5 => config_key == "F5",
        Key::F6 => config_key == "F6",
        Key::F7 => config_key == "F7",
        Key::F8 => config_key == "F8",
        Key::F9 => config_key == "F9",
        Key::F10 => config_key == "F10",
        Key::F11 => config_key == "F11",
        Key::F12 => config_key == "F12",
        Key::KeyA => config_key == "A",
        Key::KeyB => config_key == "B",
        Key::KeyC => config_key == "C",
        Key::KeyD => config_key == "D",
        Key::KeyE => config_key == "E",
        Key::KeyF => config_key == "F",
        Key::KeyG => config_key == "G",
        Key::KeyH => config_key == "H",
        Key::KeyI => config_key == "I",
        Key::KeyJ => config_key == "J",
        Key::KeyK => config_key == "K",
        Key::KeyL => config_key == "L",
        Key::KeyM => config_key == "M",
        Key::KeyN => config_key == "N",
        Key::KeyO => config_key == "O",
        Key::KeyP => config_key == "P",
        Key::KeyQ => config_key == "Q",
        Key::KeyR => config_key == "R",
        Key::KeyS => config_key == "S",
        Key::KeyT => config_key == "T",
        Key::KeyU => config_key == "U",
        Key::KeyV => config_key == "V",
        Key::KeyW => config_key == "W",
        Key::KeyX => config_key == "X",
        Key::KeyY => config_key == "Y",
        Key::KeyZ => config_key == "Z",
        Key::Num0 => config_key == "0",
        Key::Num1 => config_key == "1",
        Key::Num2 => config_key == "2",
        Key::Num3 => config_key == "3",
        Key::Num4 => config_key == "4",
        Key::Num5 => config_key == "5",
        Key::Num6 => config_key == "6",
        Key::Num7 => config_key == "7",
        Key::Num8 => config_key == "8",
        Key::Num9 => config_key == "9",
        _ => false,
    }
}

fn matches_hotkey(key: &Key, modifiers: &ModifierState, config: &HotkeyConfig) -> bool {
    if !key_matches_config_key(key, &config.key) {
        return false;
    }

    let needs_ctrl = config.modifiers.iter().any(|m| m == "Control");
    let needs_shift = config.modifiers.iter().any(|m| m == "Shift");
    let needs_alt = config.modifiers.iter().any(|m| m == "Alt");
    let needs_meta = config.modifiers.iter().any(|m| m == "Meta");

    modifiers.ctrl.get() == needs_ctrl
        && modifiers.shift.get() == needs_shift
        && modifiers.alt.get() == needs_alt
        && modifiers.meta.get() == needs_meta
}
