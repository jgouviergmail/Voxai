import { createSignal, onMount, For, Show } from "solid-js";
import type { HistoryEntry } from "../../types";
import { getHistory, clearHistory as clearHistoryCmd } from "../../lib/commands";
import Button from "../ui/Button";
import { appStore } from "../../lib/stores";

export default function HistoryTab() {
  const [entries, setEntries] = createSignal<HistoryEntry[]>([]);

  onMount(async () => {
    try {
      setEntries(await getHistory());
    } catch (e) {
      console.error("Failed to load history:", e);
    }
  });

  const handleClear = async () => {
    try {
      await clearHistoryCmd();
      setEntries([]);
    } catch (e) {
      appStore.showError(String(e));
    }
  };

  const isDark = () => appStore.theme() === "dark";
  const cardBg = () => (isDark() ? "bg-gray-800" : "bg-gray-50");
  const mutedText = () => (isDark() ? "text-gray-400" : "text-gray-500");

  return (
    <div class="space-y-4">
      <div class="flex items-center justify-between">
        <p class={`text-sm ${mutedText()}`}>
          {entries().length} transcription{entries().length !== 1 ? "s" : ""}
        </p>
        <Show when={entries().length > 0}>
          <Button size="sm" variant="danger" onClick={handleClear}>
            Clear all
          </Button>
        </Show>
      </div>

      <Show
        when={entries().length > 0}
        fallback={<p class={`${mutedText()} text-sm`}>No history yet.</p>}
      >
        <div class="space-y-2">
          <For each={entries()}>
            {(entry) => (
              <div class={`${cardBg()} rounded-lg p-3`}>
                <p class="text-sm">{entry.final_text}</p>
                <Show when={entry.raw_text !== entry.final_text}>
                  <p class={`text-xs ${mutedText()} mt-1 italic`}>
                    Original: {entry.raw_text}
                  </p>
                </Show>
                <div class={`flex gap-4 mt-2 text-xs ${mutedText()}`}>
                  <span>{entry.engine}</span>
                  <span>{entry.duration_ms}ms</span>
                  <span>
                    {new Date(entry.created_at).toLocaleString()}
                  </span>
                </div>
              </div>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
}
