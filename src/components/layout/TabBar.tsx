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
      class={`flex border-b ${
        isDark() ? "border-gray-800" : "border-gray-200"
      }`}
    >
      <For each={props.tabs}>
        {(tab) => (
          <button
            class={`px-4 py-2 text-sm font-medium transition-colors ${
              props.active === tab.id
                ? "text-blue-400 border-b-2 border-blue-400"
                : isDark()
                  ? "text-gray-400 hover:text-gray-200"
                  : "text-gray-500 hover:text-gray-700"
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
