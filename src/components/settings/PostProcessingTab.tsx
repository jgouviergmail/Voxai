import { createSignal, createEffect, onMount, onCleanup, Show, For } from "solid-js";
import type { AppConfig, CustomPrompt, LanguageInfo, LlmStatus, PipelineTestResult, PromptPreview } from "../../types";
import {
  checkLlmStatus,
  getPromptPreview,
  listOllamaModels,
  listSupportedLanguages,
  testReformulation,
  testTranslation,
  testTextPipeline,
} from "../../lib/commands";
import { onSettingsUpdated } from "../../lib/events";
import Toggle from "../ui/Toggle";
import Select from "../ui/Select";
import Input from "../ui/Input";
import Button from "../ui/Button";
import { appStore } from "../../lib/stores";
import { i18n } from "../../lib/i18n";
import { BUILTIN_STYLES } from "../../lib/constants";

export default function PostProcessingTab() {
  const [llmStatus, setLlmStatus] = createSignal<LlmStatus | null>(null);
  const [languages, setLanguages] = createSignal<LanguageInfo[]>([]);
  const [ollamaModels, setOllamaModels] = createSignal<string[]>([]);
  const [testInput, setTestInput] = createSignal("");
  const [testOutput, setTestOutput] = createSignal("");
  const [pipelineResult, setPipelineResult] = createSignal<PipelineTestResult | null>(null);
  const [testing, setTesting] = createSignal(false);

  // Prompt editor state
  const [promptPreview, setPromptPreview] = createSignal<PromptPreview | null>(null);
  const [editSystem, setEditSystem] = createSignal("");
  const [editInstruction, setEditInstruction] = createSignal("");
  const [showNewCustom, setShowNewCustom] = createSignal(false);
  const [newCustomName, setNewCustomName] = createSignal("");
  const [newCustomSystem, setNewCustomSystem] = createSignal("");
  const [newCustomInstruction, setNewCustomInstruction] = createSignal("");
  const [editingCustomId, setEditingCustomId] = createSignal<string | null>(null);

  const checkStatus = async () => {
    try {
      setLlmStatus(await checkLlmStatus());
    } catch (e) {
      console.error("LLM status check failed:", e);
    }
  };

  const loadOllamaModels = async () => {
    try {
      setOllamaModels(await listOllamaModels());
    } catch {
      setOllamaModels([]);
    }
  };

  onMount(async () => {
    checkStatus();
    try {
      setLanguages(await listSupportedLanguages());
    } catch (e) {
      console.error("Failed to load languages:", e);
    }
    const unlistenSettings = await onSettingsUpdated(() => {
      checkStatus();
    });
    onCleanup(() => unlistenSettings());
  });

  const save = (updater: (c: AppConfig) => void) => appStore.saveSetting(updater);

  const config = () => appStore.config()!;

  const currentStyle = () => {
    const s = config().postprocessing.reformulation.style;
    return typeof s === "string" ? s : "Custom";
  };

  const currentStyleKey = () => {
    const s = config().postprocessing.reformulation.style;
    if (typeof s === "string") return s;
    return (s as { Custom: string }).Custom;
  };

  const customPrompts = () => config().postprocessing.custom_prompts ?? [];

  const isBuiltinStyle = () => (BUILTIN_STYLES as readonly string[]).includes(currentStyle());

  // Load Ollama models when backend is Ollama and connected
  createEffect(() => {
    if (config().llm.active_backend === "Ollama" && llmStatus()?.available) {
      loadOllamaModels();
    } else {
      setOllamaModels([]);
    }
  });

  // Load prompt preview when style changes
  createEffect(async () => {
    const key = currentStyleKey();
    if (!key) return;
    try {
      const preview = await getPromptPreview(key);
      setPromptPreview(preview);
      setEditSystem(preview.system);
      setEditInstruction(preview.instruction);
    } catch (e) {
      console.error("Failed to load prompt preview:", e);
    }
  });

  const handleSavePromptOverride = () => {
    const style = currentStyle();
    if (!isBuiltinStyle()) return;
    save((c) => {
      if (!c.postprocessing.prompt_overrides) c.postprocessing.prompt_overrides = {};
      c.postprocessing.prompt_overrides[style] = {
        system: editSystem(),
        instruction: editInstruction(),
      };
    });
    setPromptPreview({ system: editSystem(), instruction: editInstruction(), is_modified: true });
  };

  const handleResetPrompt = async () => {
    const style = currentStyle();
    await save((c) => {
      if (c.postprocessing.prompt_overrides) {
        delete c.postprocessing.prompt_overrides[style];
      }
    });
    // Reload preview after save completes so backend returns the default prompt
    try {
      const p = await getPromptPreview(style);
      setPromptPreview(p);
      setEditSystem(p.system);
      setEditInstruction(p.instruction);
    } catch (e) {
      console.error("Failed to reload prompt preview:", e);
    }
  };

  const handleAddCustomPrompt = () => {
    const name = newCustomName().trim();
    const system = newCustomSystem().trim();
    const instruction = newCustomInstruction().trim();
    if (!name || !system || !instruction) return;
    const id = `custom-${Date.now()}`;
    save((c) => {
      if (!c.postprocessing.custom_prompts) c.postprocessing.custom_prompts = [];
      c.postprocessing.custom_prompts.push({ id, name, system, instruction });
    });
    setNewCustomName("");
    setNewCustomSystem("");
    setNewCustomInstruction("");
    setShowNewCustom(false);
  };

  const handleDeleteCustomPrompt = (id: string) => {
    save((c) => {
      c.postprocessing.custom_prompts = (c.postprocessing.custom_prompts ?? []).filter((p) => p.id !== id);
      // If this prompt was active, reset to Cleaned
      const style = c.postprocessing.reformulation.style;
      if (typeof style !== "string" && (style as { Custom: string }).Custom === id) {
        c.postprocessing.reformulation.style = "Cleaned";
      }
    });
    if (editingCustomId() === id) setEditingCustomId(null);
  };

  const handleSaveCustomPrompt = (id: string, name: string, system: string, instruction: string) => {
    save((c) => {
      const idx = (c.postprocessing.custom_prompts ?? []).findIndex((p) => p.id === id);
      if (idx >= 0) {
        c.postprocessing.custom_prompts[idx] = { id, name, system, instruction };
      }
    });
    setEditingCustomId(null);
  };

  const handleTestReformulation = async () => {
    if (!testInput()) return;
    setTesting(true);
    try {
      const result = await testReformulation(testInput());
      setTestOutput(result);
    } catch (e) {
      setTestOutput(`Error: ${e}`);
    }
    setTesting(false);
  };

  const handleTestTranslation = async () => {
    if (!testInput()) return;
    setTesting(true);
    try {
      const result = await testTranslation(
        testInput(),
        config().postprocessing.translation.target_language,
      );
      setTestOutput(result);
    } catch (e) {
      setTestOutput(`Error: ${e}`);
    }
    setTesting(false);
  };

  const llmAvailable = () => llmStatus()?.available === true;

  const isDark = () => appStore.theme() === "dark";
  const cardBg = () => (isDark() ? "bg-gray-800" : "bg-gray-50");
  const borderClass = () =>
    isDark() ? "border-gray-800" : "border-gray-200";
  const headingColor = () =>
    isDark() ? "text-gray-400" : "text-gray-500";
  const warningBg = () =>
    isDark() ? "bg-yellow-900/30 text-yellow-300 border-yellow-800" : "bg-yellow-50 text-yellow-700 border-yellow-200";

  return (
    <div class="space-y-6">
      {/* Basic post-processing */}
      <div>
        <h3 class={`text-sm font-semibold ${headingColor()} uppercase tracking-wider mb-2`}>
          {i18n.t("pp.text_cleanup")}
        </h3>
        <Toggle
          label={i18n.t("pp.auto_capitalize")}
          description={i18n.t("pp.auto_capitalize_desc")}
          checked={config().postprocessing.auto_capitalize}
          onChange={(v) => save((c) => (c.postprocessing.auto_capitalize = v))}
        />
        <Toggle
          label={i18n.t("pp.smart_spacing")}
          description={i18n.t("pp.smart_spacing_desc")}
          checked={config().postprocessing.smart_spacing}
          onChange={(v) => save((c) => (c.postprocessing.smart_spacing = v))}
        />
      </div>

      {/* LLM Status */}
      <div class={`border-t ${borderClass()} pt-4`}>
        <div class="flex items-center justify-between mb-2">
          <h3 class={`text-sm font-semibold ${headingColor()} uppercase tracking-wider`}>
            {i18n.t("pp.llm_backend")}
          </h3>
          <Button size="sm" variant="secondary" onClick={checkStatus}>
            {i18n.t("pp.refresh")}
          </Button>
        </div>
        <Show
          when={llmStatus()}
          fallback={
            <p class="text-xs text-gray-500">{i18n.t("pp.checking_status")}</p>
          }
        >
          {(status) => (
            <div class={`${cardBg()} rounded-lg p-3 text-sm`}>
              <div class="flex items-center gap-2">
                <div
                  class={`w-2 h-2 rounded-full ${
                    status().available ? "bg-green-500" : "bg-red-500"
                  }`}
                />
                <span>
                  {status().backend_name}:{" "}
                  {status().available ? i18n.t("pp.connected") : i18n.t("pp.unavailable")}
                </span>
              </div>
            </div>
          )}
        </Show>

        <Select
          label={i18n.t("pp.backend")}
          value={config().llm.active_backend}
          options={[
            { value: "None", label: i18n.t("pp.backend_none") },
            { value: "Ollama", label: i18n.t("pp.backend_ollama") },
            { value: "Local", label: i18n.t("pp.backend_local") },
          ]}
          onChange={(v) => {
            save((c) => {
              c.llm.active_backend = v as AppConfig["llm"]["active_backend"];
              if (v === "None") {
                c.postprocessing.reformulation.enabled = false;
                c.postprocessing.translation.enabled = false;
              }
            });
          }}
        />

        <Show when={config().llm.active_backend === "Ollama"}>
          <div class="grid grid-cols-3 gap-2">
            <div class="col-span-2">
              <Input
                label={i18n.t("pp.host")}
                value={config().llm.ollama.host}
                onChange={(v) => save((c) => (c.llm.ollama.host = v))}
              />
            </div>
            <Input
              label={i18n.t("pp.port")}
              type="number"
              value={String(config().llm.ollama.port)}
              onChange={(v) =>
                save((c) => (c.llm.ollama.port = parseInt(v) || 11434))
              }
            />
          </div>
          <Show
            when={ollamaModels().length > 0}
            fallback={
              <Input
                label={i18n.t("pp.model")}
                value={config().llm.ollama.model}
                placeholder="mistral"
                onChange={(v) => save((c) => (c.llm.ollama.model = v))}
              />
            }
          >
            <Select
              label={i18n.t("pp.model")}
              value={config().llm.ollama.model}
              options={ollamaModels().map((m) => ({ value: m, label: m }))}
              onChange={(v) => save((c) => (c.llm.ollama.model = v))}
            />
          </Show>
        </Show>

        <Show when={config().llm.active_backend === "Local"}>
          <div class={`${cardBg()} rounded-lg p-3 text-sm`}>
            <p class={isDark() ? "text-gray-400" : "text-gray-500"}>
              {i18n.t("pp.local_info")}
            </p>
          </div>
        </Show>
      </div>

      {/* Translation */}
      <div class={`border-t ${borderClass()} pt-4`}>
        <h3 class={`text-sm font-semibold ${headingColor()} uppercase tracking-wider mb-2`}>
          {i18n.t("pp.translation")}
        </h3>
        <Toggle
          label={i18n.t("pp.enable_translation")}
          description={i18n.t("pp.enable_translation_desc")}
          checked={config().postprocessing.translation.enabled}
          disabled={!llmAvailable()}
          onChange={(v) =>
            save((c) => (c.postprocessing.translation.enabled = v))
          }
        />
        <Show when={!llmAvailable()}>
          <p class={`text-xs rounded px-2 py-1 mt-1 border ${warningBg()}`}>
            {i18n.t("pp.requires_llm")}
          </p>
        </Show>
        <Show when={config().postprocessing.translation.enabled}>
          <Select
            label={i18n.t("pp.target_language")}
            value={config().postprocessing.translation.target_language}
            options={languages()
              .filter((l) => l.code !== "")
              .map((l) => ({
                value: l.code,
                label: l.name,
              }))}
            onChange={(v) =>
              save(
                (c) => (c.postprocessing.translation.target_language = v),
              )
            }
          />
        </Show>
      </div>

      {/* Reformulation */}
      <div class={`border-t ${borderClass()} pt-4`}>
        <h3 class={`text-sm font-semibold ${headingColor()} uppercase tracking-wider mb-2`}>
          {i18n.t("pp.reformulation")}
        </h3>
        <Toggle
          label={i18n.t("pp.enable_reformulation")}
          description={i18n.t("pp.enable_reformulation_desc")}
          checked={config().postprocessing.reformulation.enabled}
          disabled={!llmAvailable()}
          onChange={(v) =>
            save((c) => (c.postprocessing.reformulation.enabled = v))
          }
        />
        <Show when={!llmAvailable()}>
          <p class={`text-xs rounded px-2 py-1 mt-1 border ${warningBg()}`}>
            {i18n.t("pp.requires_llm")}
          </p>
        </Show>
        <Show when={config().postprocessing.reformulation.enabled}>
          {/* Style radio list — full row is clickable */}
          <div class="py-2">
            <label class="block text-sm font-medium mb-1">{i18n.t("pp.style")}</label>
            <div class={`rounded-lg border overflow-hidden ${
              isDark() ? "border-gray-700" : "border-gray-200"
            }`}>
              <For each={[
                { value: "Cleaned", label: i18n.t("pp.style_cleaned") },
                { value: "Professional", label: i18n.t("pp.style_professional") },
                { value: "Casual", label: i18n.t("pp.style_casual") },
                { value: "Concise", label: i18n.t("pp.style_concise") },
                { value: "Simplified", label: i18n.t("pp.style_simplified") },
                { value: "Structured", label: i18n.t("pp.style_structured") },
                ...customPrompts().map((p) => ({ value: p.id, label: p.name })),
              ]}>
                {(opt) => {
                  const isActive = () => currentStyleKey() === opt.value;
                  const isBuiltin = (BUILTIN_STYLES as readonly string[]).includes(opt.value);
                  return (
                    <button
                      type="button"
                      class={`w-full text-left px-3 py-2 text-sm flex items-center gap-2 border-b last:border-b-0 transition-colors cursor-pointer ${
                        isDark()
                          ? `border-gray-700 hover:bg-gray-700 ${isActive() ? "bg-gray-700" : ""}`
                          : `border-gray-100 hover:bg-gray-100 ${isActive() ? "bg-blue-50" : ""}`
                      }`}
                      onClick={() => {
                        save((c) => {
                          c.postprocessing.reformulation.style = isBuiltin
                            ? (opt.value as AppConfig["postprocessing"]["reformulation"]["style"])
                            : { Custom: opt.value };
                        });
                      }}
                    >
                      <span class={`w-3 h-3 rounded-full border-2 flex-shrink-0 ${
                        isActive()
                          ? "border-blue-500 bg-blue-500"
                          : isDark() ? "border-gray-500" : "border-gray-400"
                      }`} />
                      <span>{opt.label}</span>
                    </button>
                  );
                }}
              </For>
            </div>
          </div>

          {/* Prompt viewer — only shown for custom styles */}
          <Show when={!isBuiltinStyle() && promptPreview()}>
            <div class={`${cardBg()} rounded-lg p-3 mt-2 space-y-2`}>
              <span class={`text-xs font-semibold uppercase tracking-wider ${headingColor()}`}>
                {i18n.t("pp.prompt")}
              </span>
              <div class="space-y-1 text-sm">
                <div>
                  <span class={`text-xs font-medium ${headingColor()}`}>{i18n.t("pp.system_prompt")}</span>
                  <p class="mt-0.5">{promptPreview()?.system}</p>
                </div>
                <div>
                  <span class={`text-xs font-medium ${headingColor()}`}>{i18n.t("pp.instruction")}</span>
                  <p class="mt-0.5">{promptPreview()?.instruction}</p>
                </div>
              </div>
            </div>
          </Show>

          {/* Custom styles CRUD */}
          <div class={`border-t ${borderClass()} pt-3 mt-3`}>
            <div class="flex items-center justify-between mb-2">
              <span class={`text-xs font-semibold uppercase tracking-wider ${headingColor()}`}>
                {i18n.t("pp.custom_styles")}
              </span>
              <Button size="sm" variant="secondary" onClick={() => setShowNewCustom(!showNewCustom())}>
                {showNewCustom() ? i18n.t("general.cancel") : i18n.t("pp.add_custom")}
              </Button>
            </div>

            {/* New custom prompt form */}
            <Show when={showNewCustom()}>
              <div class={`${cardBg()} rounded-lg p-3 space-y-2 mb-2`}>
                <Input label={i18n.t("pp.name")} value={newCustomName()} onChange={setNewCustomName} placeholder="e.g. Poetic" />
                <div>
                  <label class="block text-xs font-medium mb-1">{i18n.t("pp.system_prompt")}</label>
                  <textarea
                    class={`w-full rounded-lg px-3 py-2 text-sm border resize-none h-16 ${
                      isDark()
                        ? "bg-gray-900 border-gray-700 text-gray-100"
                        : "bg-white border-gray-300 text-gray-900"
                    }`}
                    value={newCustomSystem()}
                    onInput={(e) => setNewCustomSystem(e.currentTarget.value)}
                  />
                </div>
                <div>
                  <label class="block text-xs font-medium mb-1">{i18n.t("pp.instruction")}</label>
                  <textarea
                    class={`w-full rounded-lg px-3 py-2 text-sm border resize-none h-16 ${
                      isDark()
                        ? "bg-gray-900 border-gray-700 text-gray-100"
                        : "bg-white border-gray-300 text-gray-900"
                    }`}
                    value={newCustomInstruction()}
                    onInput={(e) => setNewCustomInstruction(e.currentTarget.value)}
                  />
                </div>
                <Button
                  size="sm"
                  onClick={handleAddCustomPrompt}
                  disabled={!newCustomName().trim() || !newCustomSystem().trim() || !newCustomInstruction().trim()}
                >
                  {i18n.t("pp.create")}
                </Button>
              </div>
            </Show>

            {/* Existing custom prompts list */}
            <For each={customPrompts()}>
              {(cp) => (
                <div class={`${cardBg()} rounded-lg p-3 mb-2`}>
                  <Show when={editingCustomId() === cp.id} fallback={
                    <div class="flex items-center justify-between">
                      <div>
                        <span class="text-sm font-medium">{cp.name}</span>
                        <p class={`text-xs mt-0.5 ${headingColor()}`}>{cp.instruction.slice(0, 60)}...</p>
                      </div>
                      <div class="flex gap-1">
                        <Button size="sm" variant="secondary" onClick={() => setEditingCustomId(cp.id)}>
                          {i18n.t("pp.edit")}
                        </Button>
                        <Button size="sm" variant="danger" onClick={() => handleDeleteCustomPrompt(cp.id)}>
                          {i18n.t("pp.delete")}
                        </Button>
                      </div>
                    </div>
                  }>
                    <CustomPromptEditor
                      prompt={cp}
                      isDark={isDark}
                      onSave={(name, system, instruction) => handleSaveCustomPrompt(cp.id, name, system, instruction)}
                      onCancel={() => setEditingCustomId(null)}
                    />
                  </Show>
                </div>
              )}
            </For>
            <Show when={customPrompts().length === 0 && !showNewCustom()}>
              <p class={`text-xs ${headingColor()}`}>{i18n.t("pp.no_custom")}</p>
            </Show>
          </div>
        </Show>
      </div>

      {/* Test zone */}
      <div class={`border-t ${borderClass()} pt-4`}>
        <h3 class={`text-sm font-semibold ${headingColor()} uppercase tracking-wider mb-2`}>
          {i18n.t("pp.test_zone")}
        </h3>
        <textarea
          class={`w-full rounded-lg px-3 py-2 text-sm border resize-none h-20 ${
            isDark()
              ? "bg-gray-800 border-gray-700 text-gray-100 placeholder-gray-600"
              : "bg-white border-gray-300 text-gray-900 placeholder-gray-400"
          }`}
          placeholder={i18n.t("pp.test_placeholder")}
          value={testInput()}
          onInput={(e) => setTestInput(e.currentTarget.value)}
        />
        <div class="flex gap-2 mt-2">
          <Button
            size="sm"
            onClick={async () => {
              if (!testInput()) return;
              setTesting(true);
              setPipelineResult(null);
              setTestOutput("");
              try {
                const result = await testTextPipeline(testInput());
                setPipelineResult(result);
              } catch (e) {
                setTestOutput(`Error: ${e}`);
              }
              setTesting(false);
            }}
            loading={testing()}
            disabled={!testInput()}
          >
            {i18n.t("pp.test_pipeline")}
          </Button>
          <Button
            size="sm"
            variant="secondary"
            onClick={handleTestReformulation}
            loading={testing()}
            disabled={!testInput() || !llmStatus()?.available}
          >
            {i18n.t("pp.test_reformulation")}
          </Button>
          <Button
            size="sm"
            variant="secondary"
            onClick={handleTestTranslation}
            loading={testing()}
            disabled={!testInput() || !llmStatus()?.available}
          >
            {i18n.t("pp.test_translation")}
          </Button>
          <Button
            size="sm"
            variant="secondary"
            onClick={() => {
              setTestInput("");
              setTestOutput("");
              setPipelineResult(null);
            }}
            disabled={(!testInput() && !testOutput() && !pipelineResult()) || testing()}
          >
            {i18n.t("pp.reset")}
          </Button>
        </div>
        <Show when={testOutput()}>
          <div class={`${cardBg()} rounded-lg p-3 mt-2 text-sm`}>
            {testOutput()}
          </div>
        </Show>
        <Show when={pipelineResult()}>
          {(result) => (
            <div class={`${cardBg()} rounded-lg p-3 mt-2 text-sm space-y-2`}>
              <PipelineStep label={i18n.t("pp.step_input")} text={result().input} isDark={isDark} />
              <PipelineStep label={i18n.t("pp.step_capitalize")} text={result().after_capitalize} isDark={isDark} />
              <PipelineStep label={i18n.t("pp.step_spacing")} text={result().after_spacing} isDark={isDark} />
              <Show when={result().after_reformulation}>
                <PipelineStep label={i18n.t("pp.step_reformulation")} text={result().after_reformulation!} isDark={isDark} />
              </Show>
              <Show when={result().after_translation}>
                <PipelineStep label={i18n.t("pp.step_translation")} text={result().after_translation!} isDark={isDark} />
              </Show>
              <PipelineStep label={i18n.t("pp.step_substitutions")} text={result().after_substitutions} isDark={isDark} />
              <div class={`pt-2 border-t ${isDark() ? "border-gray-700" : "border-gray-200"}`}>
                <span class="font-semibold text-xs uppercase tracking-wider">{i18n.t("pp.final_result")}</span>
                <p class="mt-1">{result().final_text}</p>
              </div>
            </div>
          )}
        </Show>
      </div>
    </div>
  );
}

