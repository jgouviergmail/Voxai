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
import Section from "../ui/Section";
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

  return (
    <div class="space-y-4">
      {/* Text Cleanup */}
      <Section title={i18n.t("pp.text_cleanup")}>
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
      </Section>

      {/* LLM Backend */}
      <Section
        title={i18n.t("pp.llm_backend")}
        action={
          <Button size="sm" variant="secondary" onClick={checkStatus}>
            {i18n.t("pp.refresh")}
          </Button>
        }
      >
        <Show
          when={llmStatus()}
          fallback={
            <p class={`text-xs ${isDark() ? "text-white/44" : "text-black/40"}`}>{i18n.t("pp.checking_status")}</p>
          }
        >
          {(status) => (
            <div class={`rounded-lg p-3 text-sm ${isDark() ? "bg-white/5" : "bg-surface-base-light"}`}>
              <div class="flex items-center gap-2">
                <div
                  class={`w-2 h-2 rounded-full ${
                    status().available ? "bg-emerald-400" : "bg-red-400"
                  }`}
                />
                <span class={isDark() ? "text-white/92" : "text-black/88"}>
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
          <div class={`rounded-lg p-3 text-sm ${isDark() ? "bg-white/5" : "bg-surface-base-light"}`}>
            <p class={isDark() ? "text-white/64" : "text-black/60"}>
              {i18n.t("pp.local_info")}
            </p>
          </div>
        </Show>
      </Section>

      {/* Translation */}
      <Section title={i18n.t("pp.translation")}>
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
          <p class={`text-xs rounded px-2 py-1 mt-1 border ${
            isDark()
              ? "bg-amber-500/10 text-amber-400 border-amber-500/20"
              : "bg-amber-50 text-amber-600 border-amber-200"
          }`}>
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
      </Section>

      {/* Reformulation */}
      <Section title={i18n.t("pp.reformulation")}>
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
          <p class={`text-xs rounded px-2 py-1 mt-1 border ${
            isDark()
              ? "bg-amber-500/10 text-amber-400 border-amber-500/20"
              : "bg-amber-50 text-amber-600 border-amber-200"
          }`}>
            {i18n.t("pp.requires_llm")}
          </p>
        </Show>
        <Show when={config().postprocessing.reformulation.enabled}>
          {/* Style radio list -- full row is clickable */}
          <div class="py-2">
            <label class={`block text-sm font-medium mb-1 ${isDark() ? "text-white/92" : "text-black/88"}`}>{i18n.t("pp.style")}</label>
            <div class={`rounded-lg border overflow-hidden ${
              isDark() ? "border-border-default" : "border-border-default-lt"
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
                          ? `border-border-subtle hover:bg-white/5 ${isActive() ? "bg-accent-muted" : ""}`
                          : `border-border-subtle-lt hover:bg-black/3 ${isActive() ? "bg-blue-50" : ""}`
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
                          ? "border-accent bg-accent"
                          : isDark() ? "border-white/20" : "border-black/20"
                      }`} />
                      <span class={isDark() ? "text-white/92" : "text-black/88"}>{opt.label}</span>
                    </button>
                  );
                }}
              </For>
            </div>
          </div>

          {/* Prompt viewer -- only shown for custom styles */}
          <Show when={!isBuiltinStyle() && promptPreview()}>
            <div class={`rounded-lg p-3 mt-2 space-y-2 ${isDark() ? "bg-white/5" : "bg-surface-base-light"}`}>
              <span class={`text-xs font-semibold uppercase tracking-wider ${isDark() ? "text-white/44" : "text-black/40"}`}>
                {i18n.t("pp.prompt")}
              </span>
              <div class="space-y-1 text-sm">
                <div>
                  <span class={`text-xs font-medium ${isDark() ? "text-white/44" : "text-black/40"}`}>{i18n.t("pp.system_prompt")}</span>
                  <p class={`mt-0.5 ${isDark() ? "text-white/92" : "text-black/88"}`}>{promptPreview()?.system}</p>
                </div>
                <div>
                  <span class={`text-xs font-medium ${isDark() ? "text-white/44" : "text-black/40"}`}>{i18n.t("pp.instruction")}</span>
                  <p class={`mt-0.5 ${isDark() ? "text-white/92" : "text-black/88"}`}>{promptPreview()?.instruction}</p>
                </div>
              </div>
            </div>
          </Show>

          {/* Custom styles CRUD */}
          <div class={`border-t pt-3 mt-3 ${isDark() ? "border-border-default" : "border-border-default-lt"}`}>
            <div class="flex items-center justify-between mb-2">
              <span class={`text-xs font-semibold uppercase tracking-wider ${isDark() ? "text-white/44" : "text-black/40"}`}>
                {i18n.t("pp.custom_styles")}
              </span>
              <Button size="sm" variant="secondary" onClick={() => setShowNewCustom(!showNewCustom())}>
                {showNewCustom() ? i18n.t("general.cancel") : i18n.t("pp.add_custom")}
              </Button>
            </div>

            {/* New custom prompt form */}
            <Show when={showNewCustom()}>
              <div class={`rounded-lg p-3 space-y-2 mb-2 ${isDark() ? "bg-white/5" : "bg-surface-base-light"}`}>
                <Input label={i18n.t("pp.name")} value={newCustomName()} onChange={setNewCustomName} placeholder="e.g. Poetic" />
                <div>
                  <label class={`block text-xs font-medium mb-1 ${isDark() ? "text-white/92" : "text-black/88"}`}>{i18n.t("pp.system_prompt")}</label>
                  <textarea
                    class={`w-full rounded-md px-3 py-2 text-sm border resize-none h-16 focus:outline-none focus:ring-2 focus:ring-accent-glow focus:border-accent ${
                      isDark()
                        ? "bg-surface-base border-border-default text-white/92"
                        : "bg-white border-border-default-lt text-black/88"
                    }`}
                    value={newCustomSystem()}
                    onInput={(e) => setNewCustomSystem(e.currentTarget.value)}
                  />
                </div>
                <div>
                  <label class={`block text-xs font-medium mb-1 ${isDark() ? "text-white/92" : "text-black/88"}`}>{i18n.t("pp.instruction")}</label>
                  <textarea
                    class={`w-full rounded-md px-3 py-2 text-sm border resize-none h-16 focus:outline-none focus:ring-2 focus:ring-accent-glow focus:border-accent ${
                      isDark()
                        ? "bg-surface-base border-border-default text-white/92"
                        : "bg-white border-border-default-lt text-black/88"
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
                <div class={`rounded-lg p-3 mb-2 ${isDark() ? "bg-white/5" : "bg-surface-base-light"}`}>
                  <Show when={editingCustomId() === cp.id} fallback={
                    <div class="flex items-center justify-between">
                      <div>
                        <span class={`text-sm font-medium ${isDark() ? "text-white/92" : "text-black/88"}`}>{cp.name}</span>
                        <p class={`text-xs mt-0.5 ${isDark() ? "text-white/44" : "text-black/40"}`}>{cp.instruction.slice(0, 60)}...</p>
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
              <p class={`text-xs ${isDark() ? "text-white/44" : "text-black/40"}`}>{i18n.t("pp.no_custom")}</p>
            </Show>
          </div>
        </Show>
      </Section>

      {/* Test Zone */}
      <Section title={i18n.t("pp.test_zone")}>
        <textarea
          class={`w-full rounded-md px-3 py-2 text-sm border resize-none h-20 focus:outline-none focus:ring-2 focus:ring-accent-glow focus:border-accent ${
            isDark()
              ? "bg-surface-base border-border-default text-white/92 placeholder-white/20"
              : "bg-white border-border-default-lt text-black/88 placeholder-black/30"
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
          <div class={`rounded-lg p-3 mt-2 text-sm ${isDark() ? "bg-white/5 text-white/92" : "bg-surface-base-light text-black/88"}`}>
            {testOutput()}
          </div>
        </Show>
        <Show when={pipelineResult()}>
          {(result) => (
            <div class={`rounded-lg p-3 mt-2 text-sm space-y-2 ${isDark() ? "bg-white/5" : "bg-surface-base-light"}`}>
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
              <div class={`pt-2 border-t ${isDark() ? "border-border-subtle" : "border-border-subtle-lt"}`}>
                <span class={`font-semibold text-xs uppercase tracking-wider ${isDark() ? "text-white/64" : "text-black/60"}`}>{i18n.t("pp.final_result")}</span>
                <p class={`mt-1 ${isDark() ? "text-white/92" : "text-black/88"}`}>{result().final_text}</p>
              </div>
            </div>
          )}
        </Show>
      </Section>
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
        <label class={`block text-xs font-medium mb-1 ${props.isDark() ? "text-white/92" : "text-black/88"}`}>{i18n.t("pp.system_prompt")}</label>
        <textarea
          class={`w-full rounded-md px-3 py-2 text-sm border resize-none h-16 focus:outline-none focus:ring-2 focus:ring-accent-glow focus:border-accent ${
            props.isDark()
              ? "bg-surface-base border-border-default text-white/92"
              : "bg-white border-border-default-lt text-black/88"
          }`}
          value={system()}
          onInput={(e) => setSystem(e.currentTarget.value)}
        />
      </div>
      <div>
        <label class={`block text-xs font-medium mb-1 ${props.isDark() ? "text-white/92" : "text-black/88"}`}>{i18n.t("pp.instruction")}</label>
        <textarea
          class={`w-full rounded-md px-3 py-2 text-sm border resize-none h-16 focus:outline-none focus:ring-2 focus:ring-accent-glow focus:border-accent ${
            props.isDark()
              ? "bg-surface-base border-border-default text-white/92"
              : "bg-white border-border-default-lt text-black/88"
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
          props.isDark() ? "text-white/44" : "text-black/40"
        }`}
      >
        {props.label}
      </span>
      <p class={`mt-0.5 ${props.isDark() ? "text-white/92" : "text-black/88"}`}>{props.text}</p>
    </div>
  );
}
