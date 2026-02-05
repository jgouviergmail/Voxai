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
  return (
    <div class="py-2">
      <label class="block text-sm font-medium mb-1">{props.label}</label>
      <select
        value={props.value}
        onChange={(e) => props.onChange(e.currentTarget.value)}
        class={`w-full rounded-lg px-3 py-2 text-sm border ${
          appStore.theme() === "dark"
            ? "bg-gray-800 border-gray-700 text-gray-100"
            : "bg-white border-gray-300 text-gray-900"
        }`}
      >
        <For each={props.options}>
          {(opt) => <option value={opt.value}>{opt.label}</option>}
        </For>
      </select>
    </div>
  );
}
