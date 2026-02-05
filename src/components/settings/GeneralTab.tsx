import { createSignal, onMount, onCleanup } from "solid-js";
import type { AppConfig, InputDeviceInfo, LanguageInfo } from "../../types";
import { listAudioDevices, listSupportedLanguages, updateSettings } from "../../lib/commands";
import Toggle from "../ui/Toggle";
import Select from "../ui/Select";
import Button from "../ui/Button";
import { appStore } from "../../lib/stores";

interface GeneralTabProps {
  config: AppConfig;
  onUpdate: (config: AppConfig) => void;
}

/** Maps JS event.code / event.key to rdev key names used by the backend. */
function mapKeyToRdev(e: KeyboardEvent): string | null {
  const { code, key } = e;
  if (code === "Space") return "Space";
  if (/^F([1-9]|1[0-2])$/.test(code)) return code; // F1-F12
  const letter = code.match(/^Key([A-Z])$/);
  if (letter) return letter[1]; // A-Z
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

const DEFAULT_HOTKEY = { key: "Space", modifiers: ["Control", "Shift"] };

export default function GeneralTab(props: GeneralTabProps) {
  const [devices, setDevices] = createSignal<InputDeviceInfo[]>([]);
  const [languages, setLanguages] = createSignal<LanguageInfo[]>([]);
  const [saving, setSaving] = createSignal(false);
  const [recording, setRecording] = createSignal(false);

  // Hotkey recorder state — tracked outside signals for proper cleanup
  let activeHandler: ((e: KeyboardEvent) => void) | null = null;
  let activeTimeout: ReturnType<typeof setTimeout> | null = null;

  const cancelRecording = () => {
    setRecording(false);
    if (activeHandler) {
      document.removeEventListener("keydown", activeHandler, true);
      activeHandler = null;
    }
    if (activeTimeout) {
      clearTimeout(activeTimeout);
      activeTimeout = null;
    }
  };

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
  });

  onCleanup(() => cancelRecording());

  const save = async (updater: (c: AppConfig) => void) => {
    const c = structuredClone(props.config);
    updater(c);
    setSaving(true);
    try {
      await updateSettings(c);
      props.onUpdate(c);
    } catch (e) {
      appStore.showError(String(e));
    }
    setSaving(false);
  };

  const isDark = () => appStore.theme() === "dark";
  const borderClass = () =>
    isDark() ? "border-gray-800" : "border-gray-200";
  const headingColor = () =>
    isDark() ? "text-gray-400" : "text-gray-500";

  return (
    <div class="space-y-4">
      <h3 class={`text-sm font-semibold ${headingColor()} uppercase tracking-wider`}>
        Input
      </h3>

      <Select
        label="Microphone"
        value={props.config.general.input_device ?? ""}
        options={[
          { value: "", label: "Default device" },
          ...devices().map((d) => ({
            value: d.name,
            label: `${d.name}${d.is_default ? " (default)" : ""}`,
          })),
        ]}
        onChange={(v) => save((c) => (c.general.input_device = v || null))}
      />

      <Select
        label="Language"
        value={props.config.general.language}
        options={languages().map((l) => ({
          value: l.code,
          label: l.name,
        }))}
        onChange={(v) => save((c) => (c.general.language = v))}
      />

      <div class={`border-t ${borderClass()} pt-4 mt-4`}>
        <h3 class={`text-sm font-semibold ${headingColor()} uppercase tracking-wider mb-2`}>
          Behavior
        </h3>

        <Toggle
          label="Auto-enter"
          description="Press Enter after injecting text"
          checked={props.config.general.auto_enter}
          onChange={(v) => save((c) => (c.general.auto_enter = v))}
        />

        <Toggle
          label="Restore clipboard"
          description="Restore previous clipboard content after paste"
          checked={props.config.general.clipboard_restore}
          onChange={(v) => save((c) => (c.general.clipboard_restore = v))}
        />
      </div>

      <div class={`border-t ${borderClass()} pt-4 mt-4`}>
        <h3 class={`text-sm font-semibold ${headingColor()} uppercase tracking-wider mb-2`}>
          Hotkey
        </h3>
        <div
          class={`rounded-lg p-3 text-sm ${
            isDark() ? "bg-gray-800" : "bg-gray-100"
          }`}
        >
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <span class={isDark() ? "text-gray-400" : "text-gray-500"}>
                Push-to-talk:{" "}
              </span>
              {recording() ? (
                <span class="text-yellow-500 animate-pulse text-xs font-medium">
                  Press your shortcut...
                </span>
              ) : (
                <kbd
                  class={`px-2 py-0.5 rounded text-xs font-mono ${
                    isDark()
                      ? "bg-gray-700 text-gray-200"
                      : "bg-gray-200 text-gray-700"
                  }`}
                >
                  {formatHotkey(
                    props.config.general.hotkey.key,
                    props.config.general.hotkey.modifiers
                  )}
                </kbd>
              )}
            </div>
            <div class="flex gap-2">
              <Button
                size="sm"
                variant={recording() ? "danger" : "secondary"}
                onClick={() => {
                  if (recording()) {
                    cancelRecording();
                    return;
                  }
                  setRecording(true);

                  activeTimeout = setTimeout(() => cancelRecording(), 5000);

                  activeHandler = (e: KeyboardEvent) => {
                    e.preventDefault();
                    e.stopPropagation();
                    const key = mapKeyToRdev(e);
                    if (!key) return; // modifier-only, keep listening
                    const mods = modifiersFromEvent(e);
                    cancelRecording();
                    save((c) => {
                      c.general.hotkey = { key, modifiers: mods };
                    });
                  };
                  document.addEventListener("keydown", activeHandler, true);
                }}
              >
                {recording() ? "Cancel" : "Record"}
              </Button>
              <Button
                size="sm"
                variant="secondary"
                onClick={() =>
                  save((c) => {
                    c.general.hotkey = { ...DEFAULT_HOTKEY };
                  })
                }
              >
                Reset
              </Button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
