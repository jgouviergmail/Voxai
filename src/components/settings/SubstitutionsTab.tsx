import { createSignal, For, Show } from "solid-js";
import type { SubstitutionRule } from "../../types";
import {
  addSubstitution,
  deleteSubstitution,
} from "../../lib/commands";
import Button from "../ui/Button";
import Section from "../ui/Section";
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
  const mutedText = () => (isDark() ? "text-white/44" : "text-black/40");
  const inputClass = () =>
    `rounded-md px-3 py-2 text-sm border ${
      isDark()
        ? "bg-surface-raised border-border-default text-white/92 placeholder:text-white/30"
        : "bg-white border-border-default-lt text-black/88 placeholder:text-black/30"
    }`;

  return (
    <div class="space-y-4">
      <p class={`text-sm ${mutedText()}`}>
        {i18n.t("sub.description")}
      </p>

      {/* Add new rule */}
      <Section title={i18n.t("sub.add")}>
        <div class="space-y-3">
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
            <label class={`flex items-center gap-2 text-sm ${isDark() ? "text-white/44" : "text-black/40"}`}>
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
      </Section>

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
                class={`rounded-lg px-3 py-2 flex items-center justify-between ${
                  isDark()
                    ? "bg-white/5 border border-border-subtle"
                    : "bg-surface-base-light border border-border-subtle-lt"
                }`}
              >
                <div class="flex items-center gap-2 text-sm min-w-0" data-selectable>
                  <code class={isDark() ? "text-red-400" : "text-red-600"}>{rule.from}</code>
                  <span class={mutedText()}>&rarr;</span>
                  <code class={isDark() ? "text-emerald-400" : "text-emerald-600"}>{rule.to || i18n.t("sub.remove")}</code>
                  <Show when={rule.case_sensitive}>
                    <span class="text-xs text-white/44">[Aa]</span>
                  </Show>
                </div>
                <button
                  class={`${isDark() ? "text-white/44" : "text-black/40"} hover:text-red-400 transition-colors ml-2 text-sm`}
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
        <div
          class={`rounded-lg p-4 space-y-2 ${
            isDark()
              ? "bg-white/5 border border-border-subtle"
              : "bg-surface-base-light border border-border-subtle-lt"
          }`}
        >
          <h3 class="text-sm font-semibold">{i18n.t("sub.preview")}</h3>
          <input
            class={inputClass()}
            placeholder={i18n.t("sub.preview_placeholder")}
            value={previewInput()}
            onInput={(e) => setPreviewInput(e.currentTarget.value)}
            style={{ width: "100%" }}
          />
          <Show when={previewInput()}>
            <div class={`text-sm mt-1 ${isDark() ? "text-white/92" : "text-black/88"}`}>
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
