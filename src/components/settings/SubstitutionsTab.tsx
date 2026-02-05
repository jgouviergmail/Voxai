import { createSignal, For, Show } from "solid-js";
import type { AppConfig, SubstitutionRule } from "../../types";
import {
  addSubstitution,
  deleteSubstitution,
} from "../../lib/commands";
import Button from "../ui/Button";
import Toggle from "../ui/Toggle";
import { appStore } from "../../lib/stores";

interface SubstitutionsTabProps {
  config: AppConfig;
  onUpdate: (config: AppConfig) => void;
}

export default function SubstitutionsTab(props: SubstitutionsTabProps) {
  const [newFrom, setNewFrom] = createSignal("");
  const [newTo, setNewTo] = createSignal("");
  const [newCaseSensitive, setNewCaseSensitive] = createSignal(false);
  const [previewInput, setPreviewInput] = createSignal("");

  const rules = () => props.config.postprocessing.substitutions;

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
      const c = structuredClone(props.config);
      c.postprocessing.substitutions.push(rule);
      props.onUpdate(c);
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
      const c = structuredClone(props.config);
      c.postprocessing.substitutions.splice(index, 1);
      props.onUpdate(c);
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
        Substitutions replace specific words or phrases in your transcription.
        They run after all other post-processing (including LLM reformulation).
      </p>

      {/* Add new rule */}
      <div class={`${cardBg()} rounded-lg p-4 space-y-3`}>
        <h3 class="text-sm font-semibold">Add substitution</h3>
        <div class="grid grid-cols-2 gap-2">
          <input
            class={inputClass()}
            placeholder="Replace this..."
            value={newFrom()}
            onInput={(e) => setNewFrom(e.currentTarget.value)}
          />
          <input
            class={inputClass()}
            placeholder="With this..."
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
            Case sensitive
          </label>
          <Button size="sm" onClick={handleAdd} disabled={!newFrom().trim()}>
            Add
          </Button>
        </div>
      </div>

      {/* Rules list */}
      <Show
        when={rules().length > 0}
        fallback={
          <p class={`${mutedText()} text-sm`}>No substitutions configured.</p>
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
                  <code class={isDark() ? "text-green-400" : "text-green-600"}>{rule.to || "(remove)"}</code>
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
          <h3 class="text-sm font-semibold">Preview</h3>
          <input
            class={inputClass()}
            placeholder="Enter text to preview substitutions..."
            value={previewInput()}
            onInput={(e) => setPreviewInput(e.currentTarget.value)}
            style={{ width: "100%" }}
          />
          <Show when={previewInput()}>
            <div class={`text-sm mt-1 ${isDark() ? "text-gray-300" : "text-gray-700"}`}>
              <span class={mutedText()}>Result: </span>
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
