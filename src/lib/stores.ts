import { createSignal, createRoot } from "solid-js";
import type { AppConfig, RecordingState, HistoryEntry } from "../types";

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
  };
}

export const appStore = createRoot(createAppStore);
