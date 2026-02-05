# Voxai

Voice-to-text dictation application for Windows. Press a hotkey, speak, and your words are transcribed, post-processed, and injected into the focused application.

## Features

- **Push-to-talk** — Global hotkey (default: `Ctrl+Shift+Space`) with key release detection
- **Whisper STT** — Local speech-to-text using whisper.cpp (`.gguf` models), multi-language (fr, en, es, de, it, pt, ja, zh)
- **Post-processing pipeline** — Automatic capitalization, smart spacing, LLM-based reformulation (7 styles), translation, and custom substitutions
- **LLM integration** — Ollama backend for reformulation and translation (configurable model, default: Mistral)
- **Model management** — Download and manage Whisper models directly from HuggingFace Hub with progress tracking
- **Text injection** — Pastes transcribed text into the focused application with optional auto-enter and clipboard restore
- **System tray** — Runs in the background with status icon (Idle/Recording/Processing) and context menu
- **Settings UI** — 5-tab interface for full configuration (General, Engines, Post-Processing, Substitutions, History)
- **Transcription history** — Stores raw and processed text with timestamps

## Tech Stack

| Layer | Technology |
|-------|------------|
| Frontend | Solid.js, TailwindCSS, TypeScript |
| Backend | Rust (2021 edition) |
| Desktop | Tauri 2 |
| Audio | cpal + rubato (resampling to 16kHz mono) |
| STT | whisper-rs (whisper.cpp bindings) |
| LLM | ollama-rs |
| Hotkey | rdev (global keyboard hook) |
| Injection | enigo + arboard (clipboard) |
| Models | hf-hub (HuggingFace downloads) |

## Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://rustup.rs/) (stable)
- [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (MSVC, CMake)
- A `libclang.dll` available via `LIBCLANG_PATH`
- [Ollama](https://ollama.com/) (optional, for reformulation/translation features)

## Getting Started

```bash
# Install frontend dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

On first launch, download a Whisper model from the **Engines** tab before recording.

## Project Structure

```
├── src/                        # Frontend (Solid.js + TypeScript)
│   ├── components/
│   │   ├── settings/           # 5 tab components (General, Engines, PostProcessing, ...)
│   │   ├── layout/             # PageShell, TabBar
│   │   └── ui/                 # Button, Input, Select, Toggle, ProgressBar
│   ├── lib/                    # IPC commands, events, stores
│   └── types/                  # TypeScript interfaces
│
├── src-tauri/                  # Backend (Rust + Tauri)
│   └── src/
│       ├── audio/              # Audio capture & resampling
│       ├── stt/                # Whisper engine
│       ├── llm/                # Ollama backend & prompt templates
│       ├── models/             # Model registry, cache, downloader
│       ├── postprocessing/     # Pipeline: capitalize → spacing → reformulate → translate → substitute
│       ├── injection/          # Text injection (Windows)
│       ├── hotkey/             # Global keyboard hook (rdev)
│       ├── config/             # Settings persistence (~/.config/Voxai/config.json)
│       ├── history/            # Transcription history storage
│       ├── commands/           # Tauri IPC command handlers
│       └── tray/               # System tray icon & menu
│
├── package.json
├── vite.config.ts
└── src-tauri/Cargo.toml
```

## Configuration

Settings are stored in `~/.config/Voxai/config.json` and can be edited from the Settings UI:

- **General** — Hotkey, language, input device, auto-enter, clipboard restore
- **Engines** — STT model selection and download
- **Post-Processing** — Toggle capitalization, spacing, reformulation (Cleaned/Professional/Casual/Concise/Simplified/Structured/Custom), translation
- **Substitutions** — Custom find/replace rules applied after all other processing
- **History** — View past transcriptions (raw vs. final text)

## License

All rights reserved.
