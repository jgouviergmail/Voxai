# Plan: 3 Features — Hotkey Fix, Overlay Controls, Select+Hotkey

## Feature 1: Fix AZERTY/QWERTY Keyboard Hotkey Recording

### Root Cause
`mapKeyToRdev()` in [GeneralTab.tsx](src/components/settings/GeneralTab.tsx) uses `event.code` which gives **physical QWERTY position** (e.g. pressing "A" on AZERTY → `code = "KeyQ"`). But rdev on Windows reports **virtual key codes** which are layout-dependent (pressing "A" on AZERTY → `Key::KeyA`). Mismatch → hotkey never triggers.

### Fix
Use `event.key` (layout-aware character) instead of `event.code` for letter keys. No AZERTY/QWERTY setting needed.

### Changes — [GeneralTab.tsx](src/components/settings/GeneralTab.tsx)
In `mapKeyToRdev()`, replace lines 15-16:
```ts
// BEFORE: const letter = code.match(/^Key([A-Z])$/); if (letter) return letter[1];
// AFTER:
if (key.length === 1 && /^[a-zA-Z]$/.test(key)) return key.toUpperCase();
```
- Keep `event.code` for Space, F1-F12, digits (position-independent)
- Dead keys (`event.key="Dead"`, length>1) and AltGr chars (`@`, `#`) filtered by `/^[a-zA-Z]$/`
- Backward compat: default Ctrl+Shift+Space unaffected; users must re-record letter-based hotkeys once

---

## Feature 2: Overlay Controls (Translation/Reformulation)

### Design: Transparent Window Approach

