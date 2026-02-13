# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Voxai is a Windows desktop voice dictation app built with **Tauri 2** (Rust backend) + **Solid.js** (TypeScript frontend). All speech recognition and text processing runs locally. Push-to-talk recording, Whisper STT, LLM-based reformulation/translation, and text injection into the active application.

## Build Commands

Environment variables are auto-detected by `scripts/env-setup.mjs`, but for manual PowerShell sessions:

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:LIBCLANG_PATH = "C:\Program Files\Unity\Hub\Editor\6000.3.6f1\Editor\Data\PlaybackEngines\WebGLSupport\BuildTools\Emscripten\llvm"
# Only for CUDA builds:
$env:CUDA_PATH = "C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.9"
```

| Task | Command |
|------|---------|
| Frontend dev (Vite HMR) | `npm run dev` |
| Full dev (CPU) | `npm run dev:cpu` |
| Full dev (CUDA) | `npm run dev:cuda` |
| Production build (CPU) | `npm run build:windows` |
| Production build (CUDA) | `npm run build:windows:cuda` |
| Both distributions | `npm run build:all` |
| Type-check frontend | `tsc` |
| Rust tests | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Rust check | `cargo check --manifest-path src-tauri/Cargo.toml` |
| Worker tests | `cargo test --manifest-path src-tauri/llm-worker/Cargo.toml` |

When running cargo commands from bash (not PowerShell), set `LIBCLANG_PATH` and add cargo to `PATH` first.

## Architecture

### Workspace Layout

```
src-tauri/           Rust backend (main crate: "voxai")
src-tauri/llm-worker/  Separate Rust crate: "voxai-llm-worker" subprocess
src/                 Solid.js + TypeScript frontend
scripts/             Build automation (env-setup, worker prep, build-all)
```

### Process Isolation (Critical Design Constraint)

`whisper-rs-sys` and `llama-cpp-sys-2` both statically embed `ggml.c` with **incompatible ABIs**. They cannot link in the same binary (MSVC LNK2005 errors, runtime crashes with `/FORCE:MULTIPLE`).

**Solution:** The `voxai-llm-worker` runs as a separate subprocess:
- **Main crate** depends on `whisper-rs` (STT) — NO `llama-cpp-2` dependency
- **Worker crate** depends on `llama-cpp-2` (LLM) — NO `whisper-rs` dependency
- Communication: line-based JSON over stdin/stdout pipes
- `LocalLlmBackend` spawns the worker via `tauri-plugin-shell`
- `build.rs` auto-compiles the worker binary into `binaries/`

### Two Build Distributions

- **CPU build** (`cargo tauri build`): No CUDA, portable ~15MB
- **NVIDIA build** (`cargo tauri build --features cuda`): Links CUDA, bundles DLLs ~300MB+
- `cuda` feature flag propagates: `whisper-rs/cuda` (main) + `llama-cpp-2/cuda` (worker)
- CUDA DLLs are delay-loaded — app doesn't crash on machines without NVIDIA GPU
- `WHISPER_NATIVE=OFF` in `.cargo/config.toml` disables AVX-512 for portable binaries (AVX2 baseline)

### Backend Modules (src-tauri/src/)

- **`lib.rs`** — App setup, Tauri command registration, window management
- **`app_state.rs`** — Global `AppState` struct (config, engines, hotkeys, downloads)
- **`commands/`** — 30+ Tauri IPC command handlers (recording, settings, models, engines, etc.)
- **`audio/`** — CPAL capture → Rubato resampling (16kHz mono) → PCM buffer
- **`stt/`** — Whisper engine wrapper + Silero VAD
- **`llm/`** — `LlmBackend` trait with `OllamaBackend` and `LocalLlmBackend` impls
- **`postprocessing/`** — 5-stage text pipeline: capitalize → spacing → reformulate → translate → substitute
- **`models/`** — Model registry (9 Whisper + 5 LLM), HuggingFace downloader with cancellation
- **`config/`** — Schema + JSON persistence to `~/.config/Voxai/`
- **`hotkey/`** — rdev global keyboard listener (push-to-talk + text processing)
- **`injection/`** — Text injection via clipboard + Ctrl+V (enigo)
- **`streaming.rs`** — Real-time transcription with silence detection + VAD

### Frontend (src/)

- **Solid.js 1.9** with **TailwindCSS 4**, bundled by **Vite 6**
- Two windows: `index.html` (settings, 5-tab UI) + `overlay.html` (floating status)
- **`lib/commands.ts`** — Typed wrappers for all Tauri `invoke()` calls
- **`lib/stores.ts`** — Reactive stores synced with backend state
- **`lib/i18n.ts`** — Translation system (EN/FR/ZH) in `lib/translations/`
- **`types/index.ts`** — TypeScript interfaces mirroring Rust types

### Concurrency Patterns

- Use `std::sync::Mutex` (not tokio) — no guards held across `.await`
- `Arc<dyn Trait>`: clone Arc out of the lock, drop guard, then `.await`
- LLM backend: `Arc<RwLock<Option<Arc<dyn LlmBackend>>>>` — clone before await
- Use `tauri::async_runtime::spawn` / `spawn_blocking` (not `tokio::spawn`)
- rdev keyboard listener runs in a dedicated `std::thread`

### Key Crate API Notes

- **whisper-rs 0.15**: `full_n_segments()` → `c_int`. `get_segment(i)` → `Option<WhisperSegment>`. Builder methods return `&mut Self`.
- **cpal 0.17**: `SampleRate` is `type SampleRate = u32` (not a newtype). Use `description().name()` (not deprecated `name()`).
- **llama-cpp-2 0.1** (worker only): `LlamaModel` is Send+Sync. `LlamaBackend::init()` once via `OnceLock`. `clear_kv_cache()` for context reuse.
- **ollama-rs 0.3**: `Ollama::new(host, port)` — host is `impl IntoUrl`, port is `u16`.

### Data Flow

```
Microphone → CPAL → Rubato (16kHz) → Whisper STT → Text Pipeline → Clipboard → Ctrl+V injection
                                                        ↓
                                    capitalize → spacing → LLM reformulate → LLM translate → regex substitute
```

### Config & Data Paths

- Config: `~/.config/Voxai/config.json`
- Models: `%APPDATA%/Voxai/models/` (GGUF files)
- Logs: `%APPDATA%/Voxai/voxai.log` (fern, 10MB rotation, DEBUG for voxai modules)

## Switching Between CPU and CUDA Builds

When switching CMake generators (VS↔Ninja), delete the whisper build cache:
```
rm -rf target/debug/build/whisper-rs-sys-*/
```
