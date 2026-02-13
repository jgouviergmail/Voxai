# Plan : Optimisations CPU fiables pour Voxai

## Contexte

L'audit exhaustif du code a identifie 7 optimisations CPU prouvees, classees par impact reel mesurable. Le probleme le plus critique est que llama.cpp dans le worker subprocess est compile **sans SIMD** (SSE2 baseline uniquement) a cause du stripping de `CARGO_ENCODED_RUSTFLAGS` dans build.rs. Les autres gains viennent du profil release manquant, de la configuration incomplete de llama.cpp, et de micro-optimisations dans le pipeline streaming.

## Changements prevus

### A. Worker SIMD fix (impact: 2-4x LLM inference)
**Fichier:** `src-tauri/build.rs` (ligne ~45)

- Ajouter `cmd.env("RUSTFLAGS", "-C target-cpu=native");` apres le `env_remove("CARGO_ENCODED_RUSTFLAGS")`
- llama-cpp-sys-2 build.rs detectera `target-cpu=native` et activera `GGML_NATIVE=ON`
- Verifie: le format est correct, cargo re-encode en `CARGO_ENCODED_RUSTFLAGS` avec `\x1f` pour les build scripts

### B. Profil release (impact: 10-20% global)
**Fichier:** `src-tauri/Cargo.toml` (ajouter section)

```toml
[profile.release]
lto = "fat"
codegen-units = 1
panic = "abort"
strip = "symbols"
```

- Verifie: `panic = "abort"` safe avec Tauri 2.x + tokio
- Verifie: `lto = "fat"` fonctionne avec cdylib (optimise le code Rust, pas le C/C++)
- S'applique au workspace entier (main + worker via `cargo build --release`)

### C. LlamaContextParams optimise (impact: 15-30% LLM inference)
**Fichier:** `src-tauri/llm-worker/src/main.rs` (lignes 210-211)
**Fichier:** `src-tauri/llm-worker/Cargo.toml` (ajouter dep)

Ajouter `num_cpus = "1"` aux deps du worker, puis configurer:
```rust
let physical_cores = num_cpus::get_physical() as i32;
let ctx_params = LlamaContextParams::default()
    .with_n_ctx(Some(NonZeroU32::new(N_CTX).unwrap()))
    .with_n_threads(physical_cores)
    .with_n_threads_batch(physical_cores)
    .with_type_k(KvCacheType::Q8_0)
    .with_type_v(KvCacheType::Q8_0);
```

- `n_threads` : coeurs physiques (HT nuit aux workloads memory-bound)
- `type_k/type_v` Q8_0 : reduit la bande passante memoire de 75% pour l'attention (default=F32)
- Flash attention : laisser a AUTO (defaut) - llama.cpp decide selon le hardware
- Deplacer `LlamaBatch::new()` hors de `generate()` pour reutilisation (minor)

### D. mimalloc (impact: 5-15%)
**Fichiers:** `src-tauri/Cargo.toml`, `src-tauri/llm-worker/Cargo.toml`, `src-tauri/src/lib.rs`, `src-tauri/llm-worker/src/main.rs`

- Ajouter `mimalloc = "0.1"` aux deux crates
- Ajouter `#[global_allocator] static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;`
- Dans lib.rs : avant `pub fn run()`
- Dans worker main.rs : avant `fn main()`
- Verifie: fonctionne sur MSVC Windows, developpe par Microsoft

### E. Merge VAD + transcription en un seul spawn_blocking (impact: ~5-10ms/segment)
**Fichier:** `src-tauri/src/streaming.rs` (lignes 166-196)

- Fusionner les deux `spawn_blocking` (VAD puis transcription) en un seul
- Elimine `samples_16k.clone()` (~192KB/segment)
- Elimine un dispatch vers le thread pool (~2ms)
- Verifie: pas de deadlock (VAD n'utilise aucun lock, stt_engine lock est acquis apres)
- Verifie: le cas "no speech" est un simple return Ok(None) dans la closure

### F. Supprimer le buffer backup inutile en streaming (impact: memoire)
**Fichier:** `src-tauri/src/audio/capture.rs` (lignes 176-180)

- En mode streaming, le callback audio accumule les donnees dans un `Vec<f32>` derriere un Mutex ET les envoie via le channel mpsc
- Le buffer accumule n'est JAMAIS lu en streaming (stop() return value ignore a lib.rs:281)
- Solution: ajouter un boolean `is_streaming` a AudioCapture, skip `buf.extend_from_slice` quand true
- Verifie: stop() en streaming ignore le CapturedAudio (confirmed lib.rs:281)
- Gain: supprime ~4KB memcpy + mutex lock toutes les 10ms, evite accumulation memoire (115MB/5min)

### G. Resampler : pre-allocation de la sortie (impact: mineur)
**Fichier:** `src-tauri/src/audio/resampler.rs` (ligne 57)

- Remplacer `Vec::new()` par `Vec::with_capacity(mono.len() * TARGET_SAMPLE_RATE / source_rate + chunk_size)`
- Evite 8-9 reallocations dynamiques du Vec de sortie

### H. detect_cpu_count : coeurs physiques (correctness fix)
**Fichiers:** `src-tauri/Cargo.toml` (ajouter dep), `src-tauri/src/commands/gpu.rs` (ligne 12-16)

- Ajouter `num_cpus = "1"` au main crate
- Remplacer `std::thread::available_parallelism()` par `num_cpus::get_physical()`
- Le slider STT threads en frontend affichera le bon nombre de coeurs physiques
- Impact: meilleur default pour Whisper (evite surcharge HT)

## Fichiers modifies (resume)

| Fichier | Changement |
|---|---|
| `src-tauri/Cargo.toml` | `[profile.release]` + deps `mimalloc`, `num_cpus` |
| `src-tauri/build.rs` | RUSTFLAGS pour worker SIMD |
| `src-tauri/src/lib.rs` | `#[global_allocator]` mimalloc |
| `src-tauri/src/commands/gpu.rs` | `num_cpus::get_physical()` |
| `src-tauri/src/streaming.rs` | Merge VAD+transcription |
| `src-tauri/src/audio/capture.rs` | Skip buffer en streaming |
| `src-tauri/src/audio/resampler.rs` | Pre-allocation output |
| `src-tauri/llm-worker/Cargo.toml` | deps `mimalloc`, `num_cpus` |
| `src-tauri/llm-worker/src/main.rs` | `#[global_allocator]` + context params + batch reuse |

## Ce qui n'est PAS modifie (et pourquoi)

- **Injection sleeps** (windows.rs) : les delais sont necessaires pour la synchronisation clipboard Windows. Reduire risque des race conditions dependantes du hardware.
- **Flash attention** : laisse a AUTO (defaut) - llama.cpp decide selon le CPU. Forcer ENABLED sur CPU pourrait etre contre-productif.
- **OpenMP pour whisper-rs** : whisper.cpp utilise deja son propre threading via `set_n_threads()`. OpenMP ajouterait une dep runtime (libomp.dll) pour un gain incertain.
- **Resampler caching** (thread_local) : complexite disproportionnee pour ~2ms/segment.

## Verification

1. `cargo check` dans src-tauri/ (0 erreurs)
2. `cargo check` dans src-tauri/llm-worker/ (0 erreurs)
3. `cargo test` dans src-tauri/ (61 tests passent)
4. Build release CPU : `cargo tauri build` (verifie compilation + LTO)
5. Test fonctionnel : lancer l'app, enregistrer un segment, verifier que la transcription et la reformulation fonctionnent
