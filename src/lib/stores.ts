import { createSignal, createRoot } from "solid-js";
import type { AppConfig, RecordingState, HistoryEntry } from "../types";
import { updateSettings } from "./commands";

function createAppStore() {
  const [config, setConfig] = createSignal<AppConfig | null>(null);
  const [recordingState, setRecordingState] = createSignal<RecordingState>({
    kind: "Idle",
  });
  const [history, setHistory] = createSignal<HistoryEntry[]>([]);
  const [error, setError] = createSignal<string | null>(null);
  const [theme, setTheme] = createSignal<"dark" | "light">("dark");

  const showError = (msg: string) => {
    setError(msg);
    setTimeout(() => setError(null), 5000);
  };

  const toggleTheme = () => {
    setTheme((t) => (t === "dark" ? "light" : "dark"));
  };

  // Serialized save queue — prevents race conditions from rapid concurrent saves
  let saveQueue: Promise<void> = Promise.resolve();
  const saveSetting = (updater: (c: AppConfig) => void): Promise<void> => {
    saveQueue = saveQueue.then(async () => {
      const current = config();
      if (!current) return;
      const c = structuredClone(current);
      updater(c);
      try {
        await updateSettings(c);
        setConfig(c);
      } catch (e) {
        showError(String(e));
      }
    });
    return saveQueue;
  };

  return {
    config,
    setConfig,
    recordingState,
    setRecordingState,
    history,
    setHistory,
    error,
    setError,
    showError,
    theme,
    setTheme,
    toggleTheme,
    saveSetting,
  };
}

export const appStore = createRoot(createAppStore);
