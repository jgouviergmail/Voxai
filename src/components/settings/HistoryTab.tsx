import { createSignal, onMount, For, Show } from "solid-js";
import type { HistoryEntry } from "../../types";
import { getHistory, clearHistory as clearHistoryCmd } from "../../lib/commands";
import Button from "../ui/Button";
import { appStore } from "../../lib/stores";
import { i18n } from "../../lib/i18n";

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
  const mutedText = () => (isDark() ? "text-white/44" : "text-black/40");

  return (
    <div class="space-y-4">
      <div class="flex items-center justify-between">
        <p class={`text-sm ${mutedText()}`}>
          {entries().length} {entries().length !== 1 ? i18n.t("history.count_other") : i18n.t("history.count_one")}
        </p>
        <Show when={entries().length > 0}>
          <Button size="sm" variant="danger" onClick={handleClear}>
            {i18n.t("history.clear")}
          </Button>
        </Show>
      </div>

      <Show
        when={entries().length > 0}
        fallback={<p class={`${mutedText()} text-sm`}>{i18n.t("history.empty")}</p>}
      >
        <div class="space-y-2">
          <For each={entries()}>
            {(entry) => (
              <div
                class={`rounded-lg p-3 ${
                  isDark()
                    ? "bg-surface-raised border border-border-subtle"
                    : "bg-white border border-border-subtle-lt shadow-card-lt"
                }`}
              >
                <p class="text-sm">{entry.final_text}</p>
                <Show when={entry.raw_text !== entry.final_text}>
                  <p class={`text-xs ${mutedText()} mt-1 italic`}>
                    {i18n.t("history.original")} {entry.raw_text}
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
