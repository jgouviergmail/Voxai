# Plan : STT Streaming + Optimisation CPU

## Vue d'ensemble

3 phases incrementales. Chaque phase est fonctionnelle independamment.

| Phase | Contenu | Effort | Nouvelles deps |
|-------|---------|--------|----------------|
| **A** | Toggle `real_time` + quick wins CPU | ~1 jour | Aucune |
| **B** | Streaming STT (silence detection + whisper-rs) | ~2-3 jours | Aucune (tokio sync) |
| **C** | sherpa-rs + Moonshine (streaming natif) | ~3-4 jours | `sherpa-rs` |

**Principe fonctionnel** : quand `real_time=true`, le texte est transcrit et injecte segment par segment au curseur pendant la dictee. Toutes les fonctions post-traitement (majuscules, espacement, reformulation, traduction, substitutions) restent independamment configurables. Un avertissement de latence est affiche si le LLM est actif en streaming (1-3s Ollama/GPU, 5-30s CPU local par segment).

---

## Phase A : Toggle temps reel + Quick wins CPU

### A1. Config : champ `real_time`

**`src-tauri/src/config/schema.rs`** — `GeneralConfig` :
```rust
#[serde(default)]
pub real_time: bool,
```
Default impl : `real_time: false`

**`src/types/index.ts`** — `GeneralConfig` :
```typescript
real_time: boolean;
```

### A2. `real_time` = streaming vs batch (pas de desactivation LLM)

`real_time` controle UNIQUEMENT le mode de capture/transcription :
- `false` (defaut) : mode batch — enregistrer tout, transcrire en une fois, pipeline complet, injecter
- `true` : mode streaming — transcrire segment par segment, pipeline complet par segment, injecter au curseur progressivement

**Aucune modification du pipeline LLM en Phase A.** Le pipeline reste identique (capitalize → spacing → reformulate → translate → substitute). La distinction batch/streaming n'intervient qu'en Phase B.

Note : `handle_text_process` n'est PAS affecte par `real_time` (c'est un flux independant).

### A3. Frontend : toggle GeneralTab

**`src/components/settings/GeneralTab.tsx`** — nouvelle section "Mode" avec un Toggle `real_time` :
- Placement : juste apres la section "Behavior", avant "Hotkey"
- Composant : `<Toggle>` existant
- Pattern : `save((c) => (c.general.real_time = v))`

### A4. Frontend : toggle Overlay + avertissement latence

**`src/Overlay.tsx`** :
- Ajouter une ligne "Temps reel" avec toggle (meme pattern que translation/reformulation)
- Quand `real_time=true` ET (reformulation OU traduction activee) : afficher un badge d'avertissement "Latence LLM" sous les toggles. Les toggles restent ACTIFS (l'utilisateur choisit).
- Quand `real_time=false` : comportement actuel inchange (aucun avertissement)

### A5. i18n (3 langues)

| Cle | EN | FR | ZH |
|-----|----|----|-----|
| `general.real_time` | Real-time mode | Mode temps reel | 实时模式 |
| `general.real_time_desc` | Transcribe and inject text progressively during recording | Transcrire et injecter le texte progressivement pendant l'enregistrement | 录音期间逐步转录和注入文本 |
| `overlay.real_time` | Real-time | Temps reel | 实时 |
| `overlay.llm_latency_warn` | LLM adds latency per segment | Le LLM ajoute de la latence par segment | LLM 每段增加延迟 |

### A6. Modeles Distil-Whisper dans le catalogue

**`src-tauri/src/models/registry.rs`** — ajouter 2 entrees `MODEL_CATALOG` :

```rust
ModelDefinition {
    id: "distil-whisper-large-v3",
    name: "Distil-Whisper Large v3",
    engine: "whisper",
    repo: "distil-whisper/distil-large-v3-ggml",  // a verifier sur HF
    filename: "ggml-distil-large-v3.bin",
    size_mb: 756,
    description: "6x faster than Large v3. Multilingual.",
    chat_template: "",
},
```
+ i18n descriptions correspondantes

### A7. Reglage threads CPU (slider dynamique avec bornes machine)

Controle du nombre de threads CPU pour Whisper, avec detection auto des capacites de la machine.

**Detection CPU au runtime** : `std::thread::available_parallelism()` (std, zero dependance).
- Min = 1
- Max = nb coeurs logiques detectes (ex: 8 sur un i7 portable, 16 sur un Ryzen desktop)
- Defaut = `None` → Whisper choisit `min(4, nb_coeurs)` automatiquement

**Compatibilite CUDA** : `set_n_threads()` controle les threads CPU de decodage. Avec GPU actif,
le calcul lourd (encoder/attention) est sur le GPU — les threads CPU servent au pre/post-traitement.
Le reglage reste accessible, avec une note "(impact reduit avec GPU)". **Zero conflit**.

**`src-tauri/src/config/schema.rs`** — `GeneralConfig` :
```rust
/// Nombre de threads CPU pour le STT. None = auto (whisper choisit).
#[serde(default)]
pub stt_threads: Option<u32>,
```
Default impl : `stt_threads: None`

**`src/types/index.ts`** — `GeneralConfig` :
```typescript
stt_threads: number | null;
```

**Backend** — nouveau command dans `commands/gpu.rs` (meme fichier que `detect_nvidia`) :
```rust
#[tauri::command]
pub fn detect_cpu_count() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4)
}
```
Enregistre dans `invoke_handler` dans `lib.rs` : `commands::gpu::detect_cpu_count,`

**`src/lib/commands.ts`** — ajouter :
```typescript
export const detectCpuCount = () => invoke<number>("detect_cpu_count");
```

**`src-tauri/src/stt/whisper.rs`** — modifier la signature de `transcribe()` pour accepter le thread limit :

Approche : **passer `stt_threads` en parametre de `transcribe()`** au lieu de le stocker dans
`WhisperEngine`. Cela evite un probleme de synchronisation : si on stocke un champ `thread_limit`
dans la struct, il faut un mecanisme pour le mettre a jour quand l'utilisateur change le slider
(le setter `set_thread_limit()` ne serait jamais appele automatiquement).

En le passant en parametre, la valeur est lue depuis la config au moment de l'appel, toujours a jour.

**Modification du trait `SttEngine`** (src-tauri/src/stt/mod.rs) :
```rust
pub trait SttEngine: Send + Sync {
    // ... existant inchange ...
    fn transcribe(&self, samples: &[f32], language: Option<&str>) -> Result<TranscriptionResult, AppError>;
    // Note: stt_threads n'est PAS dans le trait (specifique a Whisper).
}
```

