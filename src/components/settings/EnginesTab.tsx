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
  const cardBg = () => (isDark() ? "bg-gray-800" : "bg-gray-50");

  return (
    <div class="space-y-6">
      <For each={engines()}>
        {(engine) => (
          <div>
            <div class="flex items-center gap-2 mb-3">
              <h3 class="text-sm font-semibold">{engine.name}</h3>
              <span
                class={`text-xs px-2 py-0.5 rounded-full ${
                  engine.loaded
                    ? isDark()
                      ? "bg-green-900/50 text-green-400"
                      : "bg-green-100 text-green-700"
                    : isDark()
                      ? "bg-gray-700 text-gray-500"
                      : "bg-gray-200 text-gray-500"
                }`}
              >
                {engine.loaded ? i18n.t("engines.loaded") : i18n.t("engines.not_loaded")}
              </span>
            </div>

            <div class="space-y-2">
              <For each={engine.models}>
                {(model) => (
                  <div
                    class={`${cardBg()} rounded-lg p-3 flex items-center justify-between`}
                  >
                    <div class="flex-1 min-w-0">
                      <div class="flex items-center gap-2">
                        <span class="text-sm font-medium">{model.name}</span>
                        <Show when={model.active}>
                          <span
                            class={`text-xs px-2 py-0.5 rounded-full ${
                              isDark()
                                ? "bg-blue-900/50 text-blue-400"
                                : "bg-blue-100 text-blue-700"
                            }`}
                          >
                            {i18n.t("engines.active")}
                          </span>
                        </Show>
                      </div>
                      <p class="text-xs text-gray-500 mt-0.5">
                        {model.description} &middot; {model.size_mb} MB
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
          </div>
        )}
      </For>

      <Show when={engines().length === 0}>
        <p class="text-gray-500 text-sm">{i18n.t("engines.loading")}</p>
      </Show>
    </div>
  );
}
