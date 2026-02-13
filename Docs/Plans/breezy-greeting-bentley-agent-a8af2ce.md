# Research Report: Tauri 2.x Build Optimizations for Voxai

## Executive Summary

Research completed on 5 implementation concerns for optimizing a Tauri 2.x Windows application with whisper-rs and llama-cpp-2. Key findings:

1. **`panic = "abort"`**: SAFE to use with Tauri 2.x on Windows
2. **`lto = "fat"`**: Works with cdylib, but does NOT cross Rust-C boundary for whisper.cpp/llama.cpp
3. **CARGO_ENCODED_RUSTFLAGS**: Use `\x1f` separator; RUSTFLAGS deprecated since Rust 1.55
4. **mimalloc**: Works on Windows MSVC; can be added to both main and worker crates
5. **Physical cores**: Use `num_cpus::get_physical()` for llama.cpp threads (SMT hurts performance)

---

## 1. Does `panic = "abort"` work safely with Tauri 2.x?

### Answer: YES - Safe to use

**Tokio Runtime Compatibility:**
- Tokio's default behavior: panics in spawned tasks are forwarded to JoinHandle; other tasks continue running
- Tokio supports `UnhandledPanic::ShutdownRuntime` mode for immediate shutdown on panic
- Tokio does NOT require unwinding for cleanup - it handles both panic modes gracefully
- Runtime shutdown happens via the destructor of the Runtime object

**Tauri WebView (WRY) on Windows:**
- No evidence of catch_unwind dependency in Tauri core or WRY
- Known panic issues in Tauri/WRY are bugs (e.g., webview creation failures), not architectural requirements
- Tauri 2.x officially supports MSVC Windows builds

**Recommendation:**
```toml
[profile.release]
panic = "abort"
```

**Caveat:** If you use `std::panic::catch_unwind` anywhere in your code (e.g., for plugin isolation), this will break. Current Voxai codebase does NOT use catch_unwind.

---

## 2. Does `lto = "fat"` work with `crate-type = ["lib", "cdylib", "staticlib"]`?

### Answer: YES for cdylib, but NO cross-language LTO by default

**Fat LTO with cdylib:**
- Confirmed: LTO works with cdylib targets (Tauri on Windows produces a cdylib)
- Fat LTO optimizes all Rust code at link time
- No known issues with whisper-rs-sys or llama-cpp-sys-2 when LTO is applied

**Cross-Language LTO (Rust ↔ C/C++):**
- Fat LTO does NOT cross the Rust-C boundary by default
- whisper.cpp and llama.cpp (C/C++ static libs) are linked as opaque binaries
- To enable cross-language LTO, you need:
  1. `-C linker-plugin-lto` (defers LTO to linker)
  2. Compile C/C++ with `-flto=thin`
  3. Use LLVM-based linker (lld)
  4. rustc and clang MUST use same LLVM version (ideally same major version)

**Current Voxai Build:**
- whisper-rs-sys and llama-cpp-sys-2 build.rs scripts compile C/C++ without `-flto`
- Fat LTO will only optimize Rust code (still beneficial!)

**Recommendation:**
```toml
[profile.release]
lto = "fat"  # Optimizes all Rust code; C/C++ libs remain opaque
```

For cross-language LTO, would need to patch build.rs in whisper-rs-sys and llama-cpp-sys-2 (complex, risky).

---

## 3. CARGO_ENCODED_RUSTFLAGS format

### Answer: Use `\x1f` separator; RUSTFLAGS deprecated in build.rs

**Key Facts:**
- Since Rust 1.55: `RUSTFLAGS` is removed from build.rs environment
- `CARGO_ENCODED_RUSTFLAGS` uses ASCII Unit Separator (`\x1f` = 0x1F) between flags
- Example: `"-Ctarget-cpu=native\x1f-Copt-level=3"`
- llama-cpp-sys-2 build.rs (line 538) correctly splits by `\x1f`

**How to Set in Code:**
```rust
// WRONG - will not work in build.rs:
cmd.env("RUSTFLAGS", "-Ctarget-cpu=native");

// CORRECT - for build.rs scripts:
cmd.env("CARGO_ENCODED_RUSTFLAGS", "-Ctarget-cpu=native\x1f-Copt-level=3");
```

**Purpose:**
- Synthesizes flags from multiple sources:
  - RUSTFLAGS environment variable
  - Cargo config: `target.<triple>.rustflags`, `build.rustflags`
  - Project-specific `.cargo/config.toml`

**Recommendation:**
If passing RUSTFLAGS to a build.rs subprocess (e.g., llama-cpp-2 worker build), use `CARGO_ENCODED_RUSTFLAGS` with `\x1f` separator.

---

## 4. mimalloc on Windows MSVC

### Answer: YES - Works out of the box; can be added to both crates

**Compatibility:**
- `mimalloc` crate v0.1.x works on Windows MSVC
- Latest mimalloc versions: v3.2.8 (RC3, 2026-02-03), v2.2.7 (stable, 2026-01-15)
- v3 features: faster Windows TLS access, improved calloc/aligned allocations
- Requires C compiler (MSVC) for building

**Usage:**
```toml
[dependencies]
mimalloc = { version = "0.1", default-features = false }
```

```rust
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
```

**No Known Tauri Conflicts:**
- No reports of mimalloc issues with Tauri 2.x
- Can be added to both main crate AND worker subprocess independently
- Each process gets its own allocator instance

**Recommendation:**
Add to both `src-tauri/Cargo.toml` and `src-tauri/llm-worker/Cargo.toml`:
```toml
mimalloc = "0.1"
```

