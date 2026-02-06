use serde::Serialize;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize)]
pub struct NvidiaInfo {
    pub detected: bool,
    pub gpu_name: String,
    pub driver_version: String,
    pub vram_mb: u64,
}

#[tauri::command]
pub fn detect_nvidia() -> Result<NvidiaInfo, AppError> {
    let output = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,driver_version,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let line = stdout.trim();
            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if parts.len() >= 3 {
                let vram: u64 = parts[2].parse().unwrap_or(0);
                Ok(NvidiaInfo {
                    detected: true,
                    gpu_name: parts[0].to_string(),
                    driver_version: parts[1].to_string(),
                    vram_mb: vram,
                })
            } else {
                Ok(NvidiaInfo {
                    detected: false,
                    gpu_name: String::new(),
                    driver_version: String::new(),
                    vram_mb: 0,
                })
            }
        }
        _ => Ok(NvidiaInfo {
            detected: false,
            gpu_name: String::new(),
            driver_version: String::new(),
            vram_mb: 0,
        }),
    }
}