**CRITICAL**: Tauri 2 `setSize()` does NOT work when `resizable: false` AND `decorations: false` (bug #11975). Our overlay has both.

**Solution**: Since `transparent: true`, empty areas are invisible and click-through (Windows layered window). Set window to full expanded size; control visibility via CSS only. No resize API needed, no extra permissions.

### UI Layout
```
COLLAPSED:  ● Ready                    ▲     (only pill visible, rest transparent)

EXPANDED:   ● Ready                    ▼
            ─────────────────────────────
            Reformulation  [ON/OFF]  [Style ▼]
            Translation    [ON/OFF]  [Lang  ▼]
```

### Changes

**[tauri.conf.json](src-tauri/tauri.conf.json)** — Overlay window:
- `width: 300`, `height: 180` (full expanded size, transparent areas invisible)
- Keep `resizable: false`, `decorations: false`, `transparent: true`

**[default.json](src-tauri/capabilities/default.json)** — No new permissions needed.

**[Overlay.tsx](src/Overlay.tsx)** — Major rewrite:
- **Signals**: `expanded` (bool, default false), `config` (AppConfig | null), `languages` (LanguageInfo[])
- **onMount**:
  - `invoke<AppConfig>("get_settings")` → set config + `i18n.setLocale(config.general.ui_language)`
  - `invoke<LanguageInfo[]>("list_supported_languages")` → set languages
  - Reuse `onSettingsUpdated` from `./lib/events` → re-fetch config + update locale
  - Keep existing `onRecordingStateChanged` listener
  - All listeners cleaned up via `onCleanup(() => unlisten())`
- **Structure** (drag region ONLY on pill header, NOT expanded panel):
  ```tsx
  <div> {/* root — no drag region, transparent bg */}
    <div data-tauri-drag-region onClick={toggleExpanded}
         class="flex items-center gap-2 px-3 py-1.5 rounded-full bg-gray-900/80 ...">
      {/* pill: dot + label + ▲/▼ indicator */}
    </div>
    <Show when={expanded() && config()}>
      <div class="mt-1 rounded-lg bg-gray-900/90 p-2 ...">
        {/* Reformulation row: compact toggle + style select */}
        {/* Translation row: compact toggle + language select */}
      </div>
    </Show>
  </div>
  ```
- **Save pattern** (overlay has its own JS context, cannot use `appStore`):
  ```ts
  const saveOverlay = async (updater: (c: AppConfig) => void) => {
    const current = config();
    if (!current) return;
    const c = structuredClone(current);
    updater(c);
    try {
      await invoke("update_settings", { config: c });
      setConfig(c); // optimistic update
    } catch (e) { console.error("Overlay save failed:", e); }
  };
  ```
- **UI components**: Use native `<select>` and small toggle inline (overlay is too compact for full Toggle/Select components). Style matches pill aesthetic (dark, compact, monochrome).
- **Builtin styles list**: `["Cleaned","Professional","Casual","Concise","Simplified","Structured"]` — same as [PostProcessingTab.tsx:75](src/components/settings/PostProcessingTab.tsx#L75). Custom prompts from `config().postprocessing.custom_prompts`.
- **Style value handling**: Builtin → string, Custom → `{ Custom: id }` — same serialization as PostProcessingTab.

**Settings sync** — Already working, NO changes to main window:
- Backend emits `settings-updated` after every `update_settings` ([persistence.rs:52](src-tauri/src/config/persistence.rs#L52))
- [App.tsx:65-71](src/App.tsx#L65-L71) already listens and re-fetches config + locale

**i18n** — Add keys to [en.ts](src/lib/translations/en.ts), [fr.ts](src/lib/translations/fr.ts), [zh.ts](src/lib/translations/zh.ts):
- `overlay.reformulation`, `overlay.translation`

---

## Feature 3: Select Text + Hotkey for Instant Processing

### Flow
1. User selects text in any application
2. Presses configurable hotkey (separate from push-to-talk)
3. Backend: `is_simulating=true` → save clipboard → Ctrl+C → 150ms → read clipboard → `is_simulating=false` → run LLM pipeline → `is_simulating=true` → write result → Ctrl+V → 150ms → restore clipboard → `is_simulating=false`

### Critical: `is_simulating` Flag

rdev's global hook SEES simulated keystrokes from enigo (`SendInput`). Without protection:
- **Modifier state corruption**: simulated Ctrl release → `modifiers.ctrl=false` while user still physically holds Ctrl
- **False hotkey matches** on simulated Ctrl+C / Ctrl+V

**Solution**: Shared `is_simulating: Arc<AtomicBool>` (SeqCst ordering, consistent with `is_recording`).
- Keyboard hook: `if is_simulating.load(SeqCst) { return; }` — skip ALL event processing
- Also wraps EXISTING `inject()` Ctrl+V simulation (collateral improvement)

### Config — [schema.rs](src-tauri/src/config/schema.rs) + [index.ts](src/types/index.ts)

Rust `GeneralConfig`:
```rust
#[serde(default)]
pub text_hotkey: Option<HotkeyConfig>,  // None = disabled
```
TypeScript `GeneralConfig`:
```ts
text_hotkey: HotkeyConfig | null;
```
Backward compatible via `#[serde(default)]`.

### Backend Changes

**[injection/mod.rs](src-tauri/src/injection/mod.rs)**:
- Factory: `pub fn create_injector(is_simulating: Arc<AtomicBool>) -> Box<dyn TextInjector>`
- Trait: add methods with default error implementations (no panic on unsupported platforms):
  ```rust
  fn copy_selection(&self) -> Result<(String, Option<String>), AppError> {
      Err(AppError::Internal("Not supported on this platform".into()))
  }
  fn replace_selection(&self, text: &str, saved: Option<String>) -> Result<(), AppError> {
      Err(AppError::Internal("Not supported on this platform".into()))
  }
  ```

**[injection/windows.rs](src-tauri/src/injection/windows.rs)**:
- Add field: `is_simulating: Arc<AtomicBool>`
- `copy_selection()`: flag=true → `Clipboard::new()?.get_text().ok()` (save) → Ctrl+C → 150ms sleep → `get_text()` (read) → flag=false → compare saved vs new → `Ok((text, saved))` or `Err(AppError::Injection("No text selected"))`
- `replace_selection()`: flag=true → `set_text(result)` → Ctrl+V → 150ms → `set_text(saved)` → flag=false
- Update `inject()`: wrap Ctrl+V block with `self.is_simulating.store(true/false, SeqCst)`
- Errors: `AppError::Injection(format!(...))` — consistent with existing `inject()` pattern

**[hotkey/mod.rs](src-tauri/src/hotkey/mod.rs)**:
```rust
pub enum HotkeyEvent {
    RecordStart,
    RecordStop,
    TextProcess,  // NEW
}
```

**[keyboard_hook.rs](src-tauri/src/hotkey/keyboard_hook.rs)**:
- Signature: `pub fn start_listener(hotkey: Arc<RwLock<HotkeyConfig>>, text_hotkey: Arc<RwLock<Option<HotkeyConfig>>>, is_simulating: Arc<AtomicBool>) -> mpsc::Receiver<HotkeyEvent>`
- First line of `listen` closure: `if is_simulating.load(SeqCst) { return; }`
- After push-to-talk KeyPress check, add:
  ```rust
  if let Ok(guard) = text_hotkey.read() {
      if let Some(ref text_cfg) = *guard {
          if matches_hotkey(&key, &modifiers_pressed, text_cfg) {
              let _ = tx.send(HotkeyEvent::TextProcess);
          }
      }
  }
  ```
- Reuses existing `matches_hotkey()` function ✓

**[app_state.rs](src-tauri/src/app_state.rs)** — Add:
```rust
pub text_hotkey_config: Arc<RwLock<Option<HotkeyConfig>>>,
pub is_simulating_keys: Arc<AtomicBool>,
```

**[lib.rs](src-tauri/src/lib.rs)**:
- Setup (in `setup` closure):
  ```rust
  let is_simulating_keys = Arc::new(AtomicBool::new(false));
  let text_injector = create_injector(is_simulating_keys.clone());
  let text_hotkey_config = Arc::new(RwLock::new(config.general.text_hotkey.clone()));
  // ... pass to AppState, pass to start_listener
  ```
- Handler (in hotkey dispatch `std::thread::spawn` loop):
  ```rust
  let is_text_processing = Arc::new(AtomicBool::new(false)); // local to loop, shared with tasks
  while let Ok(event) = hotkey_rx.recv() {
      match event {
          // ... existing RecordStart/RecordStop ...
          HotkeyEvent::TextProcess => {
              let state = app.state::<AppState>();
              if state.is_recording.load(SeqCst) { continue; } // don't interrupt PTT
              if is_text_processing.load(SeqCst) { continue; } // no re-entry
              let flag = is_text_processing.clone();
              let app = app.clone();
              tauri::async_runtime::spawn(async move {
                  flag.store(true, SeqCst);
                  if let Err(e) = handle_text_process(&app).await {
                      log::error!("Text processing error: {}", e);
                      let _ = app.emit(events::EVENT_ERROR, format!("{}", e));
                  }
                  flag.store(false, SeqCst);
              });
          }
      }
  }
  ```
- New `handle_text_process` function:
  1. Check reformulation.enabled OR translation.enabled (abort if neither)
  2. Extract `pp_config`, `backend` (clone Arc out of RwLock), `language` (from `config.general.language`)
  3. Emit `Processing` state
  4. `spawn_blocking(|| injector.copy_selection()).await.map_err(...)??`
  5. `postprocessing::pipeline::run_pipeline(&text, &pp_config, backend.as_deref(), language.as_deref()).await?`
  6. `spawn_blocking(move || injector.replace_selection(&result, saved)).await.map_err(...)??`
  7. Reset to Idle state
  - Error handling: follows existing pattern from `run_pipeline` (double-unwrap `??`)

**[commands/settings.rs](src-tauri/src/commands/settings.rs)** — After hotkey update block (line 89), add:
```rust
{
    let mut thk = state.text_hotkey_config.write()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    *thk = config.general.text_hotkey.clone();
}
```

### Frontend Changes

**[GeneralTab.tsx](src/components/settings/GeneralTab.tsx)** — New section after push-to-talk:
- Section header: `i18n.t("general.text_hotkey")`
- Toggle: enable/disable (`config().general.text_hotkey !== null`)
  - Enable → `save(c => c.general.text_hotkey = { key: "R", modifiers: ["Control", "Shift"] })`
  - Disable → `save(c => c.general.text_hotkey = null)`
- When enabled: hotkey recorder (reuse `mapKeyToRdev`, `modifiersFromEvent`, `formatHotkey` — already defined in same file)
  - Second `recording2` signal + `activeHandler2`/`activeTimeout2` (separate from PTT recorder)
  - Or: refactor recorder into reusable function `createHotkeyRecorder()` that returns signals+handlers (DRY)
- Description: `i18n.t("general.text_hotkey_desc")`

**i18n** — All 3 locale files:
- `general.text_hotkey`: "Text processing hotkey" / "Raccourci traitement texte" / "文本处理快捷键"
- `general.text_hotkey_desc`: "Select text and press shortcut to reformulate/translate in place" / ...
- `general.text_hotkey_enable`: "Enable text processing hotkey" / ...

---

## Implementation Order

1. **Feature 1** (Hotkey fix) — 1 file, minimal risk
2. **Feature 3 infra** — `is_simulating` flag in `create_injector` + `inject()` + hook (benefits existing code)
3. **Feature 3 completion** — config, hook TextProcess, handler, UI
4. **Feature 2** (Overlay) — independent

## Files Modified (complete list)

| File | Feature | Changes |
|------|---------|---------|
| `src/components/settings/GeneralTab.tsx` | 1, 3 | Fix mapKeyToRdev + add text hotkey section |
| `src/Overlay.tsx` | 2 | Major rewrite: expandable, settings controls |
| `src-tauri/tauri.conf.json` | 2 | Overlay size 300x180 |
| `src-tauri/src/config/schema.rs` | 3 | Add `text_hotkey` field |
| `src/types/index.ts` | 3 | Mirror `text_hotkey` field |
| `src-tauri/src/injection/mod.rs` | 3 | `create_injector(flag)`, trait methods |
| `src-tauri/src/injection/windows.rs` | 3 | `is_simulating` field, copy/replace, fix inject |
| `src-tauri/src/hotkey/mod.rs` | 3 | `TextProcess` variant |
| `src-tauri/src/hotkey/keyboard_hook.rs` | 3 | New params, simulating guard, text hotkey check |
| `src-tauri/src/app_state.rs` | 3 | `text_hotkey_config`, `is_simulating_keys` fields |
| `src-tauri/src/lib.rs` | 3 | Init, wiring, `handle_text_process` |
| `src-tauri/src/commands/settings.rs` | 3 | Update `text_hotkey_config` Arc |
| `src/lib/translations/en.ts` | 2, 3 | i18n keys |
| `src/lib/translations/fr.ts` | 2, 3 | i18n keys |
| `src/lib/translations/zh.ts` | 2, 3 | i18n keys |

## Known Limitations

- **Non-text clipboard**: Image clipboard content can't be restored (arboard text-only)
- **Focus change during LLM**: User clicks elsewhere → Ctrl+V pastes in wrong window
- **Elevated windows**: Simulated keystrokes don't reach admin/UAC windows
- **Selection detection**: If clipboard already contained the exact selected text → false negative (rare)

## Verification

1. `cargo check` — zero errors
2. `npx tsc --noEmit` — zero errors
3. `cargo test` — all existing + new tests pass
4. Manual: AZERTY record hotkey → correct letter displayed → hotkey triggers
5. Manual: overlay collapsed → only pill visible, transparent areas click-through
6. Manual: overlay expanded → toggle reformulation/translation → main window updates
7. Manual: select text in Notepad → press text-process hotkey → text replaced
8. Manual: push-to-talk during text-processing → blocked (and vice versa)
9. Manual: existing push-to-talk → transcribe → inject → `is_simulating` doesn't break anything
