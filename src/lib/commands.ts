import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  EngineInfo,
  HistoryEntry,
  InputDeviceInfo,
  LanguageInfo,
  LlmStatus,
  ModelInfo,
  PipelineTestResult,
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

// Engines
export const listEngines = () => invoke<EngineInfo[]>("list_engines");
export const setActiveModel = (modelId: string) =>
  invoke("set_active_model", { modelId });
export const listSupportedLanguages = () =>
  invoke<LanguageInfo[]>("list_supported_languages");

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
