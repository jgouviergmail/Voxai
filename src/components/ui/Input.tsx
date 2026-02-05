import { appStore } from "../../lib/stores";

interface InputProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  type?: "text" | "number";
}

export default function Input(props: InputProps) {
  return (
    <div class="py-2">
      <label class="block text-sm font-medium mb-1">{props.label}</label>
      <input
        type={props.type ?? "text"}
        value={props.value}
        onInput={(e) => props.onChange(e.currentTarget.value)}
        placeholder={props.placeholder}
        class={`w-full rounded-lg px-3 py-2 text-sm border ${
          appStore.theme() === "dark"
            ? "bg-gray-800 border-gray-700 text-gray-100 placeholder-gray-600"
            : "bg-white border-gray-300 text-gray-900 placeholder-gray-400"
        }`}
      />
    </div>
  );
}