**`WhisperEngine`** : lire `stt_threads` depuis la config AVANT l'appel a transcribe,
dans `run_pipeline()` (batch) et `run_streaming()` (streaming). Le passer dans FullParams :
```rust
// Dans le spawn_blocking qui appelle transcribe :
let stt_threads = {
    let cfg = config.read()...;
    cfg.general.stt_threads
};

// Dans WhisperEngine::transcribe(), apres creation de FullParams :
// (passe via un champ mutable ou un wrapper, ou lu depuis un Arc<RwLock<Option<u32>>>)
```

**Approche retenue** : stocker `thread_limit: Arc<RwLock<Option<u32>>>` dans `WhisperEngine`,
partage avec `AppState`, mis a jour dans `update_settings` (meme pattern que `hotkey_config`).

**`src-tauri/src/stt/whisper.rs`** — modifier struct et constructeur :
```rust
pub struct WhisperEngine {
    context: Option<WhisperContext>,
    use_gpu: bool,
    thread_limit: Arc<RwLock<Option<u32>>>,  // partage avec AppState, mis a jour live
}

impl WhisperEngine {
    pub fn new(use_gpu: bool, thread_limit: Arc<RwLock<Option<u32>>>) -> Self {
        Self { context: None, use_gpu, thread_limit }
    }
}

// Dans transcribe(), apres creation de FullParams :
let thread_limit = self.thread_limit.read().ok().and_then(|g| *g);
if let Some(n) = thread_limit {
    params.set_n_threads(n as i32);
}
```

**`src-tauri/src/app_state.rs`** — ajouter champ :
```rust
pub struct AppState {
    // ... champs existants ...
    /// Shared STT thread limit — updated live when user changes slider.
    pub stt_thread_limit: Arc<RwLock<Option<u32>>>,
}
```

**`src-tauri/src/lib.rs`** — dans `do_setup()` (apres chargement config, avant creation stt_engine) :
```rust
// Shared STT thread limit — updated live (same pattern as hotkey_config)
let stt_thread_limit = Arc::new(RwLock::new(config.general.stt_threads));

let stt_engine: Box<dyn stt::SttEngine> =
    Box::new(WhisperEngine::new(
        config.general.gpu_acceleration,
        Arc::clone(&stt_thread_limit),
    ));

// ... dans AppState { ... } :
stt_thread_limit: Arc::clone(&stt_thread_limit),
```

**`src-tauri/src/commands/settings.rs`** — dans `update_settings()`, apres les hotkey updates :
```rust
// Update shared STT thread limit (live-read by WhisperEngine)
{
    let mut tl = state
        .stt_thread_limit
        .write()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    *tl = config.general.stt_threads;
}
```

**Frontend** — `GeneralTab.tsx` : slider dans la section GPU/Performance :
- Appel `detectCpuCount()` au mount pour connaitre le max
- Slider de 1 a max, + position "Auto" (= None, tout a gauche)
- Quand GPU actif : note sous le slider "(impact reduit avec GPU)"
- Affichage : "Auto" ou "N / max coeurs"

**i18n** :
| Cle | EN | FR | ZH |
|-----|----|----|-----|
| `general.stt_threads` | CPU threads (STT) | Threads CPU (STT) | CPU 线程数 (STT) |
| `general.stt_threads_desc` | Number of CPU threads for voice recognition (Auto = let engine decide) | Nombre de threads CPU pour la reconnaissance vocale (Auto = laisser le moteur decider) | 语音识别的 CPU 线程数（自动 = 由引擎决定） |
| `general.stt_threads_gpu_note` | Reduced impact when GPU is active | Impact reduit quand le GPU est actif | GPU 激活时影响较小 |
| `general.stt_threads_auto` | Auto | Auto | 自动 |

### A8. Optimisations Whisper supplementaires (FullParams)

Plusieurs parametres `FullParams` non exploites. Impact cumule estime : ~10-15% gain CPU.

**`src-tauri/src/stt/whisper.rs`** — dans `transcribe()` :
```rust
// --- Deja present ---
// SamplingStrategy::Greedy { best_of: 1 }     ✓ (~6x vs beam=5)
// set_print_progress(false)                     ✓
// set_print_realtime(false)                     ✓
// set_print_timestamps(false)                   ✓

// --- Nouvelles optimisations ---
params.set_suppress_blank(true);     // Supprime tokens vides (silence/bruit) → moins de decodage
params.set_suppress_nst(true);       // Supprime tokens non-parole ([music], [applause]...) → sortie plus propre
params.set_no_context(true);         // Pas de contexte precedent (chaque transcription independante → -5% CPU)
params.set_temperature(0.0);         // Decodage deterministe (deja defaut mais expliciter)
```

**NON active** (volontairement) :
- `set_no_timestamps(true)` — on utilise les timestamps pour les segments
- `set_single_segment(true)` — utile SEULEMENT en streaming Phase B, pas en batch
- `enable_vad(true)` — necessite modele VAD externe, reserve Phase B ou C
- `set_translate(true)` — la traduction est geree par le pipeline LLM, pas par Whisper

**Compatibilite CUDA** : ces parametres controlent le decodeur (logique de tokens), pas le calcul matriciel (GPU). Aucun conflit avec la feature `cuda`. Les deux builds (CPU et NVIDIA) beneficient de ces optimisations.

---

## Phase B : STT Streaming (silence detection + whisper-rs)

### Architecture streaming

```
AU DEMARRAGE (handle_record_start, real_time=true) :
  1. Sauvegarder le presse-papiers (arboard)
  2. Demarrer capture streaming (cpal → mpsc channel)
  3. Lancer la boucle async run_streaming()

PENDANT L'ENREGISTREMENT :

cpal callback
    |
    v
tokio::sync::mpsc::channel<Vec<f32>>
    |
    v
Streaming processing loop (async task)
    |
    +---> Accumulation buffer + detection silence (RMS)
    |
    +---> Quand silence detecte apres parole :
    |       resample_to_16k_mono()
    |       → whisper transcribe()
    |       → capitalize (si active)
    |       → spacing (si active)
    |       → reformulation LLM (si activee — ajout latence)
    |       → traduction LLM (si activee — ajout latence)
    |       → substitutions (si configurees)
    |       → INJECTER AU CURSEUR (clipboard paste, sans Enter)
    |       → emit "transcription-partial" { text: texte_cumule }
    |
    v
Boucle continue jusqu'a fermeture du channel

A LA FIN (handle_record_stop) :
  1. stop capture → drop stream → drop tx → channel ferme → loop termine
  2. join streaming task → texte accumule (deja injecte segment par segment)
  3. Restaurer le presse-papiers original
  4. Appuyer sur Enter si auto_enter=true
  5. Historique (raw_text = concat segments bruts, final_text = concat segments traites)
```

### Mecanisme d'injection streaming

