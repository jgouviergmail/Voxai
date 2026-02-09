import { createSignal, onMount, onCleanup, Show } from "solid-js";
import type { AppConfig, HotkeyConfig, InputDeviceInfo, LanguageInfo, NvidiaInfo } from "../../types";
import { detectCpuCount, detectNvidia, listAudioDevices, listSupportedLanguages } from "../../lib/commands";
import Toggle from "../ui/Toggle";
import Select from "../ui/Select";
import Button from "../ui/Button";
import Section from "../ui/Section";
import { appStore } from "../../lib/stores";
import { i18n } from "../../lib/i18n";

/** Maps JS event.code / event.key to rdev key names used by the backend. */
function mapKeyToRdev(e: KeyboardEvent): string | null {
  const { code, key } = e;
  if (code === "Space") return "Space";
  if (/^F([1-9]|1[0-2])$/.test(code)) return code; // F1-F12
  // Use event.key for letters — layout-aware (works on AZERTY, QWERTZ, etc.)
  if (key.length === 1 && /^[a-zA-Z]$/.test(key)) return key.toUpperCase();
  // Digit keys
  const digit = code.match(/^Digit(\d)$/);
  if (digit) return digit[1];
  // Ignore modifier-only presses
  if (["Control", "Shift", "Alt", "Meta"].includes(key)) return null;
  return null;
}

function modifiersFromEvent(e: KeyboardEvent): string[] {
  const mods: string[] = [];
  if (e.ctrlKey) mods.push("Control");
  if (e.shiftKey) mods.push("Shift");
  if (e.altKey) mods.push("Alt");
  if (e.metaKey) mods.push("Meta");
  return mods;
}

function formatHotkey(key: string, modifiers: string[]): string {
  return [...modifiers, key].join("+");
}

/** Reusable hotkey recorder — avoids duplicating recorder logic. */
function createHotkeyRecorder(onRecorded: (key: string, mods: string[]) => void) {
  const [active, setActive] = createSignal(false);
  let handler: ((e: KeyboardEvent) => void) | null = null;
  let timeout: ReturnType<typeof setTimeout> | null = null;

  const cancel = () => {
    setActive(false);
    if (handler) {
      document.removeEventListener("keydown", handler, true);
      handler = null;
    }
    if (timeout) {
      clearTimeout(timeout);
      timeout = null;
    }
  };

  const start = () => {
    setActive(true);
    timeout = setTimeout(() => cancel(), 5000);
    handler = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const key = mapKeyToRdev(e);
      if (!key) return;
      const mods = modifiersFromEvent(e);
      cancel();
      onRecorded(key, mods);
    };
    document.addEventListener("keydown", handler, true);
  };

  return { active, cancel, start };
}

const DEFAULT_HOTKEY: HotkeyConfig = { key: "Space", modifiers: ["Control", "Shift"] };
const DEFAULT_TEXT_HOTKEY: HotkeyConfig = { key: "R", modifiers: ["Control", "Shift"] };