function CustomPromptEditor(props: {
  prompt: CustomPrompt;
  isDark: () => boolean;
  onSave: (name: string, system: string, instruction: string) => void;
  onCancel: () => void;
}) {
  const [name, setName] = createSignal(props.prompt.name);
  const [system, setSystem] = createSignal(props.prompt.system);
  const [instruction, setInstruction] = createSignal(props.prompt.instruction);

  return (
    <div class="space-y-2">
      <Input label={i18n.t("pp.name")} value={name()} onChange={setName} />
      <div>
        <label class="block text-xs font-medium mb-1">{i18n.t("pp.system_prompt")}</label>
        <textarea
          class={`w-full rounded-lg px-3 py-2 text-sm border resize-none h-16 ${
            props.isDark()
              ? "bg-gray-900 border-gray-700 text-gray-100"
              : "bg-white border-gray-300 text-gray-900"
          }`}
          value={system()}
          onInput={(e) => setSystem(e.currentTarget.value)}
        />
      </div>
      <div>
        <label class="block text-xs font-medium mb-1">{i18n.t("pp.instruction")}</label>
        <textarea
          class={`w-full rounded-lg px-3 py-2 text-sm border resize-none h-16 ${
            props.isDark()
              ? "bg-gray-900 border-gray-700 text-gray-100"
              : "bg-white border-gray-300 text-gray-900"
          }`}
          value={instruction()}
          onInput={(e) => setInstruction(e.currentTarget.value)}
        />
      </div>
      <div class="flex gap-1">
        <Button size="sm" onClick={() => props.onSave(name(), system(), instruction())}>
          {i18n.t("pp.save")}
        </Button>
        <Button size="sm" variant="secondary" onClick={props.onCancel}>
          {i18n.t("general.cancel")}
        </Button>
      </div>
    </div>
  );
}

function PipelineStep(props: { label: string; text: string; isDark: () => boolean }) {
  return (
    <div>
      <span
        class={`text-xs font-semibold uppercase tracking-wider ${
          props.isDark() ? "text-gray-500" : "text-gray-400"
        }`}
      >
        {props.label}
      </span>
      <p class="mt-0.5">{props.text}</p>
    </div>
  );
}
