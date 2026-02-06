import { createSignal, onMount, onCleanup, Show, For } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppConfig, LanguageInfo, RecordingState, ReformulationStyle } from "./types";
import { i18n } from "./lib/i18n";
import { BUILTIN_STYLES } from "./lib/constants";
import { getSettingsWithRetry } from "./lib/commands";

export default function Overlay() {
  const [state, setState] = createSignal<RecordingState>({ kind: "Idle" });
  const [expanded, setExpanded] = createSignal(false);
  const [config, setConfig] = createSignal<AppConfig | null>(null);
  const [languages, setLanguages] = createSignal<LanguageInfo[]>([]);

  const saveOverlay = async (updater: (c: AppConfig) => void) => {
    const current = config();
    if (!current) return;
    const c = structuredClone(current);
    updater(c);
    try {
      await invoke("update_settings", { config: c });
      setConfig(c);
    } catch (e) {
      console.error("Overlay save failed:", e);
    }
  };

  onMount(async () => {
    const unlistens: UnlistenFn[] = [];

    // Load initial state (with retry — WebView may fire before backend setup completes)
    try {
      const cfg = await getSettingsWithRetry();
      setConfig(cfg);
      if (cfg.general.ui_language) i18n.setLocale(cfg.general.ui_language);
    } catch (e) {
      console.error("Overlay: failed to load settings:", e);
    }

    try {
      setLanguages(await invoke<LanguageInfo[]>("list_supported_languages"));
    } catch (e) {
      console.error("Overlay: failed to load languages:", e);
    }

    // Listen for recording state changes
    unlistens.push(
      await listen<RecordingState>("recording-state-changed", (e) => setState(e.payload)),
    );

    // Listen for settings changes (from main window)
    unlistens.push(
      await listen("settings-updated", async () => {
        try {
          const fresh = await invoke<AppConfig>("get_settings");
          setConfig(fresh);
          if (fresh.general.ui_language) i18n.setLocale(fresh.general.ui_language);
        } catch (e) {
          console.error("Overlay: settings refresh failed:", e);
        }
      }),
    );

    onCleanup(() => unlistens.forEach((fn) => fn()));
  });

  const statusColor = () => {
    switch (state().kind) {
      case "Idle": return "#22c55e";       // green-500
      case "Recording": return "#ef4444";  // red-500
      case "Processing": return "#eab308"; // yellow-500
    }
  };

  const isAnimated = () => state().kind !== "Idle";

  const label = () => {
    switch (state().kind) {
      case "Idle":
        return i18n.t("status.ready");
      case "Recording":
        return i18n.t("status.recording");
      case "Processing":
        return i18n.t("status.processing");
    }
  };

  const currentStyleValue = (): string => {
    const cfg = config();
    if (!cfg) return "Cleaned";
    const s = cfg.postprocessing.reformulation.style;
    return typeof s === "string" ? s : (s as { Custom: string }).Custom;
  };

  const allStyleOptions = () => {
    const customs = config()?.postprocessing.custom_prompts ?? [];
    return [
      ...BUILTIN_STYLES.map((s) => ({ value: s, label: i18n.t(`pp.style_${s.toLowerCase()}`) })),
      ...customs.map((c) => ({ value: c.id, label: c.name })),
    ];
  };

  const setStyle = (val: string) => {
    const style: ReformulationStyle = (BUILTIN_STYLES as readonly string[]).includes(val)
      ? (val as ReformulationStyle)
      : { Custom: val };
    saveOverlay((c) => { c.postprocessing.reformulation.style = style; });
  };

  return (
    <div class="p-1">
      {/* Pill header — draggable */}
      <div
        data-tauri-drag-region
        class="flex items-center gap-2 px-3 py-1.5 rounded-full bg-gray-900/80 backdrop-blur-sm border border-gray-700/50 shadow-lg cursor-move select-none"
        onClick={() => setExpanded((v) => !v)}
      >
        {/* Microphone icon in pill */}
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" class="shrink-0">
          <path
            d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3Z"
            fill={statusColor()}
          >
            <Show when={isAnimated()}>
              <animate attributeName="opacity" values="1;0.4;1" dur="1.2s" repeatCount="indefinite" />
            </Show>
          </path>
          <path
            d="M19 10v2a7 7 0 0 1-14 0v-2"
            stroke={statusColor()}
            stroke-width="2"
            stroke-linecap="round"
          >
            <Show when={isAnimated()}>
              <animate attributeName="opacity" values="1;0.4;1" dur="1.2s" repeatCount="indefinite" />
            </Show>
          </path>
          <line x1="12" y1="19" x2="12" y2="23" stroke={statusColor()} stroke-width="2" stroke-linecap="round" />
          <line x1="8" y1="23" x2="16" y2="23" stroke={statusColor()} stroke-width="2" stroke-linecap="round" />
        </svg>
        <span class="text-xs font-medium text-gray-200 whitespace-nowrap pointer-events-none">
          {label()}
        </span>
        <span class="text-[10px] text-gray-500 ml-auto pointer-events-none">
          {expanded() ? "\u25BC" : "\u25B2"}
        </span>
      </div>

      {/* Expanded panel */}
      <Show when={expanded() && config()}>
        <div class="mt-1 rounded-lg bg-gray-900/90 backdrop-blur-sm border border-gray-700/50 p-2 shadow-lg text-xs text-gray-300 space-y-2">
          {/* Translation row */}
          <div class="flex items-center gap-2">
            <span class="w-20 shrink-0">{i18n.t("overlay.translation")}</span>
            <label class="relative inline-flex items-center cursor-pointer">
              <input
                type="checkbox"
                class="sr-only peer"
                checked={config()!.postprocessing.translation.enabled}
                onChange={(e) =>
                  saveOverlay((c) => { c.postprocessing.translation.enabled = e.currentTarget.checked; })
                }
              />
              <div class="w-7 h-4 bg-gray-700 rounded-full peer peer-checked:bg-blue-500 after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-3 after:w-3 after:transition-all peer-checked:after:translate-x-3" />
            </label>
            <Show when={config()!.postprocessing.translation.enabled && languages().length > 0}>
              <select
                class="bg-gray-800 text-gray-300 text-[10px] rounded px-1 py-0.5 border border-gray-700 flex-1 min-w-0"
                value={config()!.postprocessing.translation.target_language}
                onChange={(e) =>
                  saveOverlay((c) => { c.postprocessing.translation.target_language = e.currentTarget.value; })
                }
              >
                <For each={languages()}>
                  {(l) => <option value={l.code}>{l.name}</option>}
                </For>
              </select>
            </Show>
          </div>

          {/* Reformulation row */}
          <div class="flex items-center gap-2">
            <span class="w-20 shrink-0">{i18n.t("overlay.reformulation")}</span>
            <label class="relative inline-flex items-center cursor-pointer">
              <input
                type="checkbox"
                class="sr-only peer"
                checked={config()!.postprocessing.reformulation.enabled}
                onChange={(e) =>
                  saveOverlay((c) => { c.postprocessing.reformulation.enabled = e.currentTarget.checked; })
                }
              />
              <div class="w-7 h-4 bg-gray-700 rounded-full peer peer-checked:bg-blue-500 after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-3 after:w-3 after:transition-all peer-checked:after:translate-x-3" />
            </label>
            <Show when={config()!.postprocessing.reformulation.enabled}>
              <select
                class="bg-gray-800 text-gray-300 text-[10px] rounded px-1 py-0.5 border border-gray-700 flex-1 min-w-0"
                value={currentStyleValue()}
                onChange={(e) => setStyle(e.currentTarget.value)}
              >
                <For each={allStyleOptions()}>
                  {(opt) => <option value={opt.value}>{opt.label}</option>}
                </For>
              </select>
            </Show>
          </div>
        </div>
      </Show>
    </div>
  );
}