Chaque segment est injecte au curseur via clipboard+paste, comme le mode batch :
1. **Au demarrage** : `arboard::Clipboard::new()` → sauvegarder le contenu courant
2. **Par segment** : `clipboard.set_text(segment_text)` → `Ctrl+V` (enigo) → PAS de Enter
3. **A la fin** : restaurer le presse-papiers original + Enter si `auto_enter=true`

Le texte apparait progressivement a la position du curseur dans n'importe quelle application.
L'overlay affiche aussi le texte cumule (feedback visuel).

### B1. Dependance : tokio sync

**`src-tauri/Cargo.toml`** — ajouter feature `sync` a tokio :
```toml
tokio = { version = "1", features = ["time", "sync"] }
```
Aucune nouvelle crate. `tokio::sync::mpsc` est deja dans tokio.

**IMPORTANT** : utiliser `unbounded_channel()` (pas `channel(N)`). Raison : pendant le traitement
LLM d'un segment (5-30s CPU), la boucle est en `.await` sur le LLM et ne fait pas `rx.recv()`.
Un bounded channel se remplirait et `try_send()` dropperait des chunks audio (perte de parole).
Audio ≈ 96KB/s mono f32 → 30s de stall LLM = ~2.9MB accumules. Negligeable.

### B2. AudioCapture : mode streaming

**`src-tauri/src/audio/capture.rs`** :

Extraire la resolution du device dans un helper DRY :
```rust
struct ResolvedDevice {
    device: cpal::Device,
    config: StreamConfig,
    sample_format: SampleFormat,
}

fn resolve_input_device(device_name: Option<&str>) -> Result<ResolvedDevice, AppError> {
    // Logique extraite de start() (lignes 26-40)
}
```

Nouvelle methode :
```rust
pub fn start_streaming(
    &mut self,
    device_name: Option<&str>,
) -> Result<tokio::sync::mpsc::UnboundedReceiver<Vec<f32>>, AppError> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Vec<f32>>();
    let resolved = resolve_input_device(device_name)?;
    // Memes builds de stream que start(), MAIS le callback fait :
    //   1. buffer.extend_from_slice(data)  (backup)
    //   2. let _ = tx.send(data.to_vec())  (streaming — unbounded, jamais de perte)
    // Stocker sample_rate et channels comme start()
    Ok(rx)
}
```

Ajouter getters :
```rust
pub fn sample_rate(&self) -> u32 { self.sample_rate }
pub fn channels(&self) -> u16 { self.channels }
```

### B3. Detection de silence : `audio/silence.rs`

Nouveau fichier. Machine a etats simple basee sur RMS (zero dependance).

**Design cle** : le `SilenceDetector` possede un buffer interne et retourne directement
le `Vec<f32>` du segment de parole complete. Pas d'indices externes, pas de `raw_buffer`
dans le caller. Cela evite toute erreur de gestion d'indices apres drain.

```rust
const SPEECH_THRESHOLD: f32 = 0.015;      // RMS seuil parole
const SILENCE_DURATION_MS: u32 = 500;      // Duree silence pour couper
const MIN_SPEECH_DURATION_MS: u32 = 300;   // Duree min pour transcrire

enum SilenceState {
    Idle,
    InSpeech,
    TrailingSilence { silence_samples: usize },
}

pub struct SilenceDetector {
    state: SilenceState,
    sample_rate: u32,
    speech_buffer: Vec<f32>,   // accumule l'audio pendant la parole
}

impl SilenceDetector {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            state: SilenceState::Idle,
            sample_rate,
            speech_buffer: Vec::new(),
        }
    }

    /// Traite un chunk audio brut. Retourne Some(Vec<f32>) si un segment
    /// de parole est complet (silence detecte apres parole suffisante).
    pub fn process(&mut self, chunk: &[f32]) -> Option<Vec<f32>> {
        let rms = compute_rms(chunk);
        let silence_threshold_samples =
            (SILENCE_DURATION_MS as usize * self.sample_rate as usize) / 1000;
        let min_speech_samples =
            (MIN_SPEECH_DURATION_MS as usize * self.sample_rate as usize) / 1000;

        match &mut self.state {
            SilenceState::Idle => {
                if rms > SPEECH_THRESHOLD {
                    self.speech_buffer.extend_from_slice(chunk);
                    self.state = SilenceState::InSpeech;
                }
                None
            }
            SilenceState::InSpeech => {
                self.speech_buffer.extend_from_slice(chunk);
                if rms < SPEECH_THRESHOLD {
                    self.state = SilenceState::TrailingSilence { silence_samples: chunk.len() };
                }
                None
            }
            SilenceState::TrailingSilence { silence_samples } => {
                self.speech_buffer.extend_from_slice(chunk);
                if rms > SPEECH_THRESHOLD {
                    // Retour a la parole
                    self.state = SilenceState::InSpeech;
                    None
                } else {
                    *silence_samples += chunk.len();
                    if *silence_samples >= silence_threshold_samples {
                        // Silence confirme — emettre le segment
                        let speech_end = self.speech_buffer.len() - *silence_samples;
                        if speech_end >= min_speech_samples {
                            let segment = self.speech_buffer[..speech_end].to_vec();
                            self.speech_buffer.clear();
                            self.state = SilenceState::Idle;
                            Some(segment)
                        } else {
                            // Parole trop courte, ignorer
                            self.speech_buffer.clear();
                            self.state = SilenceState::Idle;
                            None
                        }
                    } else {
                        None
                    }
                }
            }
        }
    }

    /// Retourne le segment final si de la parole est en cours sans silence terminal.
    /// Appele a la fin de l'enregistrement.
    pub fn flush(&mut self) -> Option<Vec<f32>> {
        let min_speech_samples =
            (MIN_SPEECH_DURATION_MS as usize * self.sample_rate as usize) / 1000;
        if !self.speech_buffer.is_empty()
            && matches!(self.state, SilenceState::InSpeech | SilenceState::TrailingSilence { .. })
        {
            let segment = std::mem::take(&mut self.speech_buffer);
            self.state = SilenceState::Idle;
            if segment.len() >= min_speech_samples {
                Some(segment)
            } else {
                None
            }
        } else {
            None
        }
    }
}

fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() { return 0.0; }
    (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}
```

### B4. Module streaming : `src-tauri/src/streaming.rs`

Nouveau fichier. Boucle de traitement streaming qui **reutilise `pipeline::run_pipeline()`** existant
(DRY : meme code que le batch, incluant prompt building, `strip_llm_artifacts()`, `is_available()`, timeout).

