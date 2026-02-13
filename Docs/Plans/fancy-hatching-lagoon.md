# Plan de Correction : VoxAI - Application de Dictée Vocale

## État Actuel - Diagnostic

L'application a été construite avec une infrastructure complète mais **aucune intégration fonctionnelle**. Les problèmes identifiés :

### Problèmes CRITIQUES

| Problème | Statut | Impact |
|----------|--------|--------|
| **Raccourcis globaux** | ❌ Non implémenté | rdev dans Cargo.toml mais JAMAIS importé/utilisé |
| **Transcription** | ❌ Stub | Retourne "placeholder", whisper-rs non intégré |
| **Recording commands** | ❌ Manquants | Pas de start_recording/stop_recording |
| **Frontend déconnecté** | ❌ 11/12 commandes non appelées | Seul check_ollama_connection fonctionne |
| **Settings non persistés** | ❌ Mémoire seulement | tauri_plugin_store inutilisé |
| **Formulaires sans handlers** | ❌ Aucun onChange | UI décorative uniquement |

### Ce qui EXISTE et FONCTIONNE

- ✅ AudioRecorder complet (cpal, resampling)
- ✅ OllamaClient complet (reformulation, traduction)
- ✅ TextInjector complet (presse-papier + enigo)
- ✅ AppState avec RwLock
- ✅ UI React complète (mais non connectée)
- ✅ System tray basique

---

## Plan de Correction en 4 Phases

### PHASE 1 : Pipeline Minimal Fonctionnel
**Objectif** : Hotkey → Record → Transcribe → Inject
**Testable** : OUI - Ctrl+Shift+Space doit fonctionner

#### 1.1 Créer le service Hotkey (`src-tauri/src/services/hotkey.rs`)

```rust
// NOUVEAU FICHIER
use rdev::{listen, Event, EventType, Key};
use std::sync::Arc;
use parking_lot::Mutex;
use tauri::{AppHandle, Emitter};

pub struct HotkeyManager {
    shortcut_keys: Arc<Mutex<Vec<Key>>>,
    currently_pressed: Arc<Mutex<Vec<Key>>>,
    is_active: Arc<Mutex<bool>>,
}

impl HotkeyManager {
    pub fn new() -> Self;
    pub fn parse_shortcut(shortcut: &str) -> Vec<Key>;
    pub fn start_listener(self: Arc<Self>, app_handle: AppHandle);
    fn on_key_event(&self, event: Event, app_handle: &AppHandle);
}
```

**Fonctionnement** :
- Thread séparé avec `rdev::listen()`
- Émet événements Tauri : `recording-started`, `recording-stopped`
- Gère Ctrl+Shift+Space par défaut

#### 1.2 Ajouter commandes recording (`src-tauri/src/commands/mod.rs`)

```rust
// AJOUTER ces commandes
#[tauri::command]
pub async fn start_recording(state: State<'_, Arc<AppState>>) -> Result<(), String>;

#[tauri::command]
pub async fn stop_recording(state: State<'_, Arc<AppState>>) -> Result<Vec<f32>, String>;
```

#### 1.3 Implémenter transcription basique

**Option A - Placeholder intelligent** (pour tester le pipeline) :
```rust
#[tauri::command]
pub async fn transcribe_audio(audio_data: Vec<f32>) -> Result<String, String> {
    // Retourne la durée pour valider le pipeline
    Ok(format!("[Test] Audio reçu: {} samples", audio_data.len()))
}
```

**Option B - whisper-rs** (si installé) :
- Ajouter `whisper-rs = "0.11"` à Cargo.toml
- Créer `services/whisper.rs`

#### 1.4 Modifier lib.rs pour initialiser hotkey

```rust
// MODIFIER setup()
.setup(|app| {
    let app_state = Arc::new(AppState::new());
    app.manage(app_state.clone());

    // NOUVEAU: Démarrer le listener de raccourcis
    let hotkey_manager = Arc::new(HotkeyManager::new());
    hotkey_manager.clone().start_listener(app.handle().clone());
    app.manage(hotkey_manager);

    setup_tray(app)?;
    Ok(())
})
```

#### 1.5 Frontend : écouter les événements

