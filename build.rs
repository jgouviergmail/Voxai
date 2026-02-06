fn main() {
    // Ensure the worker binary placeholder exists so tauri_build resource
    // validation passes during `cargo check` and `cargo tauri dev`.
    // The real binary is built by `npm run build:worker` before release builds.
    let worker = std::path::Path::new("binaries/voxai-llm-worker.exe");
    if !worker.exists() {
        let _ = std::fs::create_dir_all("binaries");
        let _ = std::fs::write(worker, b"placeholder");
    }

    tauri_build::build();
}