```rust
use std::sync::{Arc, Mutex, RwLock};

use tauri::Emitter;

use crate::audio::{resampler, silence::SilenceDetector};
use crate::config::schema::AppConfig;
use crate::postprocessing;
use crate::stt::SttEngine;
use crate::llm::LlmBackend;
use crate::injection::TextInjector;
use crate::error::AppError;
use crate::events;

pub struct StreamingResult {
    pub raw_segments: Vec<String>,       // segments bruts (pour historique)
    pub processed_segments: Vec<String>, // segments traites (pour historique)
}

pub async fn run_streaming(
    mut rx: tokio::sync::mpsc::UnboundedReceiver<Vec<f32>>,
    stt_engine: Arc<Mutex<Box<dyn SttEngine>>>,
    llm_backend: Arc<RwLock<Option<Arc<dyn LlmBackend>>>>,
    text_injector: Arc<dyn TextInjector>,
    config: Arc<RwLock<AppConfig>>,
    app: tauri::AppHandle,
    sample_rate: u32,
    channels: u16,
) -> Result<StreamingResult, AppError> {
    let mut detector = SilenceDetector::new(sample_rate);
    let mut raw_segments: Vec<String> = Vec::new();
    let mut processed_segments: Vec<String> = Vec::new();
    let mut accumulated_display = String::new();
    let mut is_first_segment = true;

    // Lire la config une seule fois au demarrage (meme pattern que batch lib.rs:262-270)
    let (stt_language, pp_cfg) = {
        let cfg = config.read()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let lang = match &cfg.general.language {
            Some(lang) if !lang.is_empty() => Some(lang.clone()),
            _ => None,
        };
        (lang, cfg.postprocessing.clone())
    };

    while let Some(chunk) = rx.recv().await {
        // SilenceDetector gere son propre buffer interne.
        // Retourne Some(Vec<f32>) quand un segment de parole est complet.
        if let Some(speech_segment) = detector.process(&chunk) {
            process_segment(
                speech_segment, channels, sample_rate,
                &stt_engine, &llm_backend, &text_injector,
                &stt_language, &pp_cfg, &app,
                &mut raw_segments, &mut processed_segments,
                &mut accumulated_display, &mut is_first_segment,
            ).await?;
        }
    }

    // Segment final (si parole en cours sans silence terminal)
    if let Some(speech_segment) = detector.flush() {
        process_segment(
            speech_segment, channels, sample_rate,
            &stt_engine, &llm_backend, &text_injector,
            &stt_language, &pp_cfg, &app,
            &mut raw_segments, &mut processed_segments,
            &mut accumulated_display, &mut is_first_segment,
        ).await?;
    }

    Ok(StreamingResult { raw_segments, processed_segments })
}

/// Traite un segment de parole : resample → transcribe → pipeline → inject → emit.
/// Extrait en fonction pour reutilisation entre la boucle et le flush final (DRY).
async fn process_segment(
    speech_segment: Vec<f32>,
    channels: u16,
    sample_rate: u32,
    stt_engine: &Arc<Mutex<Box<dyn SttEngine>>>,
    llm_backend: &Arc<RwLock<Option<Arc<dyn LlmBackend>>>>,
    text_injector: &Arc<dyn TextInjector>,
    stt_language: &Option<String>,
    pp_cfg: &crate::config::schema::PostProcessingConfig,
    app: &tauri::AppHandle,
    raw_segments: &mut Vec<String>,
    processed_segments: &mut Vec<String>,
    accumulated_display: &mut String,
    is_first_segment: &mut bool,
) -> Result<(), AppError> {
    // 1. Resample (CPU-bound, spawn_blocking)
    let samples_16k = tauri::async_runtime::spawn_blocking({
        let ch = channels; let sr = sample_rate;
        move || resampler::resample_to_16k_mono(&speech_segment, ch, sr)
    }).await
    .map_err(|e| AppError::Internal(format!("Task join error: {}", e)))??;

    // 2. Transcrire (CPU-bound, spawn_blocking)
    let stt = Arc::clone(stt_engine);
    let lang = stt_language.clone();
    let transcription = tauri::async_runtime::spawn_blocking(move || {
        let engine = stt.lock()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        engine.transcribe(&samples_16k, lang.as_deref())
    }).await
    .map_err(|e| AppError::Internal(format!("Task join error: {}", e)))??;

    if transcription.text.is_empty() {
        return Ok(());
    }

    raw_segments.push(transcription.text.clone());

    // 3. Pipeline complet — REUTILISE pipeline::run_pipeline() existant
    //    Gere : capitalize, spacing, reformulation (prompt_templates +
    //    strip_llm_artifacts + is_available + timeout), traduction, substitutions
    let backend: Option<Arc<dyn LlmBackend>> = {
        let guard = llm_backend.read()
            .map_err(|e| AppError::Internal(e.to_string()))?;
        guard.clone()
    };
    let source_lang = transcription.language.as_deref()
        .or(stt_language.as_deref());

    let segment_text = match postprocessing::pipeline::run_pipeline(
        &transcription.text,
        pp_cfg,
        backend.as_deref(),
        source_lang,
    ).await {
        Ok(text) => text,
        Err(e) => {
            log::warn!("Streaming pipeline failed for segment: {e}");
            transcription.text.clone() // fallback : texte brut
        }
    };

    // 4. Ajouter espace avant segment (sauf le premier)
    let inject_text = if *is_first_segment {
        *is_first_segment = false;
        segment_text.clone()
    } else {
        format!(" {}", segment_text)
    };

    processed_segments.push(segment_text.clone());

    // 5. INJECTER AU CURSEUR (spawn_blocking — clipboard+paste est bloquant)
    let inj = Arc::clone(text_injector);
    let text_for_inject = inject_text;
    tauri::async_runtime::spawn_blocking(move || {
        inj.inject_no_enter(&text_for_inject)
    }).await
    .map_err(|e| AppError::Internal(format!("Task join error: {}", e)))??;

    // 6. Emettre le texte cumule pour l'overlay
    if !accumulated_display.is_empty() { accumulated_display.push(' '); }
    accumulated_display.push_str(&segment_text);
    let _ = app.emit(events::EVENT_TRANSCRIPTION_PARTIAL, &accumulated_display);

    Ok(())
}
```

**Changements cles par rapport a la version precedente (bugs corriges)** :

1. **Reutilise `pipeline::run_pipeline()`** au lieu de re-implementer le LLM inline.
   L'ancienne version appelait `llm.reformulate()` et `llm.translate()` — **ces methodes n'existent pas**
   sur le trait `LlmBackend` (seul `generate(prompt, system)` existe). `run_pipeline()` gere tout
   correctement : `build_reformulation_prompt()`, `llm_with_timeout()`, `strip_llm_artifacts()`,
   `is_available()` check. **DRY et garanti identique au batch.**

