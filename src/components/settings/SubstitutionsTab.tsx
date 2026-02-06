import { createSignal, For, Show } from "solid-js";
import type { SubstitutionRule } from "../../types";
import {
  addSubstitution,
  deleteSubstitution,
} from "../../lib/commands";
import Button from "../ui/Button";
import { appStore } from "../../lib/stores";
import { i18n } from "../../lib/i18n";

export default function SubstitutionsTab() {
  const [newFrom, setNewFrom] = createSignal("");
  const [newTo, setNewTo] = createSignal("");
  const [newCaseSensitive, setNewCaseSensitive] = createSignal(false);
  const [previewInput, setPreviewInput] = createSignal("");

  const rules = () => appStore.config()?.postprocessing.substitutions ?? [];

  const handleAdd = async () => {
    const from = newFrom().trim();
    const to = newTo().trim();
    if (!from) return;

    const rule: SubstitutionRule = {
      from,
      to,
      case_sensitive: newCaseSensitive(),
    };

    try {
      await addSubstitution(rule);
      // Store is refreshed automatically by the settings-updated event
      setNewFrom("");
      setNewTo("");
      setNewCaseSensitive(false);
    } catch (e) {
      appStore.showError(String(e));
    }
  };

  const handleDelete = async (index: number) => {
    try {
      await deleteSubstitution(index);
      // Store is refreshed automatically by the settings-updated event
    } catch (e) {
      appStore.showError(String(e));
    }
  };

  const isDark = () => appStore.theme() === "dark";
  const cardBg = () => (isDark() ? "bg-gray-800" : "bg-gray-50");
  const mutedText = () => (isDark() ? "text-gray-400" : "text-gray-500");
  const inputClass = () =>
    `rounded-lg px-3 py-2 text-sm border ${
      isDark()
        ? "bg-gray-800 border-gray-700 text-gray-100 placeholder-gray-600"
        : "bg-white border-gray-300 text-gray-900 placeholder-gray-400"
    }`;

  return (
    <div class="space-y-4">
      <p class={`text-sm ${mutedText()}`}>
        {i18n.t("sub.description")}
      </p>

      {/* Add new rule */}
      <div class={`${cardBg()} rounded-lg p-4 space-y-3`}>
        <h3 class="text-sm font-semibold">{i18n.t("sub.add")}</h3>
        <div class="grid grid-cols-2 gap-2">
          <input
            class={inputClass()}
            placeholder={i18n.t("sub.from_placeholder")}
            value={newFrom()}
            onInput={(e) => setNewFrom(e.currentTarget.value)}
          />
          <input
            class={inputClass()}
            placeholder={i18n.t("sub.to_placeholder")}
            value={newTo()}
            onInput={(e) => setNewTo(e.currentTarget.value)}
          />
        </div>
        <div class="flex items-center justify-between">
          <label class={`flex items-center gap-2 text-sm ${isDark() ? "text-gray-400" : "text-gray-500"}`}>
            <input
              type="checkbox"
              checked={newCaseSensitive()}
              onChange={(e) => setNewCaseSensitive(e.currentTarget.checked)}
              class="rounded"
            />
            {i18n.t("sub.case_sensitive")}
          </label>
          <Button size="sm" onClick={handleAdd} disabled={!newFrom().trim()}>
            {i18n.t("sub.add_button")}
          </Button>
        </div>
      </div>

      {/* Rules list */}
      <Show
        when={rules().length > 0}
        fallback={
          <p class={`${mutedText()} text-sm`}>{i18n.t("sub.none")}</p>
        }
      >
        <div class="space-y-1">
          <For each={rules()}>
            {(rule, index) => (
              <div
                class={`${cardBg()} rounded-lg px-3 py-2 flex items-center justify-between`}
              >
                <div class="flex items-center gap-2 text-sm min-w-0" data-selectable>
                  <code class={isDark() ? "text-red-400" : "text-red-600"}>{rule.from}</code>
                  <span class={mutedText()}>&rarr;</span>
                  <code class={isDark() ? "text-green-400" : "text-green-600"}>{rule.to || i18n.t("sub.remove")}</code>
                  <Show when={rule.case_sensitive}>
                    <span class="text-xs text-gray-600">[Aa]</span>
                  </Show>
                </div>
                <button
                  class="text-gray-500 hover:text-red-400 transition-colors ml-2 text-sm"
                  onClick={() => handleDelete(index())}
                  title="Delete"
                >
                  &times;
                </button>
              </div>
            )}
          </For>
        </div>
      </Show>

      {/* Test preview */}
      <Show when={rules().length > 0}>
        <div class={`${cardBg()} rounded-lg p-4 space-y-2`}>
          <h3 class="text-sm font-semibold">{i18n.t("sub.preview")}</h3>
          <input
            class={inputClass()}
            placeholder={i18n.t("sub.preview_placeholder")}
            value={previewInput()}
            onInput={(e) => setPreviewInput(e.currentTarget.value)}
            style={{ width: "100%" }}
          />
          <Show when={previewInput()}>
            <div class={`text-sm mt-1 ${isDark() ? "text-gray-300" : "text-gray-700"}`}>
              <span class={mutedText()}>{i18n.t("sub.result")}</span>
              {applySubstitutionsLocally(previewInput(), rules())}
            </div>
          </Show>
        </div>
      </Show>
    </div>
  );
}

function applySubstitutionsLocally(text: string, rules: SubstitutionRule[]): string {
  let result = text;
  for (const rule of rules) {
    if (rule.case_sensitive) {
      result = result.split(rule.from).join(rule.to);
    } else {
      result = result.replace(new RegExp(escapeRegex(rule.from), "gi"), rule.to);
    }
  }
  return result;
}

function escapeRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
