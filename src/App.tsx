import { createSignal, onMount, onCleanup, Show, Switch, Match } from "solid-js";
import type { AppConfig } from "./types";
import { getSettings, getHistory, getRecordingState } from "./lib/commands";
import {
  onRecordingStateChanged,
  onTranscriptionComplete,
  onError,
} from "./lib/events";
import { appStore } from "./lib/stores";
import PageShell from "./components/layout/PageShell";
import TabBar from "./components/layout/TabBar";
import GeneralTab from "./components/settings/GeneralTab";
import EnginesTab from "./components/settings/EnginesTab";
import PostProcessingTab from "./components/settings/PostProcessingTab";
import SubstitutionsTab from "./components/settings/SubstitutionsTab";
import HistoryTab from "./components/settings/HistoryTab";

const TABS = [
  { id: "general", label: "General" },
  { id: "engines", label: "Engines" },
  { id: "postprocessing", label: "Post-Processing" },
  { id: "substitutions", label: "Substitutions" },
  { id: "history", label: "History" },
];

function App() {
  const [activeTab, setActiveTab] = createSignal("general");

  onMount(async () => {
    try {
      const config = await getSettings();
      appStore.setConfig(config);
    } catch (e) {
      console.error("Failed to load settings:", e);
    }

    try {
      const state = await getRecordingState();
      appStore.setRecordingState(state);
    } catch (e) {
      console.error("Failed to get recording state:", e);
    }

    try {
      const entries = await getHistory();
      appStore.setHistory(entries);
    } catch (e) {
      console.error("Failed to get history:", e);
    }

    const unlistenState = await onRecordingStateChanged((state) => {
      appStore.setRecordingState(state);
    });

    const unlistenTranscription = await onTranscriptionComplete((entry) => {
      appStore.setHistory((prev) => [entry, ...prev].slice(0, 100));
    });

    const unlistenError = await onError((msg) => {
      appStore.showError(msg);
    });

    onCleanup(() => {
      unlistenState();
      unlistenTranscription();
      unlistenError();
    });
  });

  const handleConfigUpdate = (config: AppConfig) => {
    appStore.setConfig(config);
  };

  const statusColor = () => {
    const state = appStore.recordingState();
    switch (state.kind) {
      case "Idle":
        return "bg-green-500";
      case "Recording":
        return "bg-red-500 animate-pulse";
      case "Processing":
        return "bg-yellow-500 animate-pulse";
    }
  };

  const statusText = () => {
    const state = appStore.recordingState();
    switch (state.kind) {
      case "Idle":
        return "Ready";
      case "Recording":
        return "Recording...";
      case "Processing":
        return `Processing (${state.stage})...`;
      default:
        return "Unknown";
    }
  };

  return (
    <PageShell>
      {/* Status bar */}
      <div
        class={`rounded-lg p-3 mb-4 ${
          appStore.theme() === "dark" ? "bg-gray-800" : "bg-gray-100"
        }`}
      >
        <div class="flex items-center gap-3">
          <div class={`w-2.5 h-2.5 rounded-full ${statusColor()}`} />
          <span class="text-sm font-medium">{statusText()}</span>
          <span
            class={`text-xs ml-auto ${
              appStore.theme() === "dark" ? "text-gray-500" : "text-gray-400"
            }`}
          >
            <kbd
              class={`px-1.5 py-0.5 rounded text-xs ${
                appStore.theme() === "dark" ? "bg-gray-700" : "bg-gray-200"
              }`}
            >
              {(() => {
                const cfg = appStore.config();
                if (!cfg) return "Ctrl+Shift+Space";
                const hk = cfg.general.hotkey;
                return [...hk.modifiers, hk.key].join("+");
              })()}
            </kbd>
          </span>
        </div>
      </div>

      {/* Tabs */}
      <TabBar tabs={TABS} active={activeTab()} onSelect={setActiveTab} />

      {/* Tab content */}
      <Show when={appStore.config()}>
        {(config) => (
          <Switch>
            <Match when={activeTab() === "general"}>
              <GeneralTab config={config()} onUpdate={handleConfigUpdate} />
            </Match>
            <Match when={activeTab() === "engines"}>
              <EnginesTab />
            </Match>
            <Match when={activeTab() === "postprocessing"}>
              <PostProcessingTab
                config={config()}
                onUpdate={handleConfigUpdate}
              />
            </Match>
            <Match when={activeTab() === "substitutions"}>
              <SubstitutionsTab
                config={config()}
                onUpdate={handleConfigUpdate}
              />
            </Match>
            <Match when={activeTab() === "history"}>
              <HistoryTab />
            </Match>
          </Switch>
        )}
      </Show>

      <Show when={!appStore.config()}>
        <p
          class={`text-sm ${
            appStore.theme() === "dark" ? "text-gray-500" : "text-gray-400"
          }`}
        >
          Loading settings...
        </p>
      </Show>
    </PageShell>
  );
}

export default App;
