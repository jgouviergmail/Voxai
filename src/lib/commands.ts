import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  EngineInfo,
  HistoryEntry,
  InputDeviceInfo,
  LanguageInfo,
  LlmStatus,
  ModelInfo,
  NvidiaInfo,
  PipelineTestResult,
  PromptPreview,
  RecordingState,
  SubstitutionRule,
} from "../types";

// Recording
export const getRecordingState = () =>
  invoke<RecordingState>("get_recording_state");
export const startRecording = () => invoke("start_recording");
export const stopRecording = () => invoke("stop_recording");

// Settings
export const getSettings = () => invoke<AppConfig>("get_settings");

/**
 * Retry getSettings with backoff — handles the race condition where
 * the WebView JS calls invoke() before setup has called app.manage().
 */
export async function getSettingsWithRetry(maxAttempts = 10): Promise<AppConfig> {
  for (let i = 0; i < maxAttempts; i++) {
    try {
      return await getSettings();
    } catch (e) {
      if (i === maxAttempts - 1) throw e;
      await new Promise((r) => setTimeout(r, 100 * (i + 1)));
    }
  }
  throw new Error("unreachable");
}
export const updateSettings = (config: AppConfig) =>
  invoke("update_settings", { config });

// Audio devices
export const listAudioDevices = () =>
  invoke<InputDeviceInfo[]>("list_audio_devices");

// History
export const getHistory = () => invoke<HistoryEntry[]>("get_history");
export const clearHistory = () => invoke("clear_history");

// Models
export const listModels = () => invoke<ModelInfo[]>("list_models");
export const downloadModel = (modelId: string) =>
  invoke("download_model", { modelId });
export const deleteModel = (modelId: string) =>
  invoke("delete_model", { modelId });
export const cancelDownload = (modelId: string) =>
  invoke("cancel_download", { modelId });

// Engines
export const listEngines = () => invoke<EngineInfo[]>("list_engines");
export const setActiveModel = (modelId: string) =>
  invoke("set_active_model", { modelId });
export const listSupportedLanguages = () =>
  invoke<LanguageInfo[]>("list_supported_languages");
export const listOllamaModels = () =>
  invoke<string[]>("list_ollama_models");

// Substitutions
export const getSubstitutions = () =>
  invoke<SubstitutionRule[]>("get_substitutions");
export const addSubstitution = (rule: SubstitutionRule) =>
  invoke("add_substitution", { rule });
export const deleteSubstitution = (index: number) =>
  invoke("delete_substitution", { index });

// LLM / Post-processing
export const checkLlmStatus = () => invoke<LlmStatus>("check_llm_status");
export const testReformulation = (text: string) =>
  invoke<string>("test_reformulation", { text });
export const testTranslation = (text: string, targetLanguage: string) =>
  invoke<string>("test_translation", { text, targetLanguage });
export const testTextPipeline = (text: string) =>
  invoke<PipelineTestResult>("test_text_pipeline", { text });
export const getPromptPreview = (style: string) =>
  invoke<PromptPreview>("get_prompt_preview", { style });

// GPU
export const detectNvidia = () => invoke<NvidiaInfo>("detect_nvidia");
export const detectCpuCount = () => invoke<number>("detect_cpu_count");
