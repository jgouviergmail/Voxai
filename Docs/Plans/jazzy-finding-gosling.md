# Plan : 3 moteurs STT + Prompts anti-verbosite + Auto-detection langue

> Hypotheses verifiees sur internet en fevrier 2026.

## Synthese

| Fonctionnalite | Approche | Risque |
|----------------|----------|--------|
| **Parakeet** (NVIDIA) | In-process ONNX via `parakeet-rs` v0.3.1 | Faible |
| **Qwen3-ASR** (Alibaba) | Subprocess CLI via `qwen3-asr.cpp` | **Eleve** |
| **Voxtral** (Mistral) | Extension `voxai-llm-worker` (raw FFI `llama-cpp-sys-2` feature `mtmd`) | **Eleve** |
| **Prompts anti-verbosite** | Ajouter contraintes explicites aux 6 prompts de reformulation | Faible |
| **Auto-detection langue** | whisper-rs `full_lang_id_from_state()` + option "Auto" dans l'UI | Faible |

### Justifications architecturales (verifiees)

- **Parakeet in-process** : `parakeet-rs` v0.3.1 (crates.io) utilise `ort` 2.0 (ONNX Runtime 1.23) en interne. Aucun conflit de symboles ggml avec `whisper-rs`. API verifiee : `ParakeetTDT::from_pretrained(dir, config)` + `transcribe_samples(samples, sample_rate, channels)`. **Attention** : `transcribe_samples` prend `&mut self` → necessite interior mutability (`Mutex`) dans notre impl `SttEngine` (trait exige `&self`).