```typescript
// AJOUTER dans App.tsx useEffect
import { listen } from "@tauri-apps/api/event";

useEffect(() => {
    const unlistenStart = listen("recording-started", () => {
        setStatus("recording");
        console.log("Recording started via hotkey");
    });

    const unlistenStop = listen("recording-stopped", () => {
        setStatus("processing");
        console.log("Recording stopped, processing...");
    });

    const unlistenComplete = listen<{ text: string }>("transcription-complete", (event) => {
        setStatus("idle");
        console.log("Transcription:", event.payload.text);
    });

    return () => {
        unlistenStart.then(fn => fn());
        unlistenStop.then(fn => fn());
        unlistenComplete.then(fn => fn());
    };
}, []);
```

---

### PHASE 2 : Persistance des Settings
**Objectif** : Les paramètres survivent au redémarrage
**Testable** : OUI - Changer un setting, redémarrer, vérifier

#### 2.1 Créer service store (`src-tauri/src/services/store.rs`)

```rust
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;
use crate::state::Settings;

pub fn load_settings(app: &AppHandle) -> Result<Settings, anyhow::Error> {
    let store = app.store("settings.json")?;
    // Charger et désérialiser
}

pub fn save_settings(app: &AppHandle, settings: &Settings) -> Result<(), anyhow::Error> {
    let store = app.store("settings.json")?;
    // Sérialiser et sauvegarder
}
```

#### 2.2 Modifier les commandes get/update_settings

```rust
#[tauri::command]
pub async fn update_settings(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    settings: Settings,
) -> Result<(), String> {
    *state.settings.write() = settings.clone();
    store::save_settings(&app, &settings).map_err(|e| e.to_string())
}
```

#### 2.3 Charger settings au démarrage (lib.rs)

```rust
// Dans setup(), après création AppState
if let Ok(saved_settings) = store::load_settings(&app.handle()) {
    *app_state.settings.write() = saved_settings;
}
```

---

### PHASE 3 : Connecter Frontend au Backend
**Objectif** : Tous les contrôles UI fonctionnent
**Testable** : OUI - Chaque formulaire doit sauvegarder

#### 3.1 Créer hook useSettings (`src/hooks/useSettings.ts`)

```typescript
// NOUVEAU FICHIER
import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface Settings {
    shortcut: string;
    microphone_id: string | null;
    whisper_model: string;
    transcription_language: string;
    reformulation_style: string;
    translation_target: string | null;
    llm_model: string;
    auto_enter: boolean;
    preserve_clipboard: boolean;
    theme: string;
    show_overlay: boolean;
    overlay_position: string;
    auto_start: boolean;
}

export function useSettings() {
    const [settings, setSettings] = useState<Settings | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        invoke<Settings>("get_settings")
            .then(setSettings)
            .catch(e => setError(e.toString()))
            .finally(() => setLoading(false));
    }, []);

    const updateSetting = useCallback(async <K extends keyof Settings>(
        key: K,
        value: Settings[K]
    ) => {
        if (!settings) return;
        const newSettings = { ...settings, [key]: value };
        setSettings(newSettings);
        try {
            await invoke("update_settings", { settings: newSettings });
        } catch (e) {
            setError(e as string);
        }
    }, [settings]);

    return { settings, loading, error, updateSetting };
}
```

#### 3.2 Créer hook useAudioDevices (`src/hooks/useAudioDevices.ts`)

```typescript
// NOUVEAU FICHIER
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface AudioDevice {
    id: string;
    name: string;
    is_default: boolean;
}

export function useAudioDevices() {
    const [devices, setDevices] = useState<AudioDevice[]>([]);

    useEffect(() => {
        invoke<AudioDevice[]>("list_audio_devices")
            .then(setDevices)
            .catch(console.error);
    }, []);

    return devices;
}
```

#### 3.3 Modifier App.tsx pour utiliser les hooks

```typescript
// MODIFIER App.tsx
function App() {
    const { settings, loading, updateSetting } = useSettings();

    if (loading) return <div>Chargement...</div>;
    if (!settings) return <div>Erreur de chargement</div>;

    return (
        // Passer settings et updateSetting à tous les composants
        <GeneralSettings
            settings={settings}
            onUpdate={updateSetting}
        />
    );
}
```

#### 3.4 Ajouter onChange à tous les contrôles

