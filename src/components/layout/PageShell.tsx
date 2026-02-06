import { type JSX, Show } from "solid-js";
import { appStore } from "../../lib/stores";
import { i18n } from "../../lib/i18n";

interface PageShellProps {
  children: JSX.Element;
  statusBar?: JSX.Element;
}

export default function PageShell(props: PageShellProps) {
  const isDark = () => appStore.theme() === "dark";

  return (
    <div
      class={`h-screen flex flex-col ${
        isDark() ? "bg-gray-900 text-gray-100" : "bg-white text-gray-900"
      }`}
    >
      {/* Title bar — draggable */}
      <header
        data-tauri-drag-region
        class={`flex items-center justify-between px-5 py-3 border-b shrink-0 ${
          isDark() ? "border-gray-800" : "border-gray-200"
        }`}
      >
        <div data-tauri-drag-region>
          <h1 class="text-lg font-semibold tracking-tight">Voxai</h1>
          <p
            class={`text-xs ${isDark() ? "text-gray-500" : "text-gray-400"}`}
          >
            {i18n.t("app.subtitle")}
          </p>
        </div>
        <button
          onClick={appStore.toggleTheme}
          class={`w-8 h-8 flex items-center justify-center rounded-lg text-sm ${
            isDark()
              ? "hover:bg-gray-800 text-gray-400"
              : "hover:bg-gray-100 text-gray-600"
          }`}
          title="Toggle theme"
        >
          {isDark() ? "\u2600" : "\u263E"}
        </button>
      </header>

      {/* Status bar — fixed, not scrollable */}
      <Show when={props.statusBar}>
        <div
          class={`shrink-0 border-b px-5 py-2 ${
            isDark() ? "border-gray-800" : "border-gray-200"
          }`}
        >
          {props.statusBar}
        </div>
      </Show>

      {/* Scrollable content */}
      <main class="flex-1 overflow-y-auto px-5 py-4">
        <Show when={appStore.error()}>
          <div
            class={`rounded-lg p-3 mb-4 text-sm flex items-start gap-2 ${
              isDark()
                ? "bg-red-900/40 border border-red-800/60 text-red-300"
                : "bg-red-50 border border-red-200 text-red-700"
            }`}
          >
            <span class="shrink-0 mt-0.5">!</span>
            <span>{appStore.error()}</span>
          </div>
        </Show>

        {props.children}
      </main>
    </div>
  );
}
