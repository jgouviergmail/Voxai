import { appStore } from "../../lib/stores";

interface InputProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  type?: "text" | "number";
}

export default function Input(props: InputProps) {
  const isDark = () => appStore.theme() === "dark";

  return (
    <div class="py-2">
      <label class="block text-sm font-medium mb-1">{props.label}</label>
      <input
        type={props.type ?? "text"}
        value={props.value}
        onInput={(e) => props.onChange(e.currentTarget.value)}
        placeholder={props.placeholder}
        class={`w-full rounded-md px-3 py-2 text-sm border transition-all focus:outline-none focus:ring-2 focus:ring-accent-glow focus:border-accent ${
          isDark()
            ? "bg-surface-raised border-border-default text-white/92 placeholder:text-white/30"
            : "bg-white border-border-default-lt text-black/88 placeholder:text-black/30"
        }`}
      />
    </div>
  );
}
