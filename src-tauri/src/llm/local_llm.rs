use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Serialize)]
struct WorkerRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

#[derive(Deserialize)]
struct WorkerResponse {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    status: Option<String>,
}

struct WorkerHandle {
    child: Child,
    reader: BufReader<ChildStdout>,
    writer: ChildStdin,
}

impl WorkerHandle {
    fn send_and_recv(&mut self, req: &WorkerRequest) -> Result<WorkerResponse, AppError> {
        let json = serde_json::to_string(req)
            .map_err(|e| AppError::Llm(format!("JSON serialize error: {}", e)))?;

        writeln!(self.writer, "{}", json)
            .map_err(|e| AppError::Llm(format!("Failed to write to worker stdin: {}", e)))?;
        self.writer
            .flush()
            .map_err(|e| AppError::Llm(format!("Failed to flush worker stdin: {}", e)))?;

        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .map_err(|e| AppError::Llm(format!("Failed to read from worker stdout: {}", e)))?;

        if line.is_empty() {
            return Err(AppError::Llm("Worker process closed stdout".into()));
        }

        serde_json::from_str(&line)
            .map_err(|e| AppError::Llm(format!("Invalid JSON from worker: {}", e)))
    }
}

pub struct LocalLlmBackend {
    model_name: String,
    worker: Arc<Mutex<WorkerHandle>>,
}

impl LocalLlmBackend {
    pub fn new(model_path: &Path, model_name: String) -> Result<Self, AppError> {
        let worker_bin = find_worker_binary()?;

        log::info!(
            "Spawning LLM worker: {} with model {}",
            worker_bin.display(),
            model_path.display()
        );

        let mut child = Command::new(&worker_bin)
            .arg(model_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| AppError::Llm(format!("Failed to spawn LLM worker: {}", e)))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AppError::Llm("Worker stdout not captured".into()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| AppError::Llm("Worker stdin not captured".into()))?;

        let mut reader = BufReader::new(stdout);

        // Wait for the readiness signal from the worker
        let mut ready_line = String::new();
        reader
            .read_line(&mut ready_line)
            .map_err(|e| AppError::Llm(format!("Failed to read worker readiness: {}", e)))?;

        if ready_line.is_empty() {
            // Worker exited before sending readiness — get exit status for diagnostics
            let status = child.wait().ok();
            return Err(AppError::Llm(format!(
                "LLM worker exited before readiness signal (exit: {:?})",
                status
            )));
        }

        let ready: WorkerResponse = serde_json::from_str(&ready_line)
            .map_err(|e| AppError::Llm(format!("Invalid readiness JSON: {}", e)))?;

        if ready.status.as_deref() != Some("ok") {
            return Err(AppError::Llm(format!(
                "Worker readiness failed: {:?}",
                ready.error
            )));
        }

        log::info!("LLM worker ready: {}", model_name);

        Ok(Self {
            model_name,
            worker: Arc::new(Mutex::new(WorkerHandle {
                child,
                reader,
                writer: stdin,
            })),
        })
    }
}

impl Drop for LocalLlmBackend {
    fn drop(&mut self) {
        if let Ok(mut handle) = self.worker.lock() {
            // Kill the worker and wait for it to exit
            let _ = handle.child.kill();
            let _ = handle.child.wait();
        }
    }
}

#[async_trait]
impl super::LlmBackend for LocalLlmBackend {
    fn id(&self) -> &str {
        "local"
    }

    fn name(&self) -> &str {
        &self.model_name
    }

    async fn is_available(&self) -> bool {
        // Check if the worker process is still alive
        if let Ok(mut handle) = self.worker.lock() {
            match handle.child.try_wait() {
                Ok(None) => true,  // still running
                _ => false,        // exited or error
            }
        } else {
            false
        }
    }

    async fn generate(&self, prompt: &str, system: &str) -> Result<String, AppError> {
        let worker = Arc::clone(&self.worker);
        let prompt = prompt.to_string();
        let system = system.to_string();

        // Worker I/O is blocking (read_line waits for LLM inference) — offload to blocking pool
        tauri::async_runtime::spawn_blocking(move || {
            let req = WorkerRequest {
                command: None,
                prompt: Some(prompt),
                system: Some(system),
            };

            let mut handle = worker.lock().map_err(|e| {
                AppError::Llm(format!("Worker mutex poisoned: {}", e))
            })?;

            let resp = handle.send_and_recv(&req)?;

            if let Some(err) = resp.error {
                return Err(AppError::Llm(err));
            }

            resp.text
                .ok_or_else(|| AppError::Llm("Worker returned empty response".into()))
        })
        .await
        .map_err(|e| AppError::Internal(format!("Task join error: {}", e)))?
    }
}

/// Locate the `voxai-llm-worker` binary next to the main executable.
fn find_worker_binary() -> Result<PathBuf, AppError> {
    let exe = std::env::current_exe()
        .map_err(|e| AppError::Llm(format!("Cannot determine exe path: {}", e)))?;

    let dir = exe
        .parent()
        .ok_or_else(|| AppError::Llm("Cannot determine exe directory".into()))?;

    // On Windows the binary is voxai-llm-worker.exe
    let name = if cfg!(windows) {
        "voxai-llm-worker.exe"
    } else {
        "voxai-llm-worker"
    };

    let worker_path = dir.join(name);

    if worker_path.exists() {
        return Ok(worker_path);
    }

    // During development with `cargo tauri dev`, the binary might be in the
    // same target directory (debug/release)
    Err(AppError::Llm(format!(
        "LLM worker binary not found at {}",
        worker_path.display()
    )))
}