2. **`spawn_blocking()` pour l'injection.** L'injection utilise `thread::sleep()`, `Clipboard::new()`,
   `Enigo::new()` — toutes des operations bloquantes. L'ancienne version les appelait depuis le
   contexte async, ce qui bloquerait le tokio runtime.

3. **Utilise `TextInjector` trait** (via `text_injector: Arc<dyn TextInjector>`) au lieu d'une
   fonction standalone. Le `WindowsInjector` a un `is_simulating: Arc<AtomicBool>` qui indique
   au hook rdev d'ignorer les keypresses simulees. Sans ce flag, le push-to-talk pourrait se
   re-declencher pendant le Ctrl+V.

4. **Espace entre segments** : `format!(" {}", segment_text)` a partir du 2eme segment.
   Sans ca, "Bonjour" + "le monde" donnerait "Bonjourle monde" au curseur.

5. **`unbounded_channel()`** : pendant le traitement LLM (5-30s), `rx.recv()` n'est pas appele.
   Un bounded channel perdrait des chunks audio. Unbounded accumule sans perte (~2.9MB max).

**Nouvelle methode `inject_no_enter()` dans le trait `TextInjector`** :

**`src-tauri/src/injection/mod.rs`** :
```rust
pub trait TextInjector: Send + Sync {
    fn inject(&self, text: &str, options: &InjectionOptions) -> Result<(), AppError>;
    fn copy_selection(&self) -> Result<(String, Option<String>), AppError> { ... }
    fn replace_selection(&self, text: &str, saved: Option<String>) -> Result<(), AppError> { ... }

    /// Inject text at cursor via clipboard+paste, WITHOUT Enter and WITHOUT clipboard restore.
    /// Used by streaming mode — clipboard is saved/restored at session level, not per-segment.
    fn inject_no_enter(&self, text: &str) -> Result<(), AppError> {
        self.inject(text, &InjectionOptions { auto_enter: false, clipboard_restore: false })
    }
}
```

**`src-tauri/src/injection/windows.rs`** : pas de changement necessaire — la methode default
appelle `inject()` avec `auto_enter: false` et `clipboard_restore: false`, ce qui fait exactement
clipboard + paste + pas de Enter + pas de restauration. Le `is_simulating` flag est correctement
utilise via `simulate_combo()` dans `inject()`.

### B5. Evenements

**`src-tauri/src/events.rs`** :
```rust
pub const EVENT_TRANSCRIPTION_PARTIAL: &str = "transcription-partial";
```

### B6. AppState : champs streaming

**`src-tauri/src/app_state.rs`** :
```rust
pub struct AppState {
    // ... champs existants ...
    /// Handle de la tache de streaming (join a l'arret)
    pub streaming_handle: Arc<Mutex<Option<
        tauri::async_runtime::JoinHandle<Result<StreamingResult, crate::error::AppError>>
    >>>,
    /// Presse-papiers sauvegarde au debut du streaming, restaure a la fin
    pub saved_clipboard: Arc<Mutex<Option<String>>>,
}
```
Initialises a `Arc::new(Mutex::new(None))` dans `do_setup()`.

### B7. lib.rs : bifurcation batch/streaming

**`handle_record_start`** :
```rust
let (real_time, clipboard_restore) = {
    let cfg = state.config.read()...;
    (cfg.general.real_time, cfg.general.clipboard_restore)
};

if real_time {
    // 1. Sauvegarder le presse-papiers AVANT la capture (si clipboard_restore actif)
    let saved_clipboard = if clipboard_restore {
        let mut clipboard = arboard::Clipboard::new().ok();
        clipboard.as_mut().and_then(|cb| cb.get_text().ok())
    } else {
        None
    };

    // 2. Demarrer la capture streaming
    let (rx, sr, ch) = {
        let mut capture = state.audio_capture.lock()...;
        let rx = capture.start_streaming(device_name.as_deref())?;
        (rx, capture.sample_rate(), capture.channels())
    };

    let stt = Arc::clone(&state.stt_engine);
    let llm = Arc::clone(&state.llm_backend);
    let injector = Arc::clone(&state.text_injector);  // pour inject_no_enter + is_simulating
    let cfg = Arc::clone(&state.config);
    let app = app.clone();

    let handle = tauri::async_runtime::spawn(async move {
        streaming::run_streaming(rx, stt, llm, injector, cfg, app, sr, ch).await
    });

    if let Ok(mut h) = state.streaming_handle.lock() {
        *h = Some(handle);
    }
    if let Ok(mut cb) = state.saved_clipboard.lock() {
        *cb = saved_clipboard;
    }
} else {
    // Code batch existant (inchange)
    let mut capture = state.audio_capture.lock()...;
    capture.start(device_name.as_deref())?;
}
```

**`handle_record_stop`** :
```rust
let real_time = {
    let cfg = state.config.read()...;
    cfg.general.real_time
};

if real_time {
    // 1. Stopper la capture (ferme le channel → streaming loop termine)
    {
        let mut capture = state.audio_capture.lock()...;
        let _ = capture.stop();
    }

    // 2. Joindre la tache streaming
    let handle = {
        let mut h = state.streaming_handle.lock()...;
        h.take()
    };
    let result = if let Some(h) = handle {
        h.await.map_err(|e| AppError::Internal(e.to_string()))??
    } else {
        StreamingResult { raw_segments: vec![], processed_segments: vec![] }
    };

    // 3. Restaurer le presse-papiers (si sauvegarde)
    let saved = {
        let mut cb = state.saved_clipboard.lock()...;
        cb.take()
    };
    if let Some(content) = saved {
        // spawn_blocking car arboard est bloquant
        tauri::async_runtime::spawn_blocking(move || {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(&content);
            }
        }).await.ok();
    }

    // 4. Appuyer sur Enter si auto_enter (spawn_blocking car enigo est bloquant)
    let auto_enter = {
        let cfg = state.config.read()...;
        cfg.general.auto_enter
    };
    if auto_enter && !result.processed_segments.is_empty() {
        let injector = Arc::clone(&state.text_injector);
        tauri::async_runtime::spawn_blocking(move || {
            // Utilise inject() avec texte vide ou une methode dediee
            // Alternative : enigo key(Return, Click) avec is_simulating guard
            let _ = injector.inject("", &InjectionOptions { auto_enter: true, clipboard_restore: false });
        }).await.ok();
    }

    // 5. Historique
    if !result.processed_segments.is_empty() {
        let raw_text = result.raw_segments.join(" ");
        let final_text = result.processed_segments.join(" ");
        // history.add(...) — meme pattern que batch
    }

    reset_state(app, &state.is_recording, &state.recording);
} else {
    // Code batch existant (inchange)
}
```

