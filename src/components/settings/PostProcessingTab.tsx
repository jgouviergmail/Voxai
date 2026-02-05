import { createSignal, onMount, Show } from "solid-js";
import type { AppConfig, LanguageInfo, LlmStatus, PipelineTestResult } from "../../types";
import {
  updateSettings,
  checkLlmStatus,
  listSupportedLanguages,
  testReformulation,
  testTranslation,
  testTextPipeline,
} from "../../lib/commands";
import Toggle from "../ui/Toggle";
import Select from "../ui/Select";
import Input from "../ui/Input";
import Button from "../ui/Button";
import { appStore } from "../../lib/stores";

interface PostProcessingTabProps {
  config: AppConfig;
  onUpdate: (config: AppConfig) => void;
}

export default function PostProcessingTab(props: PostProcessingTabProps) {
  const [llmStatus, setLlmStatus] = createSignal<LlmStatus | null>(null);
  const [languages, setLanguages] = createSignal<LanguageInfo[]>([]);
  const [testInput, setTestInput] = createSignal("");
  const [testOutput, setTestOutput] = createSignal("");
  const [pipelineResult, setPipelineResult] = createSignal<PipelineTestResult | null>(null);
  const [testing, setTesting] = createSignal(false);

  const checkStatus = async () => {
    try {
      setLlmStatus(await checkLlmStatus());
    } catch (e) {
      console.error("LLM status check failed:", e);
    }
  };

  onMount(async () => {
    checkStatus();
    try {
      setLanguages(await listSupportedLanguages());
    } catch (e) {
      console.error("Failed to load languages:", e);
    }
  });

  const save = async (updater: (c: AppConfig) => void) => {
    const c = structuredClone(props.config);
    updater(c);
    try {
      await updateSettings(c);
      props.onUpdate(c);
    } catch (e) {
      appStore.showError(String(e));
    }
  };

  const currentStyle = () => {
    const s = props.config.postprocessing.reformulation.style;
    return typeof s === "string" ? s : "Custom";
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
        props.config.postprocessing.translation.target_language,
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
          Text cleanup
        </h3>
        <Toggle
          label="Auto-capitalize"
          description="Capitalize first letter of each sentence"
          checked={props.config.postprocessing.auto_capitalize}
          onChange={(v) => save((c) => (c.postprocessing.auto_capitalize = v))}
        />
        <Toggle
          label="Smart spacing"
          description="Fix spacing around punctuation"
          checked={props.config.postprocessing.smart_spacing}
          onChange={(v) => save((c) => (c.postprocessing.smart_spacing = v))}
        />
      </div>

      {/* LLM Status */}
      <div class={`border-t ${borderClass()} pt-4`}>
        <div class="flex items-center justify-between mb-2">
          <h3 class={`text-sm font-semibold ${headingColor()} uppercase tracking-wider`}>
            LLM Backend
          </h3>
          <Button size="sm" variant="secondary" onClick={checkStatus}>
            Refresh
          </Button>
        </div>
        <Show
          when={llmStatus()}
          fallback={
            <p class="text-xs text-gray-500">Checking LLM status...</p>
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
                  {status().available ? "Connected" : "Unavailable"}
                </span>
              </div>
            </div>
          )}
        </Show>

        <Select
          label="Backend"
          value={props.config.llm.active_backend}
          options={[
            { value: "None", label: "None" },
            { value: "Ollama", label: "Ollama (recommended)" },
            { value: "Local", label: "Local (CPU)" },
          ]}
          onChange={(v) => {
            save((c) => {
              c.llm.active_backend = v as AppConfig["llm"]["active_backend"];
              // Auto-disable LLM features when switching to None
              if (v === "None") {
                c.postprocessing.reformulation.enabled = false;
                c.postprocessing.translation.enabled = false;
              }
            });
            // Re-check LLM status after backend change
            setTimeout(checkStatus, 500);
          }}
        />

        <Show when={props.config.llm.active_backend === "Ollama"}>
          <div class="grid grid-cols-3 gap-2">
            <div class="col-span-2">
              <Input
                label="Host"
                value={props.config.llm.ollama.host}
                onChange={(v) => save((c) => (c.llm.ollama.host = v))}
              />
            </div>
            <Input
              label="Port"
              type="number"
              value={String(props.config.llm.ollama.port)}
              onChange={(v) =>
                save((c) => (c.llm.ollama.port = parseInt(v) || 11434))
              }
            />
          </div>
          <Input
            label="Model"
            value={props.config.llm.ollama.model}
            placeholder="mistral"
            onChange={(v) => save((c) => (c.llm.ollama.model = v))}
          />
        </Show>

        <Show when={props.config.llm.active_backend === "Local"}>
          <div class={`${cardBg()} rounded-lg p-3 text-sm`}>
            <p class={isDark() ? "text-gray-400" : "text-gray-500"}>
              Local LLM uses a GGUF model running on CPU via llama.cpp.
              Download the model from the <strong>Engines</strong> tab.
            </p>
          </div>
        </Show>
      </div>

      {/* Reformulation */}
      <div class={`border-t ${borderClass()} pt-4`}>
        <h3 class={`text-sm font-semibold ${headingColor()} uppercase tracking-wider mb-2`}>
          Reformulation
        </h3>
        <Toggle
          label="Enable reformulation"
          description="Use LLM to reformulate transcribed text"
          checked={props.config.postprocessing.reformulation.enabled}
          disabled={!llmAvailable()}
          onChange={(v) =>
            save((c) => (c.postprocessing.reformulation.enabled = v))
          }
        />
        <Show when={!llmAvailable()}>
          <p class={`text-xs rounded px-2 py-1 mt-1 border ${warningBg()}`}>
            Requires an active LLM backend (configure above)
          </p>
        </Show>
        <Show when={props.config.postprocessing.reformulation.enabled}>
          <Select
            label="Style"
            value={currentStyle()}
            options={[
              { value: "Cleaned", label: "Cleaned (fix grammar)" },
              { value: "Professional", label: "Professional" },
              { value: "Casual", label: "Casual" },
              { value: "Concise", label: "Concise" },
              { value: "Simplified", label: "Simplified" },
              { value: "Structured", label: "Structured" },
            ]}
            onChange={(v) =>
              save(
                (c) =>
                  (c.postprocessing.reformulation.style =
                    v as AppConfig["postprocessing"]["reformulation"]["style"]),
              )
            }
          />
        </Show>
      </div>

      {/* Translation */}
      <div class={`border-t ${borderClass()} pt-4`}>
        <h3 class={`text-sm font-semibold ${headingColor()} uppercase tracking-wider mb-2`}>
          Translation
        </h3>
        <Toggle
          label="Enable translation"
          description="Translate text after transcription"
          checked={props.config.postprocessing.translation.enabled}
          disabled={!llmAvailable()}
          onChange={(v) =>
            save((c) => (c.postprocessing.translation.enabled = v))
          }
        />
        <Show when={!llmAvailable()}>
          <p class={`text-xs rounded px-2 py-1 mt-1 border ${warningBg()}`}>
            Requires an active LLM backend (configure above)
          </p>
        </Show>
        <Show when={props.config.postprocessing.translation.enabled}>
          <Select
            label="Target language"
            value={props.config.postprocessing.translation.target_language}
            options={languages().map((l) => ({
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

      {/* Test zone */}
      <div class={`border-t ${borderClass()} pt-4`}>
        <h3 class={`text-sm font-semibold ${headingColor()} uppercase tracking-wider mb-2`}>
          Test zone
        </h3>
        <textarea
          class={`w-full rounded-lg px-3 py-2 text-sm border resize-none h-20 ${
            isDark()
              ? "bg-gray-800 border-gray-700 text-gray-100 placeholder-gray-600"
              : "bg-white border-gray-300 text-gray-900 placeholder-gray-400"
          }`}
          placeholder="Enter text to test the full post-processing pipeline..."
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
            Test full pipeline
          </Button>
          <Button
            size="sm"
            variant="secondary"
            onClick={handleTestReformulation}
            loading={testing()}
            disabled={!testInput() || !llmStatus()?.available}
          >
            Test reformulation
          </Button>
          <Button
            size="sm"
            variant="secondary"
            onClick={handleTestTranslation}
            loading={testing()}
            disabled={!testInput() || !llmStatus()?.available}
          >
            Test translation
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
              <PipelineStep label="Input" text={result().input} isDark={isDark} />
              <PipelineStep label="Capitalize" text={result().after_capitalize} isDark={isDark} />
              <PipelineStep label="Spacing" text={result().after_spacing} isDark={isDark} />
              <Show when={result().after_reformulation}>
                <PipelineStep label="Reformulation" text={result().after_reformulation!} isDark={isDark} />
              </Show>
              <Show when={result().after_translation}>
                <PipelineStep label="Translation" text={result().after_translation!} isDark={isDark} />
              </Show>
              <PipelineStep label="Substitutions" text={result().after_substitutions} isDark={isDark} />
              <div class={`pt-2 border-t ${isDark() ? "border-gray-700" : "border-gray-200"}`}>
                <span class="font-semibold text-xs uppercase tracking-wider">Final result</span>
                <p class="mt-1">{result().final_text}</p>
              </div>
            </div>
          )}
        </Show>
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
