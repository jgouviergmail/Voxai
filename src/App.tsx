import { createSignal, onMount, onCleanup, Show, Switch, Match } from "solid-js";
import { getSettings, getSettingsWithRetry, getHistory, getRecordingState } from "./lib/commands";
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
  const [loadError, setLoadError] = createSignal("");

  const tabs = () => [
    { id: "general", label: i18n.t("tab.general") },
    { id: "engines", label: i18n.t("tab.engines") },
    { id: "postprocessing", label: i18n.t("tab.postprocessing") },
    { id: "substitutions", label: i18n.t("tab.substitutions") },
    { id: "history", label: i18n.t("tab.history") },
  ];

  onMount(async () => {
    try {
      const config = await getSettingsWithRetry();
      appStore.setConfig(config);
      if (config.general.ui_language) {
        i18n.setLocale(config.general.ui_language);
      }
    } catch (e) {
      console.error("Failed to load settings:", e);
      setLoadError(String(e));
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

  const isDark = () => appStore.theme() === "dark";

  const statusAccentBg = () => {
    switch (appStore.recordingState().kind) {
      case "Idle": return "bg-emerald-500";
      case "Recording": return "bg-red-500";
      case "Processing": return "bg-amber-500";
    }
  };

  const statusDotBg = () => {
    switch (appStore.recordingState().kind) {
      case "Idle": return "bg-emerald-400";
      case "Recording": return "bg-red-400";
      case "Processing": return "bg-amber-400";
    }
  };

  const statusGlowColor = () => {
    switch (appStore.recordingState().kind) {
      case "Idle": return "#34d399";
      case "Recording": return "#f87171";
      case "Processing": return "#fbbf24";
    }
  };

  const isAnimated = () => appStore.recordingState().kind !== "Idle";

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
      class={`rounded-xl overflow-hidden flex items-stretch ${
        isDark()
          ? "bg-surface-raised border border-border-subtle"
          : "bg-surface-raised-light border border-border-subtle-lt shadow-card-lt"
      }`}
    >
      {/* Left accent strip */}
      <div class={`w-1 shrink-0 ${statusAccentBg()}`} />
      {/* Content */}
      <div class="flex items-center gap-3 px-3 py-2.5 flex-1 min-w-0">
        <div
          class={`w-2 h-2 rounded-full shrink-0 ${statusDotBg()} ${isAnimated() ? "status-glow" : ""}`}
          style={{ "--glow-color": statusGlowColor() }}
        />
        <span class="text-sm font-medium truncate">{statusText()}</span>
        <kbd
          class={`ml-auto shrink-0 px-2 py-0.5 rounded-md text-xs font-mono ${
            isDark()
              ? "bg-surface-overlay border border-border-subtle text-white/44"
              : "bg-surface-overlay-light border border-border-subtle-lt text-black/40"
          }`}
        >
          {(() => {
            const cfg = appStore.config();
            if (!cfg) return "Ctrl+Shift+Space";
            const hk = cfg.general.hotkey;
            return [...hk.modifiers, hk.key].join("+");
          })()}
        </kbd>
      </div>
    </div>
  );

  return (
    <PageShell
      statusBar={statusBarJsx()}
      tabBar={<TabBar tabs={tabs()} active={activeTab()} onSelect={setActiveTab} />}
    >
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
        <div class="p-4">
          <p class={`text-sm ${isDark() ? "text-white/44" : "text-black/40"}`}>
            {i18n.t("loading.settings")}
          </p>
          <Show when={loadError()}>
            <p class="text-red-500 text-xs mt-2 font-mono break-all">
              {loadError()}
            </p>
          </Show>
          <p class="text-white/30 text-[10px] mt-4 font-mono">v0.1.1</p>
        </div>
      </Show>
    </PageShell>
  );
}

export default App;