**Note** : `inject("", auto_enter: true)` injectera un texte vide (pas de paste) puis Enter.
Si `inject()` ne gere pas le texte vide, ajouter une methode `press_enter()` au trait `TextInjector`.

**AppState** : ajouter `saved_clipboard: Arc<Mutex<Option<String>>>` pour le presse-papiers sauvegarde.

### B8. Frontend : feedback visuel dans l'Overlay

Le texte est deja injecte au curseur par le backend (B4). L'overlay sert de feedback visuel secondaire.

**`src/Overlay.tsx`** :
```tsx
const [partialText, setPartialText] = createSignal("");

// Dans onMount, ajouter listener :
unlistens.push(
    await listen<string>("transcription-partial", (e) => {
        setPartialText(e.payload);
    })
);

// Reset quand l'etat revient a Idle :
if (e.payload.kind === "Idle") setPartialText("");
```

Affichage conditionnel sous la pill quand real_time + en cours :
```tsx
<Show when={config()?.general.real_time && state().kind !== "Idle" && partialText()}>
    <div class="mt-1 rounded-lg bg-gray-900/90 backdrop-blur-sm border border-gray-700/50
                p-2 shadow-lg text-xs text-gray-200 max-h-24 overflow-y-auto
                pointer-events-none">
        {partialText()}
    </div>
</Show>
```
`pointer-events-none` pour ne pas voler le focus a l'application cible.

### B9. Frontend : evenement `transcription-partial`

**`src/lib/events.ts`** — ajouter :
```typescript
export const onTranscriptionPartial = (cb: (text: string) => void) =>
    listen<string>("transcription-partial", (e) => cb(e.payload));
```

---

## Phase C : sherpa-rs + Moonshine (streaming natif)

### C1. Dependance

**`src-tauri/Cargo.toml`** :
```toml
sherpa-rs = { version = "0.6", optional = true }
```

Feature flag : `sherpa = ["dep:sherpa-rs"]`
Permet de compiler sans sherpa-rs (build CPU leger inchange).

### C2. SherpaEngine : `src-tauri/src/stt/sherpa.rs`

Nouveau fichier. Implemente `SttEngine` :
```rust
pub struct SherpaEngine {
    offline: Option<sherpa_rs::recognizer::OfflineRecognizer>,
    // Pour streaming natif :
    online_config: Option<sherpa_rs::recognizer::OnlineRecognizerConfig>,
}

impl SttEngine for SherpaEngine {
    fn id(&self) -> &str { "sherpa" }
    fn name(&self) -> &str { "Sherpa ONNX" }
    fn load_model(&mut self, path: &Path) -> Result<(), AppError> { ... }
    fn unload_model(&mut self) { ... }
    fn is_loaded(&self) -> bool { ... }
    fn transcribe(&self, samples: &[f32], language: Option<&str>) -> Result<TranscriptionResult, AppError> { ... }
}
```

### C3. Extension du trait SttEngine (opt-in streaming)

**`src-tauri/src/stt/mod.rs`** :
```rust
pub trait SttEngine: Send + Sync {
    // ... methodes existantes ...

    /// Indique si le moteur supporte le streaming natif
    fn supports_streaming(&self) -> bool { false }

    /// Cree une session de streaming
    fn create_stream(&self) -> Result<Box<dyn SttStream>, AppError> {
        Err(AppError::Stt("Streaming not supported".into()))
    }
}

pub trait SttStream: Send {
    fn accept_waveform(&mut self, samples: &[f32]);
    fn get_result(&self) -> Option<String>;
    fn is_endpoint(&self) -> bool;
    fn reset(&mut self);
}
```

`WhisperEngine::supports_streaming()` retourne `false` (utilise Phase B fallback).
`SherpaEngine::supports_streaming()` retourne `true`.

### C4. Modeles sherpa dans le catalogue

**Priorite : modeles multilingues.** L'application supporte 57 langues — on ne peut pas se limiter a l'anglais.

**Strategie modeles sherpa-rs** :
1. **Whisper ONNX (multilingue, prioritaire)** : sherpa-rs supporte Whisper converti en ONNX. Memes langues que whisper-rs, mais avec streaming natif. C'est le choix principal.
2. **Moonshine (anglais seulement, optionnel)** : ultra-rapide sur CPU mais anglais uniquement. Propose comme option secondaire clairement etiquetee "English only".

**`src-tauri/src/models/registry.rs`** :
```rust
// Whisper ONNX via sherpa-rs — MULTILINGUE, streaming natif
ModelDefinition {
    id: "sherpa-whisper-small",
    name: "Whisper Small (ONNX streaming)",
    engine: "sherpa",
    repo: "csukuangfj/sherpa-onnx-whisper-small",  // a verifier
    filename: "whisper-small-encoder.onnx",  // multi-fichiers
    size_mb: 470,
    description: "Whisper Small with native streaming. 57 languages.",
    chat_template: "",
},

// Moonshine — ANGLAIS SEULEMENT, ultra-rapide CPU
ModelDefinition {
    id: "moonshine-tiny",
    name: "Moonshine Tiny (English only)",
    engine: "sherpa",
    repo: "usefulsensors/moonshine-tiny-onnx",
    filename: "moonshine-tiny-encoder.onnx",
    size_mb: 190,
    description: "Ultra-fast CPU. 5-15x faster than Whisper. English only.",
    chat_template: "",
},
```

**Attention** : les modeles ONNX sont multi-fichiers (encoder.onnx, decoder.onnx, tokens.json).
Le systeme de download actuel (single file via hf-hub) devra etre adapte pour telecharger un dossier ou archive.

**UX** : dans l'onglet Engines, les modeles "English only" sont clairement marques. Si la langue selectionnee n'est pas l'anglais et que l'utilisateur selectionne Moonshine, afficher un avertissement.

### C5. Streaming natif dans `streaming.rs`

Modifier `run_streaming()` pour verifier `stt_engine.supports_streaming()` :
- Si `true` : utiliser `create_stream()` + `accept_waveform()` — pas besoin de silence detection
- Si `false` : fallback Phase B (silence detection + `transcribe()` par segment)

### C6. Selection du moteur dans `do_setup()`

Le champ `SttConfig.active_engine` determine le moteur :
- `"whisper"` → `WhisperEngine` (existant)
- `"sherpa"` → `SherpaEngine` (Phase C)

Le modele actif determine automatiquement le moteur via le champ `engine` du `ModelDefinition`.

---

## Resume des fichiers modifies

