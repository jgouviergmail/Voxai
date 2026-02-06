// Mirror of Rust structs for type-safe IPC

export interface AppConfig {
  general: GeneralConfig;
  stt: SttConfig;
  postprocessing: PostProcessingConfig;
  llm: LlmConfig;
}

export interface GeneralConfig {
  hotkey: HotkeyConfig;
  input_device: string | null;
  auto_start: boolean;
  auto_enter: boolean;
  clipboard_restore: boolean;
  language: string;
  gpu_acceleration: boolean;
  ui_language: string;
}

export interface HotkeyConfig {
  key: string;
  modifiers: string[];
}

export interface SttConfig {
  active_engine: string;
  active_model: string | null;
}

export interface PostProcessingConfig {
  auto_capitalize: boolean;
  smart_spacing: boolean;
  reformulation: ReformulationConfig;
  translation: TranslationConfig;
  substitutions: SubstitutionRule[];
  custom_prompts: CustomPrompt[];
  prompt_overrides: Record<string, PromptOverride>;
}

export interface CustomPrompt {
  id: string;
  name: string;
  system: string;
  instruction: string;
}

export interface PromptOverride {
  system: string | null;
  instruction: string | null;
}

export interface PromptPreview {
  system: string;
  instruction: string;
  is_modified: boolean;
}

export interface ReformulationConfig {
  enabled: boolean;
  style: ReformulationStyle;
}

export type ReformulationStyle =
  | "Cleaned"
  | "Professional"
  | "Casual"
  | "Concise"
  | "Simplified"
  | "Structured"
  | { Custom: string };

export interface TranslationConfig {
  enabled: boolean;
  target_language: string;
}

export interface SubstitutionRule {
  from: string;
  to: string;
  case_sensitive: boolean;
}

export interface LlmConfig {
  active_backend: LlmBackendType;
  ollama: OllamaConfig;
  local: LocalLlmConfig;
}

export interface LocalLlmConfig {
  model_id: string | null;
}

export type LlmBackendType = "Ollama" | "Local" | "None";

export interface OllamaConfig {
  host: string;
  port: number;
  model: string;
}

export interface RecordingState {
  kind: "Idle" | "Recording" | "Processing";
  stage?: "Transcribing" | "PostProcessing" | "Injecting";
}

export interface HistoryEntry {
  id: string;
  raw_text: string;
  final_text: string;
  engine: string;
  duration_ms: number;
  created_at: string;
}

export interface DownloadProgress {
  model_id: string;
  downloaded_bytes: number;
  total_bytes: number;
  percent: number;
}

export interface ModelInfo {
  id: string;
  name: string;
  size_mb: number;
  description: string;
  downloaded: boolean;
}

export interface InputDeviceInfo {
  name: string;
  is_default: boolean;
}

export interface EngineInfo {
  id: string;
  name: string;
  active: boolean;
  loaded: boolean;
  models: EngineModelInfo[];
}

export interface EngineModelInfo {
  id: string;
  name: string;
  size_mb: number;
  description: string;
  downloaded: boolean;
  active: boolean;
}

export interface LlmStatus {
  configured: boolean;
  available: boolean;
  backend_name: string;
}

export interface PipelineTestResult {
  input: string;
  after_capitalize: string;
  after_spacing: string;
  after_reformulation: string | null;
  after_translation: string | null;
  after_substitutions: string;
  final_text: string;
}

export interface LanguageInfo {
  code: string;
  name: string;
}

export interface NvidiaInfo {
  detected: boolean;
  gpu_name: string;
  driver_version: string;
  vram_mb: number;
}
