import { For } from "solid-js";
import { appStore } from "../../lib/stores";

interface Tab {
  id: string;
  label: string;
}

interface TabBarProps {
  tabs: Tab[];
  active: string;
  onSelect: (id: string) => void;
}

export default function TabBar(props: TabBarProps) {
  const isDark = () => appStore.theme() === "dark";

  return (
    <div
      class={`flex gap-1 rounded-xl p-1 ${
        isDark() ? "bg-surface-raised" : "bg-surface-overlay-light"
      }`}
    >
      <For each={props.tabs}>
        {(tab) => (
          <button
            class={`px-3.5 py-1.5 text-xs rounded-lg transition-all ${
              props.active === tab.id
                ? isDark()
                  ? "font-semibold bg-surface-overlay text-white shadow-card"
                  : "font-semibold bg-white text-black/88 shadow-card-lt"
                : isDark()
                  ? "font-medium text-white/44 hover:text-white/64 hover:bg-white/5"
                  : "font-medium text-black/40 hover:text-black/60 hover:bg-black/5"
            }`}
            onClick={() => props.onSelect(tab.id)}
          >
            {tab.label}
          </button>
        )}
      </For>
    </div>
  );
}
