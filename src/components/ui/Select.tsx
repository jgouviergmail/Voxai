import { For } from "solid-js";
import { appStore } from "../../lib/stores";

interface SelectOption {
  value: string;
  label: string;
}

interface SelectProps {
  label: string;
  value: string;
  options: SelectOption[];
  onChange: (value: string) => void;
}

export default function Select(props: SelectProps) {
  const isDark = () => appStore.theme() === "dark";

  return (
    <div class="py-2">
      <label class="block text-sm font-medium mb-1">{props.label}</label>
      <div class="relative">
        <select
          value={props.value}
          onChange={(e) => props.onChange(e.currentTarget.value)}
          class={`w-full rounded-md px-3 py-2 pr-9 text-sm border appearance-none transition-all focus:outline-none focus:ring-2 focus:ring-accent-glow focus:border-accent ${
            isDark()
              ? "bg-surface-raised border-border-default text-white/92"
              : "bg-white border-border-default-lt text-black/88"
          }`}
        >
          <For each={props.options}>
            {(opt) => <option value={opt.value}>{opt.label}</option>}
          </For>
        </select>
        <svg
          class={`absolute right-2.5 top-1/2 -translate-y-1/2 pointer-events-none w-4 h-4 ${
            isDark() ? "text-white/44" : "text-black/40"
          }`}
          viewBox="0 0 20 20"
          fill="currentColor"
        >
          <path
            fill-rule="evenodd"
            d="M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.938a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z"
            clip-rule="evenodd"
          />
        </svg>
      </div>
    </div>
  );
}