### Phase A
| Fichier | Modification |
|---------|-------------|
| `src-tauri/src/config/schema.rs` | Ajouter `real_time: bool`, `stt_threads: Option<u32>` a `GeneralConfig` + Default impl |
| `src/types/index.ts` | Ajouter `real_time: boolean`, `stt_threads: number \| null` |
| `src/components/settings/GeneralTab.tsx` | Toggle real_time + slider CPU threads |
| `src/Overlay.tsx` | Toggle real_time + avertissement latence LLM |
| `src-tauri/src/models/registry.rs` | Modeles Distil-Whisper |
| `src-tauri/src/stt/whisper.rs` | `thread_limit: Arc<RwLock<Option<u32>>>` + ctor change + optimisations FullParams (A7+A8) |
| `src-tauri/src/app_state.rs` | Ajouter `stt_thread_limit: Arc<RwLock<Option<u32>>>` |
| `src-tauri/src/commands/gpu.rs` | Nouveau command `detect_cpu_count` |
| `src-tauri/src/commands/settings.rs` | Sync `stt_thread_limit` dans `update_settings` |
| `src-tauri/src/lib.rs` | Creer `stt_thread_limit` Arc dans `do_setup()`, enregistrer `detect_cpu_count` dans invoke_handler |
| `src/lib/commands.ts` | Ajouter `detectCpuCount()` |
| `src/lib/translations/{en,fr,zh}.ts` | Cles i18n |

### Phase B
| Fichier | Modification |
|---------|-------------|
| `src-tauri/Cargo.toml` | tokio `sync` feature |
| `src-tauri/src/audio/capture.rs` | `start_streaming()`, `resolve_input_device()`, getters |
| `src-tauri/src/audio/silence.rs` | **Nouveau** — detection silence RMS |
| `src-tauri/src/audio/mod.rs` | Declarer `pub mod silence;` |
| `src-tauri/src/streaming.rs` | **Nouveau** — boucle de streaming |
| `src-tauri/src/lib.rs` | Bifurcation batch/streaming dans `handle_record_start/stop`, `mod streaming;` |
| `src-tauri/src/app_state.rs` | Champs `streaming_handle`, `saved_clipboard` |
| `src-tauri/src/injection/mod.rs` | Methode default `inject_no_enter()` sur trait `TextInjector` |
| `src-tauri/src/events.rs` | `EVENT_TRANSCRIPTION_PARTIAL` |
| `src/Overlay.tsx` | Affichage texte partiel |
| `src/lib/events.ts` | `onTranscriptionPartial` |

### Phase C
| Fichier | Modification |
|---------|-------------|
| `src-tauri/Cargo.toml` | `sherpa-rs` (optional) |
| `src-tauri/src/stt/sherpa.rs` | **Nouveau** — SherpaEngine |
| `src-tauri/src/stt/mod.rs` | Trait `SttStream`, methodes streaming |
| `src-tauri/src/streaming.rs` | Branche streaming natif |
| `src-tauri/src/models/registry.rs` | Modeles Moonshine |
| `src-tauri/src/lib.rs` | Selection moteur dynamique |

---

## Decisions d'architecture

1. **`real_time` = streaming vs batch, pas de desactivation LLM** : toutes les fonctions post-traitement restent independamment configurables. Un avertissement latence est affiche si LLM actif en streaming, mais l'utilisateur choisit.

2. **Injection au curseur par segment** : chaque segment est injecte via clipboard+paste (meme mecanisme que le batch). Le presse-papiers est sauvegarde une fois au debut, restaure une fois a la fin. Enter n'est envoye qu'a la toute fin si `auto_enter=true`.

3. **Pipeline complet par segment via `pipeline::run_pipeline()`** : reutilise la MEME fonction que le batch. Garantit : prompt building, `strip_llm_artifacts()`, `is_available()`, timeout 60s, capitalize, spacing, reformulation, traduction, substitutions. DRY.

4. **`unbounded_channel()`** pour le streaming audio : pendant le LLM (5-30s CPU), la boucle ne fait pas `recv()`. Un bounded channel perdrait des chunks. Unbounded accumule sans perte (~2.9MB max pour 30s de stall). `send()` est non-bloquant dans le callback cpal.

5. **Pas de VAD ML en Phase B** : detection silence par RMS (zero dependance, ~0 CPU). Upgrade possible vers sherpa-rs VAD integre en Phase C.

6. **Resampler batch par segment** : reutilise `resample_to_16k_mono()` existant pour chaque segment. Pas de `StreamingResampler` — KISS.

7. **Injection via `TextInjector` trait + `spawn_blocking()`** : l'injection est bloquante (`thread::sleep`, `Clipboard::new`, `Enigo`). Doit etre dans `spawn_blocking()`. Utilise `text_injector` existant qui a le flag `is_simulating` pour le hook rdev.

8. **Isolation des phases** : le code batch existant est INCHANGE. La bifurcation se fait par `if real_time { ... } else { ... }` dans `handle_record_start/stop`.

9. **sherpa-rs optionnel** (Phase C) : feature flag `sherpa` pour ne pas alourdir le build CPU.

10. **handle_text_process non affecte** : le hotkey de traitement de texte fonctionne normalement quel que soit `real_time`.

11. **Espace entre segments** : chaque segment apres le 1er est prefixe d'un espace a l'injection. Sans ca, "Bonjour" + "le monde" donnerait "Bonjourle monde".

---

## Tests & Verification

### Tests unitaires (a ajouter)
- `silence.rs` : `compute_rms()`, `SilenceDetector` avec patterns connus (silence pur, parole simulee, mixte)
- `streaming.rs` : logique d'accumulation (mock receiver avec chunks pre-construits)

### Tests existants (47 — doivent tous passer)
```bash
cargo test  # 0 failures
```

### Build verification
```bash
cargo check                    # 0 errors
cargo check --features cuda    # 0 errors
npx tsc --noEmit               # 0 errors
```

### Tests manuels
1. **Phase A** : toggle `real_time` ON → config sauvegardee → toggle visible dans overlay
2. **Phase A** : slider CPU threads → valeur sauvegardee → plage 1-max correcte
3. **Phase A** : optimisations Whisper → transcrire du texte → pas de regression
4. **Phase B** : `real_time` ON + reformulation OFF → enregistrer → texte apparait au curseur segment par segment → overlay affiche le texte cumule → Enter a la fin si auto_enter
5. **Phase B** : `real_time` ON + reformulation ON → enregistrer → texte reformule injecte (avec latence) → avertissement visible dans overlay
6. **Phase B** : `real_time` ON → presse-papiers restaure apres fin d'enregistrement
7. **Phase B** : `real_time` OFF → comportement batch inchange
8. **Phase C** : selectionner modele Whisper ONNX (sherpa) → streaming natif token-par-token

---

## Verification approfondie (10 iterations)

