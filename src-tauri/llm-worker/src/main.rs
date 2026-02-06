//! Standalone LLM worker process.
//!
//! This binary runs in a separate process from the main Voxai app to avoid
//! ggml symbol collisions between whisper-rs-sys and llama-cpp-sys-2.
//!
//! Protocol (line-based JSON over stdin/stdout):
//!   Request:  {"prompt": "...", "system": "..."}
//!   Response: {"text": "..."} or {"error": "..."}
//!   Ping:     {"command": "ping"}  →  {"status": "ok"}
//!   Exits on stdin EOF.

use std::io::{self, BufRead, Write};
use std::num::NonZeroU32;
use std::path::PathBuf;

#[allow(deprecated)]
use llama_cpp_2::model::{AddBos, LlamaModel, Special};
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::sampling::LlamaSampler;
use serde::{Deserialize, Serialize};

const MAX_TOKENS: usize = 512;
const N_CTX: u32 = 2048;

#[derive(Deserialize)]
struct Request {
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    system: Option<String>,
}

#[derive(Serialize)]
struct Response {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
}

impl Response {
    fn ok(text: String) -> Self {
        Self { text: Some(text), error: None, status: None }
    }
    fn err(msg: String) -> Self {
        Self { text: None, error: Some(msg), status: None }
    }
    fn pong() -> Self {
        Self { text: None, error: None, status: Some("ok".into()) }
    }
}

fn format_chat(system: &str, prompt: &str, template: &str) -> String {
    match template {
        "qwen2" | "chatml" => format!(
            "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
            system, prompt
        ),
        "gemma" => format!(
            "<start_of_turn>user\n{}\n\n{}<end_of_turn>\n<start_of_turn>model\n",
            system, prompt
        ),
        // "phi3" and default
        _ => format!(
            "<|system|>\n{}<|end|>\n<|user|>\n{}<|end|>\n<|assistant|>\n",
            system, prompt
        ),
    }
}

fn generate(model: &LlamaModel, backend: &LlamaBackend, prompt: &str, system: &str, chat_template: &str) -> Result<String, String> {
    let combined = format_chat(system, prompt, chat_template);

    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(Some(NonZeroU32::new(N_CTX).unwrap()));

    let mut ctx = model
        .new_context(backend, ctx_params)
        .map_err(|e| format!("Failed to create context: {}", e))?;

    let tokens = model
        .str_to_token(&combined, AddBos::Always)
        .map_err(|e| format!("Tokenization error: {}", e))?;

    if tokens.len() >= N_CTX as usize {
        return Err("Prompt too long for context window".into());
    }

    let mut batch = LlamaBatch::new(N_CTX as usize, 1);

    for (i, &token) in tokens.iter().enumerate() {
        let is_last = i == tokens.len() - 1;
        batch
            .add(token, i as i32, &[0], is_last)
            .map_err(|e| format!("Batch add error: {}", e))?;
    }

    ctx.decode(&mut batch)
        .map_err(|e| format!("Decode error: {}", e))?;

    let mut sampler = LlamaSampler::chain_simple(vec![
        LlamaSampler::temp(0.3),
        LlamaSampler::top_k(40),
        LlamaSampler::top_p(0.95, 1),
        LlamaSampler::greedy(),
    ]);

    let mut output = String::new();
    let mut n_decoded = tokens.len();

    for _ in 0..MAX_TOKENS {
        let token = sampler.sample(&ctx, -1);

        if model.is_eog_token(token) {
            break;
        }

        #[allow(deprecated)]
        let piece = model
            .token_to_str(token, Special::Tokenize)
            .map_err(|e| format!("Token decode error: {}", e))?;
        output.push_str(&piece);

        batch.clear();
        batch
            .add(token, n_decoded as i32, &[0], true)
            .map_err(|e| format!("Batch add error: {}", e))?;

        ctx.decode(&mut batch)
            .map_err(|e| format!("Decode error: {}", e))?;

        n_decoded += 1;
    }

    Ok(output.trim().to_string())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: voxai-llm-worker <model-path> [chat-template] [--gpu-layers N]");
        std::process::exit(1);
    }

    let model_path = PathBuf::from(&args[1]);
    if !model_path.exists() {
        eprintln!("Model file not found: {}", model_path.display());
        std::process::exit(1);
    }

    // Parse chat template (arg 2, default "phi3")
    let chat_template = args.get(2)
        .filter(|s| !s.starts_with("--"))
        .map(|s| s.as_str())
        .unwrap_or("phi3");

    // Parse --gpu-layers flag
    let gpu_layers: u32 = args.windows(2)
        .find(|w| w[0] == "--gpu-layers")
        .and_then(|w| w[1].parse().ok())
        .unwrap_or(0);

    let backend = LlamaBackend::init().expect("Failed to init llama backend");
    let mut model_params = LlamaModelParams::default();
    if gpu_layers > 0 {
        model_params = model_params.with_n_gpu_layers(gpu_layers);
    }
    let model = LlamaModel::load_from_file(&backend, &model_path, &model_params)
        .expect("Failed to load model");

    // Signal readiness
    let mut stdout = io::stdout().lock();
    let _ = writeln!(stdout, "{}", serde_json::to_string(&Response::pong()).unwrap());
    let _ = stdout.flush();
    drop(stdout);

    // Main loop: read requests from stdin
    let stdin = io::stdin().lock();
    for line in stdin.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break, // stdin closed
        };

        if line.trim().is_empty() {
            continue;
        }

        let req: Request = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = Response::err(format!("Invalid JSON: {}", e));
                let mut stdout = io::stdout().lock();
                let _ = writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap());
                let _ = stdout.flush();
                continue;
            }
        };

        let resp = if req.command.as_deref() == Some("ping") {
            Response::pong()
        } else if let (Some(prompt), Some(system)) = (req.prompt.as_ref(), req.system.as_ref()) {
            match generate(&model, &backend, prompt, system, chat_template) {
                Ok(text) => Response::ok(text),
                Err(e) => Response::err(e),
            }
        } else {
            Response::err("Missing 'prompt' and 'system' fields".into())
        };

        let mut stdout = io::stdout().lock();
        let _ = writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap());
        let _ = stdout.flush();
    }
}