Memory-intensive workloads (whisper transcription, llama inference) should benefit from mimalloc's performance.

---

## 5. Physical vs Logical CPU Cores on Windows

### Answer: Use `num_cpus::get_physical()` for llama.cpp threads

**Detection:**
```rust
// Logical cores (includes hyperthreading):
let logical = std::thread::available_parallelism().unwrap().get();

// Physical cores (CORRECT for llama.cpp):
let physical = num_cpus::get_physical();
```

**num_cpus Crate:**
- `num_cpus::get_physical()` is supported on Windows, Linux, macOS
- Returns physical cores (on unsupported platforms, falls back to logical cores)
- Current Voxai code uses `available_parallelism()` → **INCORRECT for llama.cpp**

**llama.cpp Performance Data:**
- **SMT/Hyperthreading HURTS inference performance**
- Best practice: 1 thread per physical core
- Using logical cores (2x physical) causes noticeable performance drop
- For Intel P/E cores: use P-cores only, reserve 1 P-core for main thread

**Memory Bandwidth Bottleneck:**
- CPU inference is memory-bandwidth-limited (not compute-limited)
- SMT doubles threads but does NOT double memory bandwidth
- Result: threads compete for memory → worse performance

**Current Voxai Bug:**
File: `d:\Developpement\Voxai\src-tauri\src\commands\gpu.rs:15`
```rust
pub fn detect_cpu_count() -> u32 {
    std::thread::available_parallelism()  // ← WRONG: returns logical cores
        .map(|n| n.get() as u32)
        .unwrap_or(4)
}
```

**Recommended Fix:**
```toml
# Add to src-tauri/Cargo.toml
num_cpus = "1.16"
```

```rust
pub fn detect_cpu_count() -> u32 {
    num_cpus::get_physical() as u32
}
```

**For whisper-rs:**
Whisper can benefit from hyperthreading (different workload than llama.cpp), but testing needed. Consider:
- Default: physical cores for both
- Advanced: separate sliders for Whisper vs LLM threads

---

## Summary Recommendations

### Cargo.toml Changes

**src-tauri/Cargo.toml:**
```toml
[dependencies]
num_cpus = "1.16"
mimalloc = "0.1"

[profile.release]
panic = "abort"
lto = "fat"
codegen-units = 1
opt-level = 3
```

**src-tauri/llm-worker/Cargo.toml:**
```toml
[dependencies]
mimalloc = "0.1"

[profile.release]
panic = "abort"
lto = "fat"
codegen-units = 1
opt-level = 3
```

### Code Changes

1. Add `#[global_allocator]` in both `src-tauri/src/main.rs` and `src-tauri/llm-worker/src/main.rs`
2. Fix `detect_cpu_count()` to use `num_cpus::get_physical()`
3. (Optional) Pass `-Ctarget-cpu=native` via `.cargo/config.toml`:
   ```toml
   [build]
   rustflags = ["-Ctarget-cpu=native"]
   ```

### Build Size Impact

- `panic = "abort"`: ~5-10% smaller binary (no unwinding tables)
- `lto = "fat"`: ~10-20% smaller (dead code elimination, inlining)
- Total: Expect 15-30% size reduction for release build

---

## Sources

### Panic/Tokio:
- [TokioHandle Documentation](https://docs.rs/tauri/latest/tauri/async_runtime/struct.TokioHandle.html)
- [Tauri Async Runtime](https://docs.rs/tauri/latest/src/tauri/async_runtime.rs.html)
- [Tokio UnhandledPanic](https://docs.rs/tokio/latest/tokio/runtime/enum.UnhandledPanic.html)
- [Issue #10289: Tokio spawn panic](https://github.com/tauri-apps/tauri/issues/10289)
- [Rust Panic Abort Discussion](https://users.rust-lang.org/t/is-there-a-way-to-abort-process-on-panic-in-tokio/61561)

### LTO:
- [LTO Error Discussion](https://users.rust-lang.org/t/error-lto-can-only-be-run-for-executables-cdylibs-and-static-library-outputs/73369)
- [RFC 1510 cdylib](https://rust-lang.github.io/rfcs/1510-cdylib.html)
- [Cross-Language LTO Rustc Book](https://doc.rust-lang.org/rustc/linker-plugin-lto.html)
- [LLVM Blog: Cross-Language LTO](https://blog.llvm.org/2019/09/closing-gap-cross-language-lto-between.html)

### CARGO_ENCODED_RUSTFLAGS:
- [Cargo Environment Variables](https://doc.rust-lang.org/cargo/reference/environment-variables.html)
- [Issue #10555: Improve Documentation](https://github.com/rust-lang/cargo/issues/10555)
- [rustflags Crate](https://docs.rs/rustflags)

### mimalloc:
- [mimalloc Crate](https://crates.io/crates/mimalloc)
- [mimalloc GitHub](https://github.com/microsoft/mimalloc)
- [mimalloc-rust Wrapper](https://github.com/purpleprotocol/mimalloc_rust)

### CPU Cores:
- [num_cpus::get_physical](https://docs.rs/num_cpus/latest/num_cpus/fn.get_physical.html)
- [num_cpus Crate](https://docs.rs/num_cpus/)
- [llama.cpp Discussion #3167](https://github.com/ggml-org/llama.cpp/discussions/3167)
- [Discussion #572: Performance Cores](https://github.com/ggml-org/llama.cpp/discussions/572)
- [LLaMA CPU Performance Blog](https://justine.lol/matmul/)
- [CPU-Only LLM Inference](https://nikro.me/articles/professional/cpu-only-llm-inference/)
