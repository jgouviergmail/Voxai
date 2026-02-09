import { type JSX, Show } from "solid-js";
import { appStore } from "../../lib/stores";
import { i18n } from "../../lib/i18n";

interface PageShellProps {
  children: JSX.Element;
  statusBar?: JSX.Element;
  tabBar?: JSX.Element;
}

export default function PageShell(props: PageShellProps) {
  const isDark = () => appStore.theme() === "dark";

  return (
    <div
      class={`h-screen flex flex-col ${
        isDark()
          ? "bg-surface-base text-white/92"
          : "bg-surface-base-light text-black/88"
      }`}
    >
      {/* Title bar — draggable */}
      <header
        data-tauri-drag-region
        class={`flex items-center justify-between px-5 py-2.5 shrink-0 ${
          isDark() ? "shadow-card" : "shadow-card-lt"
        }`}
        style={{ "z-index": "10", position: "relative" }}
      >
        <div data-tauri-drag-region>
          <h1 class="text-lg font-bold tracking-tight">Voxai</h1>
          <p class={`text-xs ${isDark() ? "text-white/44" : "text-black/40"}`}>
            {i18n.t("app.subtitle")}
          </p>
        </div>
        <button
          onClick={appStore.toggleTheme}
          class={`w-8 h-8 flex items-center justify-center rounded-lg transition-colors ${
            isDark()
              ? "hover:bg-surface-overlay text-white/44 hover:text-white/64"
              : "hover:bg-surface-overlay-light text-black/40 hover:text-black/60"
          }`}
          title="Toggle theme"
        >
          {isDark() ? (
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <circle cx="12" cy="12" r="5" />
              <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" />
            </svg>
          ) : (
            <svg
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
            </svg>
          )}
        </button>
      </header>

      {/* Status bar — fixed, not scrollable */}
      <Show when={props.statusBar}>
        <div class="shrink-0 px-5 py-2">{props.statusBar}</div>
      </Show>

      {/* Tab bar — fixed, not scrollable */}
      <Show when={props.tabBar}>
        <div class="shrink-0 px-5 pt-2 pb-1">{props.tabBar}</div>
      </Show>

      {/* Scrollable content */}
      <main class="flex-1 overflow-y-auto px-5 py-4">
        <Show when={appStore.error()}>
          <div
            class={`rounded-xl p-3 mb-4 text-sm flex items-start gap-2 ${
              isDark()
                ? "bg-red-900/30 border border-red-800/50 text-red-300"
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