export default function GeneralTab() {
  const [devices, setDevices] = createSignal<InputDeviceInfo[]>([]);
  const [languages, setLanguages] = createSignal<LanguageInfo[]>([]);
  const [gpuInfo, setGpuInfo] = createSignal<NvidiaInfo | null>(null);
  const [cpuCount, setCpuCount] = createSignal(4);

  const save = (updater: (c: AppConfig) => void) => appStore.saveSetting(updater);

  // Push-to-talk hotkey recorder
  const pttRecorder = createHotkeyRecorder((key, mods) => {
    save((c) => { c.general.hotkey = { key, modifiers: mods }; });
  });

  // Text processing hotkey recorder
  const textRecorder = createHotkeyRecorder((key, mods) => {
    save((c) => { c.general.text_hotkey = { key, modifiers: mods }; });
  });

  onMount(async () => {
    try {
      const [d, langs] = await Promise.all([
        listAudioDevices(),
        listSupportedLanguages(),
      ]);
      setDevices(d);
      setLanguages(langs);
    } catch (e) {
      console.error("Failed to load devices/languages:", e);
    }
    try {
      setGpuInfo(await detectNvidia());
    } catch (e) {
      console.error("GPU detection failed:", e);
    }
    try {
      setCpuCount(await detectCpuCount());
    } catch (e) {
      console.error("CPU count detection failed:", e);
    }
  });

  onCleanup(() => {
    pttRecorder.cancel();
    textRecorder.cancel();
  });

  const config = () => appStore.config()!;

  const isDark = () => appStore.theme() === "dark";

  return (
    <div class="space-y-3">
      {/* ── Input ── */}
      <Section title={i18n.t("general.input")}>
        <Show when={devices().length > 0}>
          <Select
            label={i18n.t("general.microphone")}
            value={config().general.input_device ?? ""}
            options={[
              { value: "", label: i18n.t("general.default_device") },
              ...devices().map((d) => ({
                value: d.name,
                label: `${d.name}${d.is_default ? " (default)" : ""}`,
              })),
            ]}
            onChange={(v) => save((c) => (c.general.input_device = v || null))}
          />
        </Show>

        <Show when={languages().length > 0}>
          <Select
            label={i18n.t("general.language")}
            value={config().general.language ?? ""}
            options={languages().map((l) => ({
              value: l.code,
              label: l.name,
            }))}
            onChange={(v) => save((c) => (c.general.language = v || null))}
          />
        </Show>

        <Select
          label={i18n.t("general.ui_language")}
          value={config().general.ui_language ?? "en"}
          options={[
            { value: "en", label: "English" },
            { value: "fr", label: "Fran\u00e7ais" },
            { value: "zh", label: "\u4e2d\u6587" },
          ]}
          onChange={(v) => {
            save((c) => (c.general.ui_language = v));
            i18n.setLocale(v);
          }}
        />
      </Section>

      {/* ── Behavior ── */}
      <Section title={i18n.t("general.behavior")}>
        <Toggle
          label={i18n.t("general.auto_enter")}
          description={i18n.t("general.auto_enter_desc")}
          checked={config().general.auto_enter}
          onChange={(v) => save((c) => (c.general.auto_enter = v))}
        />

        <Toggle
          label={i18n.t("general.restore_clipboard")}
          description={i18n.t("general.restore_clipboard_desc")}
          checked={config().general.clipboard_restore}
          onChange={(v) => save((c) => (c.general.clipboard_restore = v))}
        />
      </Section>

      {/* ── Mode ── */}
      <Section title={i18n.t("general.mode")}>
        <Toggle
          label={i18n.t("general.real_time")}
          description={i18n.t("general.real_time_desc")}
          checked={config().general.real_time}
          onChange={(v) => save((c) => (c.general.real_time = v))}
        />
      </Section>

      {/* ── Hotkey ── */}
      <Section title={i18n.t("general.hotkey")}>
        <div
          class={`rounded-lg p-3 text-sm ${
            isDark()
              ? "bg-white/5 border border-border-subtle"
              : "bg-surface-overlay-light border border-border-subtle-lt"
          }`}
        >
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <span class={isDark() ? "text-white/44" : "text-black/40"}>
                {i18n.t("general.push_to_talk")}{" "}
              </span>
              {pttRecorder.active() ? (
                <span class="text-amber-400 animate-pulse text-xs font-medium">
                  {i18n.t("general.press_shortcut")}
                </span>
              ) : (
                <kbd
                  class={`px-2 py-0.5 rounded-md text-xs font-mono ${
                    isDark()
                      ? "bg-surface-overlay border border-border-default text-white/64"
                      : "bg-surface-overlay-light border border-border-default-lt text-black/40"
                  }`}
                >
                  {formatHotkey(
                    config().general.hotkey.key,
                    config().general.hotkey.modifiers
                  )}
                </kbd>
              )}
            </div>
            <div class="flex gap-2">
              <Button
                size="sm"
                variant={pttRecorder.active() ? "danger" : "secondary"}
                onClick={() => pttRecorder.active() ? pttRecorder.cancel() : pttRecorder.start()}
              >
                {pttRecorder.active() ? i18n.t("general.cancel") : i18n.t("general.record")}
              </Button>
              <Button
                size="sm"
                variant="secondary"
                onClick={() => save((c) => { c.general.hotkey = { ...DEFAULT_HOTKEY }; })}
              >
                {i18n.t("general.reset")}
              </Button>
            </div>
          </div>
        </div>
      </Section>

      {/* ── Text Hotkey ── */}
      <Section title={i18n.t("general.text_hotkey")}>
        <Toggle
          label={i18n.t("general.text_hotkey_enable")}
          description={i18n.t("general.text_hotkey_desc")}
          checked={config().general.text_hotkey !== null}
          onChange={(v) =>
            save((c) => {
              c.general.text_hotkey = v ? { ...DEFAULT_TEXT_HOTKEY } : null;
            })
          }
        />
        <Show when={config().general.text_hotkey}>
          <div
            class={`rounded-lg p-3 text-sm mt-2 ${
              isDark()
                ? "bg-white/5 border border-border-subtle"
                : "bg-surface-overlay-light border border-border-subtle-lt"
            }`}
          >
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2">
                {textRecorder.active() ? (
                  <span class="text-amber-400 animate-pulse text-xs font-medium">
                    {i18n.t("general.press_shortcut")}
                  </span>
                ) : (
                  <kbd
                    class={`px-2 py-0.5 rounded-md text-xs font-mono ${
                      isDark()
                        ? "bg-surface-overlay border border-border-default text-white/64"
                        : "bg-surface-overlay-light border border-border-default-lt text-black/40"
                    }`}
                  >
                    {formatHotkey(
                      config().general.text_hotkey!.key,
                      config().general.text_hotkey!.modifiers
                    )}
                  </kbd>
                )}
              </div>
              <div class="flex gap-2">
                <Button
                  size="sm"
                  variant={textRecorder.active() ? "danger" : "secondary"}
                  onClick={() => textRecorder.active() ? textRecorder.cancel() : textRecorder.start()}
                >
                  {textRecorder.active() ? i18n.t("general.cancel") : i18n.t("general.record")}
                </Button>
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => save((c) => { c.general.text_hotkey = { ...DEFAULT_TEXT_HOTKEY }; })}
                >
                  {i18n.t("general.reset")}
                </Button>
              </div>
            </div>
          </div>
        </Show>
      </Section>

      {/* ── GPU ── */}
      <Section title={i18n.t("general.gpu")}>
        <Show when={gpuInfo()?.detected} fallback={
          <p class={`text-xs ${isDark() ? "text-white/44" : "text-black/40"}`}>
            {i18n.t("general.no_gpu")}
          </p>
        }>
          <div
            class={`rounded-lg p-3 text-sm mb-2 ${
              isDark()
                ? "bg-white/5 border border-border-subtle"
                : "bg-surface-overlay-light border border-border-subtle-lt"
            }`}
          >
            <div class="flex items-center gap-2">
              <div class="w-2 h-2 rounded-full bg-emerald-400" />
              <span>{gpuInfo()!.gpu_name}</span>
            </div>
            <p class={`text-xs mt-1 ${isDark() ? "text-white/44" : "text-black/40"}`}>
              {i18n.t("general.driver")} {gpuInfo()!.driver_version} &middot; {gpuInfo()!.vram_mb} MB VRAM
            </p>
          </div>
          <Toggle
            label={i18n.t("general.enable_gpu")}
            description={gpuInfo()!.cuda_compiled ? i18n.t("general.enable_gpu_desc") : i18n.t("general.cpu_build_hint")}
            checked={gpuInfo()!.cuda_compiled && config().general.gpu_acceleration}
            onChange={(v) => save((c) => (c.general.gpu_acceleration = v))}
            disabled={!gpuInfo()!.cuda_compiled}
          />
        </Show>
      </Section>

      {/* ── Performance ── */}
      <Section title={i18n.t("general.performance")}>
        <div class="space-y-1">
          <div class="flex items-center justify-between">
            <label class={`text-sm ${isDark() ? "text-white/64" : "text-black/60"}`}>
              {i18n.t("general.stt_threads")}
            </label>
            <span class={`text-sm font-mono ${isDark() ? "text-white/44" : "text-black/40"}`}>
              {config().general.stt_threads === null
                ? i18n.t("general.stt_threads_auto")
                : `${config().general.stt_threads} / ${cpuCount()}`}
            </span>
          </div>
          <input
            type="range"
            min="0"
            max={cpuCount()}
            step="1"
            value={config().general.stt_threads ?? 0}
            class="w-full accent-blue-500"
            onInput={(e) => {
              const val = parseInt(e.currentTarget.value);
              save((c) => (c.general.stt_threads = val === 0 ? null : val));
            }}
          />
          <p class={`text-xs ${isDark() ? "text-white/44" : "text-black/40"}`}>
            {i18n.t("general.stt_threads_desc")}
          </p>
          <Show when={config().general.gpu_acceleration}>
            <p class="text-xs text-amber-400">
              {i18n.t("general.stt_threads_gpu_note")}
            </p>
          </Show>
        </div>
      </Section>
    </div>
  );
}
