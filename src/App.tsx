import { createSignal, onMount, onCleanup, Show, Switch, Match } from "solid-js";
import { getSettings, getHistory, getRecordingState } from "./lib/commands";
import {
  onRecordingStateChanged,
  onTranscriptionComplete,
  onSettingsUpdated,
  onError,
} from "./lib/events";
import { appStore } from "./lib/stores";
import { i18n } from "./lib/i18n";
import PageShell from "./components/layout/PageShell";
import TabBar from "./components/layout/TabBar";
import GeneralTab from "./components/settings/GeneralTab";
import EnginesTab from "./components/settings/EnginesTab";
import PostProcessingTab from "./components/settings/PostProcessingTab";
import SubstitutionsTab from "./components/settings/SubstitutionsTab";
import HistoryTab from "./components/settings/HistoryTab";

const TAB_IDS = ["general", "engines", "postprocessing", "substitutions", "history"] as const;

function App() {
  const [activeTab, setActiveTab] = createSignal("general");

  const tabs = () => [
    { id: "general", label: i18n.t("tab.general") },
    { id: "engines", label: i18n.t("tab.engines") },
    { id: "postprocessing", label: i18n.t("tab.postprocessing") },
    { id: "substitutions", label: i18n.t("tab.substitutions") },
    { id: "history", label: i18n.t("tab.history") },
  ];

  onMount(async () => {
    try {
      const config = await getSettings();
      appStore.setConfig(config);
      if (config.general.ui_language) {
        i18n.setLocale(config.general.ui_language);
      }
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

    const unlistenSettings = await onSettingsUpdated(async () => {
      const fresh = await getSettings();
      appStore.setConfig(fresh);
      if (fresh.general.ui_language) {
        i18n.setLocale(fresh.general.ui_language);
      }
    });

    const unlistenError = await onError((msg) => {
      appStore.showError(msg);
    });

    onCleanup(() => {
      unlistenState();
      unlistenTranscription();
      unlistenSettings();
      unlistenError();
    });
  });

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
        return i18n.t("status.ready");
      case "Recording":
        return i18n.t("status.recording");
      case "Processing":
        return `${i18n.t("status.processing")} (${state.stage})...`;
      default:
        return i18n.t("status.unknown");
    }
  };

  const statusBarJsx = () => (
    <div
      class={`rounded-lg p-3 ${
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
  );

  return (
    <PageShell statusBar={statusBarJsx()}>
      {/* Tabs */}
      <TabBar tabs={tabs()} active={activeTab()} onSelect={setActiveTab} />

      {/* Tab content */}
      <Show when={appStore.config()}>
        <Switch>
          <Match when={activeTab() === "general"}>
            <GeneralTab />
          </Match>
          <Match when={activeTab() === "engines"}>
            <EnginesTab />
          </Match>
          <Match when={activeTab() === "postprocessing"}>
            <PostProcessingTab />
          </Match>
          <Match when={activeTab() === "substitutions"}>
            <SubstitutionsTab />
          </Match>
          <Match when={activeTab() === "history"}>
            <HistoryTab />
          </Match>
        </Switch>
      </Show>

      <Show when={!appStore.config()}>
        <p
          class={`text-sm ${
            appStore.theme() === "dark" ? "text-gray-500" : "text-gray-400"
          }`}
        >
          {i18n.t("loading.settings")}
        </p>
      </Show>
    </PageShell>
  );
}

export default App;