- **Qwen3-ASR subprocess** : `qwen3-asr.cpp` est CLI-only (pas d'API C). Utilise ggml → conflit de symboles obligatoire. **Probleme critique** : Pas de GGUF officiels sur HuggingFace — la conversion doit etre faite via les scripts du projet. L'integration necessite de distribuer les GGUF nous-memes ou d'attendre une publication communautaire.

- **Voxtral subprocess** : `llama-cpp-2` Rust crate n'expose PAS l'API multimodale (`llama_mtmd_*`). Deux options : (a) raw FFI via `llama-cpp-sys-2` feature `mtmd` dans le worker existant, ou (b) subprocess `llama-mtmd-cli`. Le mmproj est dans le repo `ggml-org/Voxtral-Mini-3B-2507-GGUF` (pas bartowski). Audio marque **experimental** par llama.cpp. Voxtral-4B-Realtime (fev 2026) n'est PAS dispo en GGUF (vLLM uniquement).

---

## Phase 0 : Infrastructure (pre-requis pour tout)

### 0.1 Multi-file models dans le registre

**Fichier** : `src-tauri/src/models/registry.rs`

Ajouter un champ `extra_files` a `ModelDefinition` :
```rust
pub struct ModelDefinition {
    // ... champs existants ...
    /// Additional files needed (empty for single-file models).
    /// Not serialized — compile-time only.
    #[serde(skip)]
    pub extra_files: &'static [&'static str],
}
```
Toutes les entrees existantes : `extra_files: &[]`.

### 0.2 Downloader multi-fichiers

**Fichier** : `src-tauri/src/models/downloader.rs`

- Extraire la logique de telechargement d'un seul fichier dans une sous-fonction `download_single_file()`.
- Si `def.extra_files` est non-vide :
  - Creer un sous-repertoire `{models_dir}/{model_id}/`
  - Telecharger `def.filename` + chaque `extra_files[i]` dans ce sous-repertoire
  - Progres agrege : `total_bytes` = somme de tous les fichiers, `downloaded_bytes` cumule
  - `cancel_token` partage entre tous les fichiers
  - Atomic : renommer le repertoire temporaire en final
- Si `def.extra_files` est vide : comportement actuel inchange (retrocompatibilite).

### 0.3 ModelCache multi-fichiers

**Fichier** : `src-tauri/src/models/cache.rs`

- Ajouter `model_dir(&self, model_id: &str) -> Option<PathBuf>` :
  - Si `extra_files` non vide : retourne `{models_dir}/{model_id}/` si le fichier primaire existe dedans
  - Si `extra_files` vide : retourne `None` (utiliser `model_path()` a la place)
- Mettre a jour `is_downloaded()` et `list_downloaded()` pour verifier aussi les modeles multi-fichiers
- Mettre a jour `remove_model()` pour supprimer le sous-repertoire si multi-fichiers

### 0.4 Engine factory + switching

**Fichier** : `src-tauri/src/stt/mod.rs`

Ajouter une factory :
```rust
pub fn create_engine(engine_id: &str) -> Result<Box<dyn SttEngine>, AppError> {
    match engine_id {
        "whisper" => Ok(Box::new(whisper::WhisperEngine::new())),
        "parakeet" => Ok(Box::new(parakeet::ParakeetEngine::new())),
        "qwen3-asr" => Ok(Box::new(qwen3_asr::Qwen3AsrEngine::new())),
        "voxtral" => Ok(Box::new(voxtral::VoxtralEngine::new())),
        _ => Err(AppError::Stt(format!("Unknown STT engine: {}", engine_id))),
    }
}
```

### 0.5 Engine switching dans `set_active_model`

**Fichier** : `src-tauri/src/commands/engines.rs`

Le bloc `else` (STT model, lignes 131-149) doit verifier si le `model_def.engine` differe de l'engine actuellement instancie. Si oui, swapper le `Box<dyn SttEngine>` :

```rust
} else {
    // STT model — swap engine if needed
    let current_engine_id = {
        let engine = state.stt_engine.lock()...;
        engine.id().to_string()
    };
    if model_def.engine != current_engine_id {
        let new_engine = stt::create_engine(model_def.engine)?;
        let mut guard = state.stt_engine.lock()...;
        *guard = new_engine;
    }
    // Load model into (possibly new) engine
    let model_path = /* model_path OU model_dir selon extra_files */;
    {
        let mut engine = state.stt_engine.lock()...;
        engine.load_model(&model_path)?;
    }
    // Update config
    {
        let mut config = state.config.write()...;
        config.stt.active_engine = model_def.engine.to_string();
        config.stt.active_model = Some(model_id.clone());
        persistence::save_and_notify(&config, &state.app_handle)?;
    }
}
```

### 0.6 Generaliser `list_engines` pour STT multiples

**Fichier** : `src-tauri/src/commands/engines.rs`

Remplacer le filtre hardcode `def.engine == "whisper"` par un groupement dynamique. Les types d'engines STT connus :
```rust
const STT_ENGINE_TYPES: &[(&str, &str)] = &[
    ("whisper", "Whisper (OpenAI)"),
    ("parakeet", "Parakeet (NVIDIA)"),
    ("qwen3-asr", "Qwen3-ASR (Alibaba)"),
    ("voxtral", "Voxtral (Mistral)"),
];
```
Pour chaque type, filtrer `MODEL_CATALOG`, construire un `EngineInfo`. Le champ `loaded` est `true` seulement si l'engine active (`stt.id()`) correspond ET `stt.is_loaded()`.

### 0.7 Corriger le "whisper" hardcode dans l'historique

**Fichier** : `src-tauri/src/lib.rs`, ligne 447

```rust
// Avant :
"whisper".to_string(),
// Apres :
{
    let stt = stt_engine.lock().map_err(|e| error::AppError::Internal(e.to_string()))?;
    stt.id().to_string()
}
```

### 0.8 Auto-load au demarrage respecte l'engine active

**Fichier** : `src-tauri/src/lib.rs`, lignes 125-164

- Lire `config.stt.active_engine` en plus de `config.stt.active_model`
- Si l'engine active != "whisper" : creer la bonne engine via `stt::create_engine()`
- Remplacer le `WhisperEngine` par defaut dans l'AppState
- Puis `load_model()` comme avant

### 0.9 Langues par engine

**Fichier** : `src-tauri/src/commands/engines.rs`

`list_supported_languages()` est hardcode sur `WHISPER_LANGUAGES`. Chaque engine doit exposer ses langues :
- Ajouter `fn supported_languages(&self) -> &[(&str, &str)]` au trait `SttEngine`
- `list_supported_languages` lit l'engine active et retourne ses langues

---

## Phase 1 : Parakeet (in-process via parakeet-rs)

### 1.1 Dependance

**Fichier** : `src-tauri/Cargo.toml`

```toml
parakeet-rs = { version = "0.3", features = ["download-binaries"] }
```

Note : `parakeet-rs` depend de `ort` qui telecharge automatiquement le runtime ONNX Runtime au build. Le feature `download-binaries` assure un build sans install systeme.

### 1.2 ParakeetEngine

**Nouveau fichier** : `src-tauri/src/stt/parakeet.rs`

**Contrainte API verifiee** : `transcribe_samples(&mut self, ...)` prend `&mut self`.
Notre trait `SttEngine::transcribe(&self, ...)` exige `&self`. Solution : `Mutex` interne.

```rust
use std::sync::Mutex;
use std::path::Path;
use crate::error::AppError;
use super::{SttEngine, TranscriptionResult, Segment};

pub struct ParakeetEngine {
    // Mutex car parakeet_rs::ParakeetTDT::transcribe_samples prend &mut self
    model: Mutex<Option<parakeet_rs::ParakeetTDT>>,
}

impl ParakeetEngine {
    pub fn new() -> Self {
        Self { model: Mutex::new(None) }
    }
}

impl SttEngine for ParakeetEngine {
    fn id(&self) -> &str { "parakeet" }
    fn name(&self) -> &str { "Parakeet" }

    fn load_model(&mut self, model_path: &Path) -> Result<(), AppError> {
        // model_path = directory contenant les 9 fichiers ONNX
        let model = parakeet_rs::ParakeetTDT::from_pretrained(
            model_path.to_str().unwrap_or(""),
            None, // ExecutionConfig par defaut (CPU)
        ).map_err(|e| AppError::Stt(format!("Failed to load Parakeet: {}", e)))?;
        *self.model.lock().map_err(|e| AppError::Internal(e.to_string()))? = Some(model);
        Ok(())
    }

    fn unload_model(&mut self) {
        if let Ok(mut guard) = self.model.lock() { *guard = None; }
    }
    fn is_loaded(&self) -> bool {
        self.model.lock().map(|g| g.is_some()).unwrap_or(false)
    }

    fn transcribe(&self, samples: &[f32], _language: Option<&str>) -> Result<TranscriptionResult, AppError> {
        let mut guard = self.model.lock().map_err(|e| AppError::Internal(e.to_string()))?;
        let model = guard.as_mut()
            .ok_or_else(|| AppError::Stt("Parakeet model not loaded".into()))?;
        let start = std::time::Instant::now();
        // API verifiee : transcribe_samples(&mut self, Vec<f32>, u32, u32) -> Result<TranscriptionResult>
        let result = model.transcribe_samples(samples.to_vec(), 16000, 1)
            .map_err(|e| AppError::Stt(format!("Parakeet transcription failed: {}", e)))?;
        // result.text: String, result.tokens: Vec<Token> (avec start/end/text)
        let segments = result.tokens.iter().map(|t| Segment {
            text: t.text.clone(),
            start_ms: (t.start * 1000.0) as u64,
            end_ms: (t.end * 1000.0) as u64,
        }).collect();
        Ok(TranscriptionResult {
            text: result.text,
            language: None, // TDT v3 detecte automatiquement
            segments,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn supported_languages(&self) -> &[(&str, &str)] {
        &PARAKEET_LANGUAGES
    }
}
```

Langues supportees (Parakeet TDT v3 = 25 langues europeennes) :
```rust
const PARAKEET_LANGUAGES: &[(&str, &str)] = &[
    ("en", "English"), ("de", "German"), ("es", "Spanish"), ("fr", "French"),
    ("it", "Italian"), ("pt", "Portuguese"), ("nl", "Dutch"), ("pl", "Polish"),
    // ... 25 langues au total — a verifier sur le model card NVIDIA
];
```

### 1.3 Modeles dans le registre

**Fichier** : `src-tauri/src/models/registry.rs`

**Fichiers modele verifies** (9 fichiers necessaires pour TDT) :
- `encoder-model.onnx` + `encoder-model.onnx_data` (poids externes)
- `decoder_joint-model.onnx`
- `vocab.txt`, `config.json`, `preprocessor_config.json`
- `tokenizer.json`, `tokenizer_config.json`
- `nemo128.onnx` (preprocesseur audio)

```rust
// Parakeet STT models
ModelDefinition {
    id: "parakeet-tdt-0.6b",
    name: "Parakeet TDT 0.6B",
    engine: "parakeet",
    repo: "nvidia/parakeet-tdt-0.6b-v2",  // anglais seulement
    filename: "encoder-model.onnx",
    extra_files: &[
        "encoder-model.onnx_data", "decoder_joint-model.onnx",
        "vocab.txt", "config.json", "preprocessor_config.json",
        "tokenizer.json", "tokenizer_config.json", "nemo128.onnx",
    ],
    size_mb: 2400,
    description: "Fastest STT. English only. #1 on HuggingFace ASR leaderboard.",
    chat_template: "",
},
ModelDefinition {
    id: "parakeet-tdt-0.6b-multilingual",
    name: "Parakeet TDT 0.6B Multilingual",
    engine: "parakeet",
    repo: "nvidia/parakeet-tdt-0.6b-v3",  // 25 langues
    filename: "encoder-model.onnx",
    extra_files: &[
        "encoder-model.onnx_data", "decoder_joint-model.onnx",
        "vocab.txt", "config.json", "preprocessor_config.json",
        "tokenizer.json", "tokenizer_config.json", "nemo128.onnx",
    ],
    size_mb: 2400,
    description: "Fastest multilingual STT. 25 European languages. Auto-detect.",
    chat_template: "",
},
```

**Note** : Les repos NVIDIA distribuent en format `.nemo`. Les fichiers ONNX doivent venir des repos communautaires (`istupakov/parakeet-tdt-0.6b-v3-onnx`) ou etre exportes via `parakeet-rs`. Verifier les noms exacts a l'implementation. `parakeet-rs` n'a PAS d'auto-download — notre downloader gere tout.

### 1.4 Module declaration

**Fichier** : `src-tauri/src/stt/mod.rs`

```rust
pub mod whisper;
pub mod parakeet;
```

---

## Phase 2 : Qwen3-ASR (subprocess worker)

### Architecture

Comme pour le LLM worker (ggml symbol collision), on cree un binaire separe `voxai-stt-worker` qui embarque `qwen3-asr.cpp`.

**Protocole** : Le worker est une CLI simple (pas de pipe bidirectionnel continu) :
```
voxai-stt-worker --engine qwen3 --model <path.gguf> --audio <path.wav> [--language <code>]
```
Stdout : JSON `{"text": "...", "segments": [...], "language": "..."}` ou `{"error": "..."}`

Ce pattern est plus simple que le JSON-over-pipe du LLM worker car la transcription est un one-shot (pas un stream).

### 2.1 Nouveau crate workspace

**Nouveau repertoire** : `src-tauri/stt-worker/`

**Fichier** : `src-tauri/stt-worker/Cargo.toml`
```toml
[package]
name = "voxai-stt-worker"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
hound = "3"  # lecture fichiers WAV
```

**Fichier** : `src-tauri/stt-worker/build.rs`
- Utilise `cc` ou `cmake` pour compiler `qwen3-asr.cpp` depuis les sources vendorisees
- Genere les bindings FFI via `bindgen` si necessaire
- Ou alternative plus simple : compiler qwen3-asr.cpp comme un executable separe et le redistribuer

**Fichier** : `src-tauri/stt-worker/src/main.rs`
```rust
fn main() {
    let args = parse_args(); // --engine, --model, --audio, --language
    match args.engine.as_str() {
        "qwen3" => run_qwen3(&args),
        _ => eprintln!("Unknown engine"),
    }
}

fn run_qwen3(args: &Args) {
    // 1. Load GGUF model via FFI to qwen3-asr.cpp
    // 2. Read WAV file via hound
    // 3. Transcribe
    // 4. Print JSON result to stdout
}
```

**Workspace** : `src-tauri/Cargo.toml` :
```toml
[workspace]
members = [".", "llm-worker", "stt-worker"]
```

### 2.2 Qwen3AsrEngine (proxy)

**Nouveau fichier** : `src-tauri/src/stt/qwen3_asr.rs`

```rust
pub struct Qwen3AsrEngine {
    model_path: Option<PathBuf>,
}

impl SttEngine for Qwen3AsrEngine {
    fn id(&self) -> &str { "qwen3-asr" }
    fn name(&self) -> &str { "Qwen3-ASR" }

    fn load_model(&mut self, model_path: &Path) -> Result<(), AppError> {
        // Verify model file exists, store path
        // Optionally: spawn worker once to verify it loads correctly
        self.model_path = Some(model_path.to_path_buf());
        Ok(())
    }

    fn transcribe(&self, samples: &[f32], language: Option<&str>) -> Result<TranscriptionResult, AppError> {
        let model_path = self.model_path.as_ref().ok_or(...)?;
        // 1. Write samples to temp WAV file (16kHz mono f32 → i16)
        let temp_wav = write_temp_wav(samples)?;
        // 2. Spawn voxai-stt-worker --engine qwen3 --model <path> --audio <temp_wav>
        let output = Command::new(worker_binary_path()?)
            .args(["--engine", "qwen3", "--model", model_path.to_str()..., "--audio", temp_wav...])
            .output()?;
        // 3. Parse JSON output
        // 4. Delete temp file
        // 5. Return TranscriptionResult
    }
}
```

**Helper** : `write_temp_wav(samples: &[f32]) -> Result<PathBuf, AppError>` — Ecrit les echantillons 16kHz mono en WAV dans un fichier temporaire. Utilise `hound` ou ecriture manuelle du header WAV.

### 2.3 Modeles dans le registre

**PROBLEME VERIFIE** : Il n'existe PAS de GGUF officiels sur HuggingFace pour Qwen3-ASR.
Les modeles Qwen3-ASR sont distribues en format PyTorch (safetensors). La conversion en GGUF doit etre faite manuellement via les scripts de `qwen3-asr.cpp`.

**Options** :
- (a) Publier nos propres GGUF convertis sur un repo HF Voxai
- (b) Attendre que la communaute publie des GGUF (pas encore fait en fev 2026)
- (c) Integrer le script de conversion dans le workflow de telechargement (complexe, necessite Python)

**Recommendation** : Option (a) — convertir et publier sur un repo `voxai-community/Qwen3-ASR-0.6B-GGUF`.

```rust
ModelDefinition {
    id: "qwen3-asr-0.6b-q8",
    name: "Qwen3-ASR 0.6B (Q8)",
    engine: "qwen3-asr",
    repo: "voxai-community/Qwen3-ASR-0.6B-GGUF",  // repo a creer avec GGUF pre-convertis
    filename: "qwen3-asr-0.6b-q8_0.gguf",
    extra_files: &[],
    size_mb: 1300,
    description: "Excellente precision. 52 langues + dialectes chinois.",
    chat_template: "",
},
ModelDefinition {
    id: "qwen3-asr-0.6b-f16",
    name: "Qwen3-ASR 0.6B (F16)",
    engine: "qwen3-asr",
    repo: "voxai-community/Qwen3-ASR-0.6B-GGUF",
    filename: "qwen3-asr-0.6b-f16.gguf",
    extra_files: &[],
    size_mb: 1800,
    description: "Meilleure precision (pleine precision). 52 langues.",
    chat_template: "",
},
```

---

## Phase 3 : Voxtral (subprocess via llama-mtmd-cli ou raw FFI)

### Architecture (verifiee fev 2026)

**Faits verifies** :
- `llama-cpp-2` Rust crate n'expose PAS l'API multimodale (`llama_mtmd_*`)
- `llama-cpp-sys-2` expose les raw bindings C via feature `mtmd`
- `llama-mtmd-cli` est le CLI officiel llama.cpp pour le multimodal audio
- Le mmproj est dans le repo `ggml-org/Voxtral-Mini-3B-2507-GGUF` (716 MB, Q8_0)
- Les modeles GGUF principaux sont dans `bartowski/mistralai_Voxtral-Mini-3B-2507-GGUF` (24 quantisations)
- Audio est marque **experimental** par llama.cpp
- Voxtral-Mini-4B-Realtime (fev 2026) n'est PAS dispo en GGUF (vLLM uniquement)

**Approche choisie** : Etendre le `voxai-llm-worker` avec raw FFI via `llama-cpp-sys-2` feature `mtmd`. C'est preferable au subprocess `llama-mtmd-cli` car :
- Le worker est deja un process isole (pas de conflit ggml)
- On reutilise l'infrastructure existante (process spawn, protocol JSON)
- Plus de controle sur le format de sortie

### 3.1 Extension du LLM worker avec multimodal

**Fichier** : `src-tauri/llm-worker/Cargo.toml`

```toml
[dependencies]
llama-cpp-2 = "0.1"
llama-cpp-sys-2 = { version = "0.1", features = ["mtmd"] }
hound = "3"  # pour lire les WAV
```

**Fichier** : `src-tauri/llm-worker/src/main.rs`

Ajouter au protocole JSON stdin/stdout :
```json
// Nouvelle requete :
{"command": "transcribe", "audio_path": "C:/tmp/audio.wav", "language": "fr"}

// Reponse :
{"text": "transcription...", "segments": []}
```

Le worker :
1. Si lance avec `--mmproj <path>`, active le mode multimodal
2. Charge le modele GGUF principal + mmproj via raw FFI `llama_mtmd_*`
3. Sur commande `transcribe` : lit le WAV, encode l'audio, genere le texte
4. Retourne le JSON sur stdout

**CLI etendu** :
```
voxai-llm-worker <model-path> [chat-template] [--gpu-layers N] [--mmproj <mmproj-path>]
```

**Fallback** : Si le raw FFI `mtmd` est trop complexe, utiliser `llama-mtmd-cli` en subprocess :
```bash
llama-mtmd-cli -m model.gguf --mmproj mmproj.gguf --audio audio.wav -p "Transcribe this audio." -n 512
```
Parse stdout pour extraire la transcription.

### 3.2 VoxtralEngine (proxy)

**Nouveau fichier** : `src-tauri/src/stt/voxtral.rs`

```rust
pub struct VoxtralEngine {
    worker: Mutex<Option<WorkerHandle>>,  // meme pattern que LocalLlmBackend
}

impl SttEngine for VoxtralEngine {
    fn id(&self) -> &str { "voxtral" }
    fn name(&self) -> &str { "Voxtral" }

    fn load_model(&mut self, model_path: &Path) -> Result<(), AppError> {
        // model_path = directory contenant main.gguf + mmproj.gguf
        let main_gguf = model_path.join("Voxtral-Mini-3B-2507-Q4_K_M.gguf");
        let mmproj = model_path.join("mmproj-Voxtral-Mini-3B-2507-Q8_0.gguf");
        // Spawn voxai-llm-worker <main_gguf> --mmproj <mmproj>
        // Wait for readiness ping
    }

    fn transcribe(&self, samples: &[f32], language: Option<&str>) -> Result<TranscriptionResult, AppError> {
        // 1. Write samples to temp WAV (16kHz mono)
        // 2. Send {"command": "transcribe", "audio_path": "...", "language": "..."} via stdin
        // 3. Read JSON response from stdout
        // 4. Delete temp WAV
    }
}
```

### 3.3 Modeles dans le registre

**IMPORTANT** : Les fichiers viennent de DEUX repos differents :
- Modele principal : `bartowski/mistralai_Voxtral-Mini-3B-2507-GGUF`
- mmproj : `ggml-org/Voxtral-Mini-3B-2507-GGUF`

Le downloader doit supporter des repos differents par fichier. Options :
- (a) Ajouter un champ `extra_repos` a `ModelDefinition` pour mapper chaque extra_file a son repo
- (b) Publier main + mmproj dans un repo unique (notre propre mirror)
- (c) Telecharger le mmproj separement avec une logique speciale

**Recommendation** : Option (a) pour la genericite.

```rust
ModelDefinition {
    id: "voxtral-mini-3b-q4",
    name: "Voxtral Mini 3B (Q4)",
    engine: "voxtral",
    repo: "bartowski/mistralai_Voxtral-Mini-3B-2507-GGUF",
    filename: "Voxtral-Mini-3B-2507-Q4_K_M.gguf",
    extra_files: &["mmproj-Voxtral-Mini-3B-2507-Q8_0.gguf"],
    extra_repos: &["ggml-org/Voxtral-Mini-3B-2507-GGUF"],  // repo pour chaque extra_file
    size_mb: 3200,  // 2470 + 716 (mmproj)
    description: "Qualite premium multilingue. 8+ langues. Audio experimental. Apache 2.0.",
    chat_template: "",
},
ModelDefinition {
    id: "voxtral-mini-3b-q5",
    name: "Voxtral Mini 3B (Q5)",
    engine: "voxtral",
    repo: "bartowski/mistralai_Voxtral-Mini-3B-2507-GGUF",
    filename: "Voxtral-Mini-3B-2507-Q5_K_M.gguf",
    extra_files: &["mmproj-Voxtral-Mini-3B-2507-Q8_0.gguf"],
    extra_repos: &["ggml-org/Voxtral-Mini-3B-2507-GGUF"],
    size_mb: 3590,  // 2870 + 716
    description: "Meilleure qualite. 8+ langues. Audio experimental. Apache 2.0.",
    chat_template: "",
},
```

---

## Phase 4 : Frontend + i18n

### 4.1 EnginesTab — deja generique

Le composant `EnginesTab.tsx` itere deja sur `EngineInfo[]` de facon generique. Aucun changement structurel n'est necessaire — les nouveaux groupes STT apparaitront automatiquement quand le backend les retournera.

### 4.2 Traductions

**Fichiers** : `src/lib/translations/en.ts`, `fr.ts`, `zh.ts`

Ajouter les cles pour les descriptions d'engines si necessaire. Les noms et descriptions de modeles viennent deja du backend (`engine.name`, `model.description`).

### 4.3 Liste des langues par engine

Le frontend `GeneralTab.tsx` appelle `listSupportedLanguages()` qui est actuellement hardcode sur Whisper. Apres la Phase 0.9, cette commande retournera les langues de l'engine active. Aucun changement frontend — le select se met a jour automatiquement quand l'engine change (via `settings-updated` event → re-fetch config → re-fetch languages si l'engine a change).

---

## Phase 5 : Prompts anti-verbosite (reformulation + traduction)

### Probleme

Les 6 prompts de reformulation sont **permissifs** — ils ne contraignent pas le LLM a repondre UNIQUEMENT avec le texte reformule. Resultat : les petits modeles locaux (Mistral 7B, Phi-3, etc.) ajoutent souvent des explications, des commentaires, ou des prefixes comme "Here is the cleaned text:" avant le resultat.

Seul le prompt de traduction a deja la contrainte `"Output ONLY the translated text, nothing else."` et fonctionne correctement.

### 5.1 Mettre a jour les 6 prompts de reformulation

**Fichier** : `src-tauri/src/llm/prompt_templates.rs`

Ajouter une contrainte explicite dans le **system prompt** de chaque style. La contrainte doit etre en fin de system prompt pour etre la plus saillante possible :

```rust
"Cleaned" => Some((
    "You are a text cleanup assistant. Fix grammar, punctuation, and minor errors while preserving the original meaning and tone. Do not add or remove information. Output ONLY the corrected text, nothing else — no explanations, no preamble.",
    "Clean up the following dictated text:",
)),
"Professional" => Some((
    "You are a professional writing assistant. Reformulate text into clear, formal, business-appropriate language. Output ONLY the reformulated text, nothing else — no explanations, no preamble.",
    "Reformulate the following text in a professional, formal tone:",
)),
"Casual" => Some((
    "You are a friendly writing assistant. Reformulate text into natural, conversational language. Output ONLY the reformulated text, nothing else — no explanations, no preamble.",
    "Reformulate the following text in a casual, friendly tone:",
)),
"Concise" => Some((
    "You are a concise writing assistant. Shorten text while preserving all key information. Remove filler words and redundancy. Output ONLY the shortened text, nothing else — no explanations, no preamble.",
    "Make the following text more concise:",
)),
"Simplified" => Some((
    "You are a plain language assistant. Simplify text to be easily understood by everyone. Use short sentences and common words. Output ONLY the simplified text, nothing else — no explanations, no preamble.",
    "Simplify the following text:",
)),
"Structured" => Some((
    "You are a structured writing assistant. Organize text into clear paragraphs or bullet points when appropriate. Output ONLY the restructured text, nothing else — no explanations, no preamble.",
    "Restructure the following text for clarity:",
)),
```

**Principes appliques** :
- Contrainte `"Output ONLY ... nothing else — no explanations, no preamble."` en fin de system prompt (derniere phrase = plus saillante pour le LLM)
- Instructions utilisateur simplifiees (suppression des explications redondantes deja dans le system prompt)
- Meme pattern que le prompt de traduction qui fonctionne deja

### 5.2 Renforcer le prompt de traduction

**Fichier** : `src-tauri/src/llm/prompt_templates.rs`

Le prompt de traduction est deja bon mais peut etre renforce :

```rust
pub fn build_translation_prompt(text: &str, target_language: &str) -> Prompt {
    let lang = language_name(target_language);
    Prompt {
        system: format!(
            "You are a professional translator. Translate the given text to {}. \
             Preserve the original tone and meaning. Output ONLY the translated text, \
             nothing else — no explanations, no preamble, no quotation marks.",
            lang
        ),
        user: text.to_string(),
    }
}
```

### 5.3 Post-processing de securite (strip preamble)

**Fichier** : `src-tauri/src/postprocessing/pipeline.rs`

Meme avec les contraintes, certains modeles ajoutent quand meme un prefixe. Ajouter un nettoyage leger en post-traitement du resultat LLM :

```rust
/// Strip common LLM preambles that leak through despite prompt constraints.
fn strip_preamble(text: &str) -> &str {
    // Common patterns: "Here is the ...: ", "Sure, here...: ", etc.
    // Only strip if the text contains a colon+newline pattern in the first 100 chars
    if let Some(pos) = text[..text.len().min(100)].find(":\n") {
        let after = text[pos + 2..].trim_start();
        if !after.is_empty() {
            return after;
        }
    }
    text
}
```

Appliquer apres chaque appel LLM dans `run_pipeline()` :
```rust
Ok(reformulated) => {
    let cleaned = strip_preamble(&reformulated);
    if !cleaned.is_empty() {
        text = cleaned.to_string();
    }
}
```

### 5.4 Tests

Ajouter des tests unitaires pour `strip_preamble` :
```rust
#[test]
fn test_strip_preamble_with_prefix() {
    assert_eq!(strip_preamble("Here is the corrected text:\nHello world."), "Hello world.");
}
#[test]
fn test_strip_preamble_no_prefix() {
    assert_eq!(strip_preamble("Hello world."), "Hello world.");
}
#[test]
fn test_strip_preamble_colon_in_text() {
    // Don't strip if the colon is deep in the text
    let text = "This is a long sentence that contains a colon: but it's not a preamble because it's beyond 100 chars boundary.";
    assert_eq!(strip_preamble(text), text);
}
```

---

## Phase 6 : Detection automatique de la langue

### Probleme

L'utilisateur doit actuellement choisir manuellement la langue dans les parametres. Tous les moteurs STT supportent l'auto-detection :
- **Whisper** : `full_lang_id_from_state()` + `get_lang_str()` (API verifiee dans whisper-rs 0.15.1)
- **Parakeet TDT v3** : detecte automatiquement parmi 25 langues
- **Qwen3-ASR** : detecte parmi 52+ langues
- **Voxtral** : detecte parmi 8+ langues

L'infrastructure est **deja prete** : le trait `SttEngine::transcribe()` accepte `Option<&str>`, et `TranscriptionResult.language` est `Option<String>`. Il suffit de brancher le tout.

### 6.1 Capturer la langue detectee dans WhisperEngine

**Fichier** : `src-tauri/src/stt/whisper.rs`

Apres `state.full(params, samples)`, extraire la langue :

```rust
use whisper_rs::get_lang_str;

// ... apres state.full(params, samples) ...

let detected_lang = {
    let lang_id = state.full_lang_id_from_state();
    get_lang_str(lang_id as i32).map(|s| s.to_string())
};

Ok(TranscriptionResult {
    text: text.trim().to_string(),
    language: detected_lang,  // etait: None
    segments,
    duration_ms,
})
```

### 6.2 Config : language devient Option<String>

**Fichier** : `src-tauri/src/config/schema.rs`

```rust
pub struct GeneralConfig {
    // ...
    /// STT language. None or empty = auto-detect.
    #[serde(default)]
    pub language: Option<String>,
    // ...
}
```

Default : `None` (auto-detect par defaut — meilleure UX pour la premiere utilisation).

**Migration** : `#[serde(default)]` + `Option<String>` est backwards-compatible. Un ancien config avec `"language": "fr"` sera deserialisee en `Some("fr")`. Un absent sera `None`.

### 6.3 Pipeline : passer la langue auto-detectee

**Fichier** : `src-tauri/src/lib.rs`

Dans `run_pipeline()` (lignes 374-380), adapter :

```rust
let stt_language = {
    let cfg = config.read().map_err(|e| error::AppError::Internal(e.to_string()))?;
    match &cfg.general.language {
        Some(lang) if !lang.is_empty() => Some(lang.clone()),
        _ => None,  // auto-detect
    }
};
```

C'est deja presque ca — juste changer le type de `String` a `Option<String>`.

### 6.4 Backend : ajouter "Auto" a la liste des langues

**Fichier** : `src-tauri/src/commands/engines.rs`

Dans `list_supported_languages()`, prepend une entree "Auto" :

```rust
pub fn list_supported_languages(state: tauri::State<'_, AppState>) -> Vec<LanguageInfo> {
    let mut result = vec![LanguageInfo {
        code: "".to_string(),    // empty = auto-detect
        name: "Auto-detect".to_string(),
    }];
    // ... existing priority + rest logic ...
    result.extend(priority);
    result
}
```

### 6.5 Frontend : mettre a jour GeneralTab.tsx

**Fichier** : `src/components/settings/GeneralTab.tsx`

Le `<Select>` pour la langue (lignes 111-119) doit gerer `null`/`""` comme "Auto" :

```tsx
<Select
  label={i18n.t("general.language")}
  value={config().general.language ?? ""}
  options={languages().map((l) => ({
    value: l.code,
    label: l.name,
  }))}
  onChange={(v) => save((c) => (c.general.language = v || null))}
/>
```

Pas de changement structurel — le `""` venant de la liste correspond a "Auto-detect", et `save` convertit `""` en `null`.

### 6.6 Traductions i18n

**Fichiers** : `src/lib/translations/en.ts`, `fr.ts`, `zh.ts`

Ajouter :
```
"general.auto_detect": "Auto-detect" / "Detection automatique" / "自动检测"
```

Note : Le nom "Auto-detect" vient du backend (`LanguageInfo.name`), pas de l'i18n frontend. Mais pour le label du champ, ajouter la cle si on veut localiser.

### 6.7 Historique : stocker la langue detectee

**Fichier** : `src-tauri/src/history/mod.rs`

Le `HistoryEntry` a probablement un champ `language` ou similaire. Remplir avec `result.language` (la langue detectee retournee par `TranscriptionResult`).

**Fichier** : `src-tauri/src/lib.rs`, dans la creation de l'entree historique :
```rust
language: result.language.unwrap_or_else(|| stt_language.unwrap_or_default()),
```

---

## Fichiers impactes (resume)

### Rust (backend)

| Fichier | Modification |
|---------|-------------|
| `Cargo.toml` | +`parakeet-rs`, workspace +`stt-worker` |
| `src/stt/mod.rs` | +`pub mod parakeet; pub mod qwen3_asr; pub mod voxtral;`, +`create_engine()`, +`supported_languages()` dans le trait |
| `src/stt/whisper.rs` | Capturer la langue detectee via `full_lang_id_from_state()` |
| `src/stt/parakeet.rs` | **NOUVEAU** — ParakeetEngine impl |
| `src/stt/qwen3_asr.rs` | **NOUVEAU** — Qwen3AsrEngine proxy (subprocess) |
| `src/stt/voxtral.rs` | **NOUVEAU** — VoxtralEngine proxy (LLM worker) |
| `src/models/registry.rs` | +`extra_files` field, +6 modeles (2 Parakeet, 2 Qwen3, 2 Voxtral) |
| `src/models/downloader.rs` | +`download_single_file()`, support multi-fichiers, sous-repertoires |
| `src/models/cache.rs` | +`model_dir()`, MAJ `is_downloaded()`/`list_downloaded()`/`remove_model()` |
| `src/commands/engines.rs` | Generaliser `list_engines` (multi-STT), engine switching, langues par engine, +Auto-detect |
| `src/config/schema.rs` | `language: String` → `Option<String>` (auto-detect par defaut) |
| `src/lib.rs` | Fix "whisper" hardcode, auto-load respecte `active_engine`, language Option |
| `src/llm/prompt_templates.rs` | 6 prompts reformulation + traduction : ajout contraintes anti-verbosite |
| `src/postprocessing/pipeline.rs` | +`strip_preamble()` nettoyage post-LLM |
| `src/error.rs` | +`From<parakeet_rs::Error>` si necessaire |

### Nouveau crate : stt-worker

| Fichier | Contenu |
|---------|---------|
| `stt-worker/Cargo.toml` | Dependances : serde, serde_json, hound + FFI qwen3-asr.cpp |
| `stt-worker/build.rs` | Compilation qwen3-asr.cpp depuis sources vendorisees |
| `stt-worker/src/main.rs` | CLI : --engine qwen3 --model X --audio Y → JSON stdout |

### Extension LLM worker

| Fichier | Modification |
|---------|-------------|
| `llm-worker/src/main.rs` | +commande `transcribe` (audio multimodal), +flag `--mmproj` |

### TypeScript (frontend)

| Fichier | Modification |
|---------|-------------|
| `GeneralTab.tsx` | Language select : gerer `null` / `""` = auto-detect |
| `EnginesTab.tsx` | Deja generique, aucun changement structurel |
| `translations/en.ts`, `fr.ts`, `zh.ts` | +cles auto-detect, optionnel : descriptions d'engines |

---

## Risques et mitigations (verifies fev 2026)

### Risque 1 : Fichiers ONNX Parakeet (MOYEN) ✓ verifie
Les repos NVIDIA distribuent en `.nemo`, pas ONNX directement. 9 fichiers ONNX necessaires.
**Mitigation** : Utiliser le repo `istupakov/parakeet-tdt-0.6b-v3-onnx` ou `altunenes/parakeet-rs` HF. Verifier les noms exacts a l'implementation.

### Risque 2 : `parakeet-rs` `&mut self` (FAIBLE) ✓ verifie
`transcribe_samples(&mut self, ...)` alors que notre trait exige `&self`.
**Mitigation** : `Mutex` interne dans `ParakeetEngine` (deja planifie ci-dessus).

### Risque 3 : Send+Sync de `ParakeetTDT` (MOYEN) ✓ a verifier au build
Non documente. ONNX Session est generalement thread-safe pour l'inference.
**Mitigation** : Verifier a la compilation. Si pas Send/Sync, wrapper dans `Arc<Mutex<>>` avec `unsafe impl Send/Sync` si justifie.

### Risque 4 : Pas de GGUF Qwen3-ASR sur HuggingFace (ELEVE) ✓ verifie
Les GGUF doivent etre convertis manuellement. Aucun repo communautaire publie en fev 2026.
**Mitigation** : Convertir et publier sur un repo HF Voxai. Ou reporter Qwen3-ASR en attendant la communaute.

### Risque 5 : Build C++ de qwen3-asr.cpp (ELEVE) ✓ verifie
CLI-only, pas d'API C. Necessite FFI manuels ou invocation CLI.
**Mitigation** : Option A (prefere) : compiler qwen3-asr.cpp en CLI, l'invoquer via `Command`. Option B : FFI via `cc`/`cmake`. Option C : reporter.

### Risque 6 : llama-cpp-2 PAS de multimodal (CONFIRME) ✓ verifie
Le crate n'expose pas `llama_mtmd_*`. Raw bindings disponibles via `llama-cpp-sys-2` feature `mtmd`.
**Mitigation** : Utiliser les raw FFI dans le worker. Fallback : `llama-mtmd-cli` en subprocess.

### Risque 7 : Voxtral audio experimental (MOYEN) ✓ verifie
llama.cpp marque l'audio comme "highly experimental and may have reduced quality".
**Mitigation** : Indiquer clairement "experimental" dans la description du modele. Tester la qualite avant publication.

### Risque 8 : Taille du binaire (MOYEN)
`parakeet-rs` / `ort` ajoute ONNX Runtime (~50-150 MB de DLL partagee).
**Mitigation** : Prix inevitable pour ONNX local. Partage entre Parakeet et futurs modeles ONNX.

### Risque 9 : Repos multiples pour Voxtral (FAIBLE) ✓ verifie
mmproj dans `ggml-org/`, modele principal dans `bartowski/`. Necessite un champ `extra_repos`.
**Mitigation** : Ajouter `extra_repos` a `ModelDefinition` (cf. Phase 0.1).

---

## Ordre d'implementation

```
Phase 5 : Prompts anti-verbosite      (rapide — peut etre fait en premier)
Phase 6 : Auto-detection langue        (rapide — independant des moteurs)
Phase 0 : Infrastructure STT           (pre-requis pour les 3 moteurs)
Phase 1 : Parakeet                     (livrable rapidement)
Phase 2 : Qwen3-ASR                    (risque eleve)
Phase 3 : Voxtral                      (risque eleve)
Phase 4 : Frontend + i18n              (incremental)
```

**Recommendation** : Commencer par Phases 5+6 (ameliorations rapides, zero risque), puis enchainer Phase 0+1 (Parakeet fonctionnel), et enfin Qwen3-ASR + Voxtral en parallele.

---

## Verification

Apres chaque phase :
1. `cargo check -p voxai && cargo check -p voxai-llm-worker` — 0 erreurs
2. `npx tsc --noEmit` — 0 erreurs
3. `cargo test --lib -p voxai` — tous les tests passent
4. `cargo tauri dev` — app demarre sans panic

Tests fonctionnels — Phases 5+6 :
- Reformulation avec petit modele → resultat direct, pas de "Here is..." preamble
- Traduction → resultat direct, pas de guillemets superflus
- Language selector montre "Auto-detect" en premier
- Transcription sans langue selectionnee → langue detectee correctement

Tests fonctionnels — Moteurs STT :
- Telecharger un modele Parakeet (multi-fichiers) → progression OK
- Activer Parakeet → engine swappee, transcription fonctionne
- Revenir a Whisper → swap inverse, toujours fonctionnel
- Historique affiche le bon nom d'engine
- Liste des langues change selon l'engine active
- Annuler un telechargement multi-fichiers → nettoyage des fichiers temporaires
