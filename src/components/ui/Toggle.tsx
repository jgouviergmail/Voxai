import { appStore } from "../../lib/stores";

interface ToggleProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: string;
  description?: string;
  disabled?: boolean;
}

export default function Toggle(props: ToggleProps) {
  const isDark = () => appStore.theme() === "dark";

  return (
    <label class="flex items-center justify-between py-2 cursor-pointer">
      <div>
        <span class="text-sm font-medium">{props.label}</span>
        {props.description && (
          <p class={`text-xs mt-0.5 ${isDark() ? "text-white/44" : "text-black/40"}`}>
            {props.description}
          </p>
        )}
      </div>
      <button
        type="button"
        role="switch"
        aria-checked={props.checked}
        disabled={props.disabled}
        class={`relative inline-flex h-[22px] w-[42px] items-center rounded-full shrink-0 transition-all duration-200 ${
          props.checked
            ? "bg-blue-500"
            : isDark()
              ? "bg-white/10"
              : "bg-black/15"
        } ${props.disabled ? "opacity-50 cursor-not-allowed" : ""}`}
        onClick={() => !props.disabled && props.onChange(!props.checked)}
      >
        <span
          class={`inline-block h-4 w-4 rounded-full bg-white shadow-sm transition-transform duration-200 ${
            props.checked ? "translate-x-[22px]" : "translate-x-[3px]"
          }`}
        />
      </button>
    </label>
  );
}
