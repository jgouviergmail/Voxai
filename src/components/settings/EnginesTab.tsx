import { createSignal, onMount, onCleanup, For, Show } from "solid-js";
import type { EngineInfo, DownloadProgress } from "../../types";
import {
  listEngines,
  downloadModel,
  deleteModel,
  setActiveModel,
  cancelDownload,
} from "../../lib/commands";
import { onDownloadProgress, onSettingsUpdated } from "../../lib/events";
import Button from "../ui/Button";
import ProgressBar from "../ui/ProgressBar";
import Section from "../ui/Section";
import { appStore } from "../../lib/stores";
import { i18n } from "../../lib/i18n";

export default function EnginesTab() {
  const [engines, setEngines] = createSignal<EngineInfo[]>([]);
  const [downloading, setDownloading] = createSignal<string | null>(null);
  const [progress, setProgress] = createSignal<DownloadProgress | null>(null);

  const loadEngines = async () => {
    try {
      setEngines(await listEngines());
    } catch (e) {
      appStore.showError(String(e));
    }
  };

  onMount(async () => {
    await loadEngines();
    const unlistenProgress = await onDownloadProgress((p) => {
      setProgress(p);
      if (p.percent >= 100) {
        setDownloading(null);
        setProgress(null);
        loadEngines();
      }
    });
    const unlistenSettings = await onSettingsUpdated(() => {
      loadEngines();
    });
    onCleanup(() => {
      unlistenProgress();
      unlistenSettings();
    });
  });

  const handleDownload = async (modelId: string) => {
    setDownloading(modelId);
    try {
      await downloadModel(modelId);
      await loadEngines();
    } catch (e) {
      // Don't show error if download was cancelled by user
      if (downloading()) appStore.showError(String(e));
    }
    setDownloading(null);
    setProgress(null);
  };

  const handleCancel = async () => {
    const id = downloading();
    setDownloading(null);
    setProgress(null);
    if (id) await cancelDownload(id).catch(() => {});
  };

  const handleDelete = async (modelId: string) => {
    try {
      await deleteModel(modelId);
      await loadEngines();
    } catch (e) {
      appStore.showError(String(e));
    }
  };

  const handleActivate = async (modelId: string) => {
    try {
      await setActiveModel(modelId);
      await loadEngines();
    } catch (e) {
      appStore.showError(String(e));
    }
  };

  const isDark = () => appStore.theme() === "dark";

  const statusBadge = (engine: EngineInfo) =>
    engine.loaded ? (
      <span
        class={`text-xs px-2 py-0.5 rounded-full ${
          isDark()
            ? "bg-emerald-500/15 text-emerald-400 ring-1 ring-emerald-500/20"
            : "bg-emerald-50 text-emerald-600 ring-1 ring-emerald-200"
        }`}
      >
        {i18n.t("engines.loaded")}
      </span>
    ) : (
      <span
        class={`text-xs px-2 py-0.5 rounded-full ${
          isDark()
            ? "bg-white/5 text-white/44"
            : "bg-black/5 text-black/40"
        }`}
      >
        {i18n.t("engines.not_loaded")}
      </span>
    );

  return (
    <div class="space-y-4">
      <For each={engines()}>
        {(engine) => (
          <Section
            title={(() => {
              const nameKey = `engines.name.${engine.id}`;
              const translated = i18n.t(nameKey);
              return translated !== nameKey ? translated : engine.name;
            })()}
            action={statusBadge(engine)}
          >
            <p class={`text-xs mb-3 ${isDark() ? "text-white/44" : "text-black/40"}`}>
              {i18n.t(engine.name.toLowerCase().includes("whisper") ? "engines.whisper_desc" : "engines.llm_desc")}
            </p>

            <div class="space-y-2">
              <For each={engine.models}>
                {(model) => (
                  <div
                    class={`rounded-lg p-3 flex items-center justify-between ${
                      isDark()
                        ? "bg-white/5 border border-border-subtle"
                        : "bg-surface-base-light border border-border-subtle-lt"
                    }`}
                  >
                    <div class="flex-1 min-w-0">
                      <div class="flex items-center gap-2">
                        <span class="text-sm font-medium">{(() => {
                          const nameKey = `model.name.${model.id}`;
                          const translated = i18n.t(nameKey);
                          return translated !== nameKey ? translated : model.name;
                        })()}</span>
                        <Show when={model.active}>
                          <span
                            class={`text-xs px-2 py-0.5 rounded-full ${
                              isDark()
                                ? "bg-accent-muted text-blue-400 ring-1 ring-blue-500/20"
                                : "bg-blue-50 text-blue-600 ring-1 ring-blue-200"
                            }`}
                          >
                            {i18n.t("engines.active")}
                          </span>
                        </Show>
                      </div>
                      <p class={`text-xs mt-0.5 ${isDark() ? "text-white/44" : "text-black/40"}`}>
                        {(() => {
                          const key = `model.desc.${model.id}`;
                          const translated = i18n.t(key);
                          return translated !== key ? translated : model.description;
                        })()} &middot; {model.size_mb} MB
                      </p>

                      <Show
                        when={
                          downloading() === model.id && progress()
                        }
                      >
                        <div class="mt-2 flex items-center gap-2">
                          <div class="flex-1">
                            <ProgressBar
                              percent={progress()?.percent ?? 0}
                              label={i18n.t("engines.downloading")}
                            />
                          </div>
                          <Button
                            size="sm"
                            variant="danger"
                            onClick={handleCancel}
                          >
                            {i18n.t("engines.cancel")}
                          </Button>
                        </div>
                      </Show>
                    </div>

                    <div class="flex gap-2 ml-3">
                      <Show
                        when={model.downloaded}
                        fallback={
                          <Button
                            size="sm"
                            onClick={() => handleDownload(model.id)}
                            loading={downloading() === model.id}
                            disabled={downloading() !== null}
                          >
                            {i18n.t("engines.download")}
                          </Button>
                        }
                      >
                        <Show when={!model.active}>
                          <Button
                            size="sm"
                            variant="secondary"
                            onClick={() => handleActivate(model.id)}
                          >
                            {i18n.t("engines.activate")}
                          </Button>
                        </Show>
                        <Button
                          size="sm"
                          variant="danger"
                          onClick={() => handleDelete(model.id)}
                          disabled={model.active}
                        >
                          {i18n.t("engines.delete")}
                        </Button>
                      </Show>
                    </div>
                  </div>
                )}
              </For>
            </div>
          </Section>
        )}
      </For>

      <Show when={engines().length === 0}>
        <p class={`text-sm ${isDark() ? "text-white/44" : "text-black/40"}`}>{i18n.t("engines.loading")}</p>
      </Show>
    </div>
  );
}