**Exemple GeneralSettings** :
```typescript
<input
    type="checkbox"
    checked={settings.auto_start}
    onChange={(e) => onUpdate("auto_start", e.target.checked)}
/>
```

**Exemple TranscriptionSettings** :
```typescript
<select
    value={settings.whisper_model}
    onChange={(e) => onUpdate("whisper_model", e.target.value)}
>
    <option value="tiny">Tiny (75 Mo)</option>
    <option value="base">Base (142 Mo)</option>
    // etc.
</select>
```

---

### PHASE 4 : Fonctionnalités Avancées
**Objectif** : Capture de raccourci, overlay, historique

#### 4.1 Composant ShortcutCapture

```typescript
// Composant pour capturer un nouveau raccourci
function ShortcutCapture({ onCapture, onCancel }) {
    const [keys, setKeys] = useState<string[]>([]);

    useEffect(() => {
        const handler = (e: KeyboardEvent) => {
            e.preventDefault();
            // Construire la combinaison de touches
        };
        window.addEventListener("keydown", handler);
        return () => window.removeEventListener("keydown", handler);
    }, []);

    return (
        <div className="modal">
            <p>Appuyez sur votre nouvelle combinaison...</p>
            <kbd>{keys.join("+") || "En attente..."}</kbd>
        </div>
    );
}
```

#### 4.2 Synchroniser historique

```typescript
// Dans App.tsx
useEffect(() => {
    invoke<TranscriptionEntry[]>("get_history")
        .then(setHistory)
        .catch(console.error);
}, []);

// Écouter les nouvelles transcriptions
listen<TranscriptionEntry>("transcription-complete", (event) => {
    setHistory(prev => [event.payload, ...prev].slice(0, 10));
});
```

---

## Fichiers à Modifier/Créer

### Nouveaux fichiers (6)

| Fichier | Description |
|---------|-------------|
| `src-tauri/src/services/hotkey.rs` | Service rdev pour raccourcis globaux |
| `src-tauri/src/services/store.rs` | Persistance avec tauri_plugin_store |
| `src/hooks/useSettings.ts` | Hook React pour settings |
| `src/hooks/useAudioDevices.ts` | Hook React pour liste micros |
| `src/hooks/useRecording.ts` | Hook React pour état recording |
| `src/components/ShortcutCapture.tsx` | Modal capture raccourci |

### Fichiers à modifier (6)

| Fichier | Changements |
|---------|-------------|
| `src-tauri/src/services/mod.rs` | Exporter hotkey, store |
| `src-tauri/src/commands/mod.rs` | Ajouter start_recording, stop_recording |
| `src-tauri/src/lib.rs` | Initialiser HotkeyManager au démarrage |
| `src-tauri/Cargo.toml` | Ajouter whisper-rs (optionnel phase 1) |
| `src/App.tsx` | Utiliser hooks, écouter événements Tauri |
| `src-tauri/src/state/mod.rs` | Ajouter AudioRecorder à AppState |

---

## Ordre d'exécution recommandé

1. **Phase 1.1** : Créer hotkey.rs avec rdev
2. **Phase 1.4** : Modifier lib.rs pour démarrer hotkey
3. **Phase 1.5** : Ajouter listeners dans App.tsx
4. **TEST** : Vérifier que Ctrl+Shift+Space affiche des logs
5. **Phase 1.2** : Ajouter start/stop_recording commands
6. **Phase 1.3** : Implémenter transcription (placeholder ou whisper-rs)
7. **TEST** : Pipeline complet fonctionne
8. **Phase 2** : Persistance settings
9. **Phase 3** : Frontend connecté
10. **Phase 4** : Features avancées

---

## Vérification finale

### Test Phase 1
```
1. Lancer npm run tauri dev
2. Appuyer Ctrl+Shift+Space
3. Console affiche "Recording started"
4. Relâcher
5. Console affiche "Recording stopped" puis texte transcrit
6. Texte apparaît au curseur (Notepad)
```

### Test Phase 2
```
1. Changer le modèle Whisper dans l'UI
2. Fermer l'app
3. Relancer
4. Vérifier que le modèle est toujours sélectionné
```

### Test Phase 3
```
1. Changer chaque paramètre dans l'UI
2. Vérifier les logs Tauri (update_settings appelé)
3. Redémarrer et vérifier persistance
```
