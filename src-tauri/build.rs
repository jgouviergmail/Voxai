use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    build_llm_worker();
    tauri_build::build();
}

/// CUDA runtime DLLs required by whisper-rs (cuBLAS) and llama-cpp-2 (cuBLAS).
/// These are load-time dependencies (in the PE import table) so they MUST be
/// next to the executable — setting PATH at runtime is too late.
const CUDA_DLLS: &[&str] = &["cublas64_12.dll", "cublasLt64_12.dll", "cudart64_12.dll"];

/// Build the `voxai-llm-worker` binary with matching feature flags.
///
/// When the main crate is compiled with `--features cuda`, this ensures
/// the worker is also compiled with `cuda` (enabling `llama-cpp-2/cuda`).
/// Uses a separate target directory to avoid cargo lock conflicts.
fn build_llm_worker() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let profile = env::var("PROFILE").unwrap();
    let cuda_enabled = env::var("CARGO_FEATURE_CUDA").is_ok();

    let worker_manifest = manifest_dir.join("llm-worker").join("Cargo.toml");
    // Separate target dir avoids deadlock with the outer cargo holding its lock
    let worker_target = manifest_dir.join("target").join("llm-worker-build");

    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut cmd = Command::new(&cargo);
    cmd.arg("build")
        .arg("--manifest-path")
        .arg(&worker_manifest)
        .arg("--target-dir")
        .arg(&worker_target);

    // Sanitize env vars inherited from the outer build script to prevent
    // interference with the inner cargo's own build scripts (e.g. llama-cpp-sys-2).
    // Keep CARGO (binary path), CARGO_MAKEFLAGS (jobserver), PATH, and user
    // env vars like LIBCLANG_PATH, CUDA_PATH which are needed for compilation.
    cmd.env_remove("OUT_DIR")
        .env_remove("CARGO_MANIFEST_DIR")
        .env_remove("CARGO_MANIFEST_PATH")
        .env_remove("PROFILE")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("OPT_LEVEL")
        .env_remove("DEBUG")
        .env_remove("NUM_JOBS")
        .env_remove("HOST")
        .env_remove("TARGET")
        // Detach from the outer cargo's jobserver to avoid deadlock:
        // the outer cargo holds tokens waiting for this build script,
        // so the inner cargo would starve if it shared the same jobserver.
        .env_remove("CARGO_MAKEFLAGS")
        .env_remove("MAKEFLAGS");

    for (key, _) in env::vars() {
        if key.starts_with("CARGO_FEATURE_")
            || key.starts_with("CARGO_CFG_")
            || key.starts_with("DEP_")
        {
            cmd.env_remove(&key);
        }
    }

    if profile == "release" {
        cmd.arg("--release");
    }

    if cuda_enabled {
        cmd.args(["--features", "cuda"]);
    }

    println!(
        "cargo:warning=Building voxai-llm-worker (cuda={})",
        cuda_enabled
    );

    // In a build script, stdout is a pipe to cargo (for `cargo:` directives).
    // Redirect inner cargo's stdout to null; progress output goes via stderr.
    let status = cmd
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit())
        .status()
        .expect("Failed to execute cargo build for voxai-llm-worker");

    if !status.success() {
        panic!(
            "voxai-llm-worker build failed (exit code: {:?})",
            status.code()
        );
    }

    // Copy the built binary to binaries/
    let bin_name = if cfg!(windows) {
        "voxai-llm-worker.exe"
    } else {
        "voxai-llm-worker"
    };
    let profile_dir = if profile == "release" { "release" } else { "debug" };
    let built_path = worker_target.join(profile_dir).join(bin_name);

    let dest_dir = manifest_dir.join("binaries");
    std::fs::create_dir_all(&dest_dir).expect("Failed to create binaries/ directory");
    let dest_path = dest_dir.join(bin_name);

    std::fs::copy(&built_path, &dest_path).unwrap_or_else(|e| {
        panic!(
            "Failed to copy worker binary from {} to {}: {}",
            built_path.display(),
            dest_path.display(),
            e
        )
    });

    // Bundle or clean CUDA DLLs depending on feature flag
    if cuda_enabled {
        bundle_cuda_dlls(&dest_dir);
    } else {
        clean_cuda_dlls(&dest_dir);
    }

    // Watch the entire src directory (recursive since Rust 1.50) + Cargo.toml
    println!("cargo:rerun-if-changed=llm-worker/src");
    println!("cargo:rerun-if-changed=llm-worker/Cargo.toml");
    // Re-run when cuda feature flag changes (CPU↔CUDA switch without source changes)
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_CUDA");
}

/// Copy CUDA runtime DLLs from the local CUDA Toolkit into `binaries/`
/// so Tauri bundles them next to the executable in the NSIS installer.
fn bundle_cuda_dlls(dest_dir: &Path) {
    let cuda_path = env::var("CUDA_PATH").unwrap_or_else(|_| {
        // Auto-detect on Windows
        let base = r"C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA";
        if let Ok(entries) = std::fs::read_dir(base) {
            let mut versions: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_name().to_string_lossy().starts_with('v'))
                .collect();
            versions.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
            if let Some(latest) = versions.first() {
                return latest.path().to_string_lossy().into_owned();
            }
        }
        panic!("CUDA_PATH not set and CUDA Toolkit not found. Cannot build CUDA variant.");
    });

    let cuda_bin = PathBuf::from(&cuda_path).join("bin");

    for dll in CUDA_DLLS {
        let src = cuda_bin.join(dll);
        let dest = dest_dir.join(dll);
        if src.exists() {
            std::fs::copy(&src, &dest).unwrap_or_else(|e| {
                panic!("Failed to copy {}: {}", dll, e)
            });
            println!("cargo:warning=Bundled CUDA DLL: {} ({:.1} MB)",
                dll,
                std::fs::metadata(&dest).map(|m| m.len() as f64 / 1_048_576.0).unwrap_or(0.0));
        } else {
            panic!("Required CUDA DLL not found: {}", src.display());
        }
    }
}

/// Remove any CUDA DLLs from `binaries/` (CPU build must not ship them).
fn clean_cuda_dlls(dest_dir: &Path) {
    for dll in CUDA_DLLS {
        let path = dest_dir.join(dll);
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }
    }
}
