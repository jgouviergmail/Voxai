use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, RwLock};
use std::thread;

use rdev::{listen, Event, EventType, Key};

use super::HotkeyEvent;
use crate::config::schema::HotkeyConfig;

/// Starts the global keyboard listener in a dedicated thread.
/// Reads hotkey config from shared Arc<RwLock>s so they can be updated at runtime
/// without restarting the listener thread.
/// `is_simulating` prevents processing of simulated keystrokes (from enigo).
pub fn start_listener(
    hotkey: Arc<RwLock<HotkeyConfig>>,
    text_hotkey: Arc<RwLock<Option<HotkeyConfig>>>,
    is_simulating: Arc<AtomicBool>,
) -> mpsc::Receiver<HotkeyEvent> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let mut modifiers_pressed = ModifierState::default();

        listen(move |event: Event| {
            // Skip all processing while we are simulating keystrokes (Ctrl+C/V)
            if is_simulating.load(Ordering::SeqCst) {
                return;
            }

            match event.event_type {
                EventType::KeyPress(key) => {
                    update_modifier_state(&mut modifiers_pressed, &key, true);

                    // Push-to-talk hotkey
                    if let Ok(config) = hotkey.read() {
                        if matches_hotkey(&key, &modifiers_pressed, &config) {
                            let _ = tx.send(HotkeyEvent::RecordStart);
                        }
                    }

                    // Text processing hotkey (fire on press, no release needed)
                    if let Ok(guard) = text_hotkey.read() {
                        if let Some(ref text_cfg) = *guard {
                            if matches_hotkey(&key, &modifiers_pressed, text_cfg) {
                                let _ = tx.send(HotkeyEvent::TextProcess);
                            }
                        }
                    }
                }
                EventType::KeyRelease(key) => {
                    if let Ok(config) = hotkey.read() {
                        if matches_hotkey(&key, &modifiers_pressed, &config) {
                            let _ = tx.send(HotkeyEvent::RecordStop);
                        }
                    }

                    update_modifier_state(&mut modifiers_pressed, &key, false);
                }
                _ => {}
            }
        })
        .expect("Failed to start keyboard listener");
    });

    log::info!("Hotkey listener started");
    rx
}

#[derive(Default)]
struct ModifierState {
    ctrl: bool,
    shift: bool,
    alt: bool,
    meta: bool,
}

fn update_modifier_state(state: &mut ModifierState, key: &Key, pressed: bool) {
    match key {
        Key::ControlLeft | Key::ControlRight => state.ctrl = pressed,
        Key::ShiftLeft | Key::ShiftRight => state.shift = pressed,
        Key::Alt | Key::AltGr => state.alt = pressed,
        Key::MetaLeft | Key::MetaRight => state.meta = pressed,
        _ => {}
    }
}

fn matches_hotkey(key: &Key, modifiers: &ModifierState, config: &HotkeyConfig) -> bool {
    let key_matches = match key {
        Key::Space => config.key == "Space",
        Key::F1 => config.key == "F1",
        Key::F2 => config.key == "F2",
        Key::F3 => config.key == "F3",
        Key::F4 => config.key == "F4",
        Key::F5 => config.key == "F5",
        Key::F6 => config.key == "F6",
        Key::F7 => config.key == "F7",
        Key::F8 => config.key == "F8",
        Key::F9 => config.key == "F9",
        Key::F10 => config.key == "F10",
        Key::F11 => config.key == "F11",
        Key::F12 => config.key == "F12",
        Key::KeyA => config.key == "A",
        Key::KeyB => config.key == "B",
        Key::KeyC => config.key == "C",
        Key::KeyD => config.key == "D",
        Key::KeyE => config.key == "E",
        Key::KeyF => config.key == "F",
        Key::KeyG => config.key == "G",
        Key::KeyH => config.key == "H",
        Key::KeyI => config.key == "I",
        Key::KeyJ => config.key == "J",
        Key::KeyK => config.key == "K",
        Key::KeyL => config.key == "L",
        Key::KeyM => config.key == "M",
        Key::KeyN => config.key == "N",
        Key::KeyO => config.key == "O",
        Key::KeyP => config.key == "P",
        Key::KeyQ => config.key == "Q",
        Key::KeyR => config.key == "R",
        Key::KeyS => config.key == "S",
        Key::KeyT => config.key == "T",
        Key::KeyU => config.key == "U",
        Key::KeyV => config.key == "V",
        Key::KeyW => config.key == "W",
        Key::KeyX => config.key == "X",
        Key::KeyY => config.key == "Y",
        Key::KeyZ => config.key == "Z",
        Key::Num0 => config.key == "0",
        Key::Num1 => config.key == "1",
        Key::Num2 => config.key == "2",
        Key::Num3 => config.key == "3",
        Key::Num4 => config.key == "4",
        Key::Num5 => config.key == "5",
        Key::Num6 => config.key == "6",
        Key::Num7 => config.key == "7",
        Key::Num8 => config.key == "8",
        Key::Num9 => config.key == "9",
        _ => false,
    };

    if !key_matches {
        return false;
    }

    let needs_ctrl = config.modifiers.iter().any(|m| m == "Control");
    let needs_shift = config.modifiers.iter().any(|m| m == "Shift");
    let needs_alt = config.modifiers.iter().any(|m| m == "Alt");
    let needs_meta = config.modifiers.iter().any(|m| m == "Meta");

    modifiers.ctrl == needs_ctrl
        && modifiers.shift == needs_shift
        && modifiers.alt == needs_alt
        && modifiers.meta == needs_meta
}