### Ronde 1 (iterations 1-5) : Verification initiale
- 6 bugs trouves et corriges (voir "Changements cles" dans B4)
- `LlmBackend` trait : pas de `reformulate()`/`translate()` → reutilise `pipeline::run_pipeline()` ✓
- `TextInjector::inject()` bloquant → `spawn_blocking()` ✓
- `WindowsInjector.is_simulating` pour rdev hook → passe `text_injector` ✓
- Bounded channel → perte audio → `unbounded_channel()` ✓
- `stt_threads` jamais synchro → `Arc<RwLock<Option<u32>>>` partage ✓
- `clipboard_restore=false` ignore → verifie dans `handle_record_start` ✓

### Ronde 2, Iteration 6 : Conformite API — verification code source
- `pipeline::run_pipeline(raw_text, config, llm, source_language)` : 4 args verifies ✓
  - `backend.as_deref()` : `Option<Arc<dyn LlmBackend>>` → `Option<&dyn LlmBackend>` ✓ (meme pattern que lib.rs:315)
- `resample_to_16k_mono(samples, channels, source_rate)` : types `&[f32], u16, u32` verifies ✓
- `SttEngine::transcribe(&self, &[f32], Option<&str>)` : args verifies ✓
- `inject("", &InjectionOptions { auto_enter: true, clipboard_restore: false })` : set_text("") + paste vide + Enter ✓ (verifie dans windows.rs:44-92)
- `WhisperEngine::new(use_gpu, thread_limit)` : signature mise a jour ✓
- `WhisperEngine` struct : 3 champs (context, use_gpu, thread_limit) ✓
- `do_setup()` : creation `stt_thread_limit` Arc, passage a WhisperEngine ✓
- `detect_cpu_count` : dans `commands/gpu.rs` (meme pattern que `detect_nvidia`) ✓
- `detectCpuCount()` : dans `commands.ts` ✓

### Ronde 2, Iteration 7 : Nommage, imports, arguments
- Rust : snake_case (fonctions/variables), PascalCase (types), SCREAMING_SNAKE_CASE (constantes) ✓
- TypeScript : snake_case (struct fields pour miroir Rust), camelCase (fonctions) ✓
- `streaming.rs` imports complets : `std::sync::{Arc, Mutex, RwLock}`, `tauri::Emitter`, `crate::config::schema::{AppConfig, PostProcessingConfig}` ✓
- `mod streaming;` dans lib.rs ✓
- `stt_language` : filtre les chaines vides (meme pattern que lib.rs:262-270) ✓ (corrige dans B4)
- `process_segment()` : fonction helper extraite pour DRY (loop + flush) ✓
- `map_err(|e| AppError::Internal(format!("Task join error: {}", e)))` : pattern identique au batch ✓

### Ronde 2, Iteration 8 : Buffer management (bug critique corrige)
- **ANCIEN** : `raw_buffer[seg_start..seg_end]` + `drain(..seg_end)` + `reset_offset()` non definie → indices invalides apres 1er drain ✗
- **NOUVEAU** : `SilenceDetector` possede `speech_buffer: Vec<f32>` interne ✓
  - `process(&chunk)` retourne `Option<Vec<f32>>` (segment complet) ✓
  - `flush()` retourne le segment final (parole sans silence terminal) ✓
  - Plus de `raw_buffer`, plus d'indices, plus de `reset_offset()` ✓
  - `compute_rms()` avec guard `if samples.is_empty() { return 0.0; }` ✓
  - Parole trop courte (<300ms) : ignoree (`speech_end >= min_speech_samples`) ✓
  - Trailing silence trim : `self.speech_buffer[..speech_end].to_vec()` ✓

### Ronde 2, Iteration 9 : Gestion d'erreurs et cas limites
- Pipeline echoue sur un segment → `log::warn`, fallback texte brut, continue ✓
- LLM timeout 60s → gere par `pipeline::run_pipeline()` → warn + texte brut ✓
- Transcription echouee → erreur remonte (coherent avec batch qui retourne Err) ✓
- `inject_no_enter` echoue → erreur remonte (si on ne peut plus injecter, streaming inutile) ✓
- Channel ferme → `rx.recv()` retourne `None` → boucle termine naturellement ✓
- `compute_rms()` slice vide → retourne 0.0, pas de division par zero ✓
- Segment trop court → SilenceDetector ignore (< MIN_SPEECH_DURATION_MS) ✓
- `flush()` appele a la fin → segment final traite correctement ✓
- `inject("")` pour Enter final → fonctionne (paste vide + Enter) ✓
- Mutex/RwLock poisonne → `.map_err(|e| AppError::Internal(...))` partout ✓
- `JoinHandle.await` erreur → `.map_err(|e| AppError::Internal(format!("Task join error: {}", e)))` ✓

### Ronde 2, Iteration 10 : Completude et framework
- **Phase A completude** :
  - Config : `real_time`, `stt_threads` dans schema.rs + Default impl ✓
  - TypeScript : `real_time`, `stt_threads` dans types/index.ts ✓
  - Frontend : toggle GeneralTab, toggle Overlay, avertissement latence ✓
  - Backend : `detect_cpu_count` dans gpu.rs + invoke_handler ✓
  - Frontend : `detectCpuCount` dans commands.ts ✓
  - AppState : `stt_thread_limit` ✓
  - do_setup() : creation Arc, passage a WhisperEngine ✓
  - update_settings : sync stt_thread_limit ✓
  - WhisperEngine : constructeur modifie, `set_n_threads` dans transcribe ✓
  - FullParams optimisations : 4 set_* dans transcribe() ✓
  - Distil-Whisper models dans registry ✓
  - i18n : 8 cles en 3 langues ✓
- **Phase B completude** :
  - Cargo.toml : tokio sync feature ✓
  - AudioCapture : `start_streaming()`, `resolve_input_device()`, getters ✓
  - SilenceDetector : `process()` retourne `Option<Vec<f32>>`, `flush()` ✓
  - streaming.rs : `run_streaming()` + `process_segment()` helper ✓
  - lib.rs : bifurcation `handle_record_start/stop`, `mod streaming;` ✓
  - AppState : `streaming_handle`, `saved_clipboard` ✓
  - TextInjector : `inject_no_enter()` default method ✓
  - events.rs : `EVENT_TRANSCRIPTION_PARTIAL` ✓
  - Overlay : listener `transcription-partial`, affichage texte partiel ✓
- **Compatibilite** :
  - `#[serde(default)]` sur nouveaux champs → backward compat ✓
  - Batch code inchange → tests existants (47) non affectes ✓
  - `pipeline::run_pipeline()` signature inchangee ✓
  - `TextInjector` trait : ajout methode default → aucun impl casse ✓
  - Build CPU et CUDA inchanges ✓
  - tokio sync feature n'ajoute aucune dep externe ✓
