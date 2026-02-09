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
  const [partialText, setPartialText] = createSignal("");

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
      await listen<RecordingState>("recording-state-changed", (e) => {
        setState(e.payload);
        if (e.payload.kind === "Idle") setPartialText("");
      }),
    );

    // Listen for partial transcription (streaming mode)
    unlistens.push(
      await listen<string>("transcription-partial", (e) => {
        setPartialText(e.payload);
      }),
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
        class="flex items-center gap-2 px-3 py-1.5 glass rounded-full shadow-float cursor-move select-none"
        onClick={() => setExpanded((v) => !v)}
      >
        {/* Microphone icon in pill */}
        <span
          class="shrink-0"
          style={{
            filter: isAnimated() ? `drop-shadow(0 0 6px ${statusColor()})` : "none",
            transition: "filter 0.3s ease",
          }}
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
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
        </span>
        <span class="text-xs font-medium text-white/90 whitespace-nowrap pointer-events-none">
          {label()}
        </span>
        <svg class={`w-3 h-3 text-white/44 ml-auto pointer-events-none transition-transform duration-200 ${expanded() ? "rotate-180" : ""}`} viewBox="0 0 20 20" fill="currentColor">
          <path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.938a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z" clip-rule="evenodd"/>
        </svg>
      </div>

      {/* Expanded panel */}
      <Show when={expanded() && config()}>
        <div class="mt-1 glass-panel animate-slide-down rounded-xl shadow-float p-3 text-xs text-gray-300 space-y-3">
          {/* Translation row */}
          <div class="flex items-center gap-2">
            <span class="w-20 shrink-0 text-white/50 font-medium">{i18n.t("overlay.translation")}</span>
            <label class="relative inline-flex items-center cursor-pointer">
              <input
                type="checkbox"
                class="sr-only peer"
                checked={config()!.postprocessing.translation.enabled}
                onChange={(e) =>
                  saveOverlay((c) => { c.postprocessing.translation.enabled = e.currentTarget.checked; })
                }
              />
              <div class="w-8 h-[18px] bg-white/10 rounded-full peer peer-checked:bg-blue-500 after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:h-3.5 after:w-3.5 after:bg-white after:rounded-full after:shadow-sm after:transition-all peer-checked:after:translate-x-3.5" />
            </label>
            <Show when={config()!.postprocessing.translation.enabled && languages().length > 0}>
              <select
                class="bg-white/8 text-white/80 text-[10px] rounded-md px-1.5 py-0.5 border border-white/10 focus:ring-1 focus:ring-accent-glow flex-1 min-w-0"
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
            <span class="w-20 shrink-0 text-white/50 font-medium">{i18n.t("overlay.reformulation")}</span>
            <label class="relative inline-flex items-center cursor-pointer">
              <input
                type="checkbox"
                class="sr-only peer"
                checked={config()!.postprocessing.reformulation.enabled}
                onChange={(e) =>
                  saveOverlay((c) => { c.postprocessing.reformulation.enabled = e.currentTarget.checked; })
                }
              />
              <div class="w-8 h-[18px] bg-white/10 rounded-full peer peer-checked:bg-blue-500 after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:h-3.5 after:w-3.5 after:bg-white after:rounded-full after:shadow-sm after:transition-all peer-checked:after:translate-x-3.5" />
            </label>
            <Show when={config()!.postprocessing.reformulation.enabled}>
              <select
                class="bg-white/8 text-white/80 text-[10px] rounded-md px-1.5 py-0.5 border border-white/10 focus:ring-1 focus:ring-accent-glow flex-1 min-w-0"
                value={currentStyleValue()}
                onChange={(e) => setStyle(e.currentTarget.value)}
              >
                <For each={allStyleOptions()}>
                  {(opt) => <option value={opt.value}>{opt.label}</option>}
                </For>
              </select>
            </Show>
          </div>

          {/* Real-time row */}
          <div class="flex items-center gap-2">
            <span class="w-20 shrink-0 text-white/50 font-medium">{i18n.t("overlay.real_time")}</span>
            <label class="relative inline-flex items-center cursor-pointer">
              <input
                type="checkbox"
                class="sr-only peer"
                checked={config()!.general.real_time}
                onChange={(e) =>
                  saveOverlay((c) => { c.general.real_time = e.currentTarget.checked; })
                }
              />
              <div class="w-8 h-[18px] bg-white/10 rounded-full peer peer-checked:bg-blue-500 after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:h-3.5 after:w-3.5 after:bg-white after:rounded-full after:shadow-sm after:transition-all peer-checked:after:translate-x-3.5" />
            </label>
          </div>

          {/* LLM latency warning */}
          <Show when={config()!.general.real_time && (config()!.postprocessing.reformulation.enabled || config()!.postprocessing.translation.enabled)}>
            <div class="rounded-md px-2 py-1 bg-amber-500/10 border border-amber-500/20 text-amber-400 text-[10px]">
              {i18n.t("overlay.llm_latency_warn")}
            </div>
          </Show>
        </div>
      </Show>

      {/* Streaming partial text feedback */}
      <Show when={config()?.general.real_time && state().kind !== "Idle" && partialText()}>
        <div class="mt-1 glass-panel rounded-xl p-2.5 shadow-float text-xs text-white/85 max-h-24 overflow-y-auto pointer-events-none">
          {partialText()}
        </div>
      </Show>
    </div>
  );
}
