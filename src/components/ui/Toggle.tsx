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
          <p class="text-xs text-gray-500 mt-0.5">{props.description}</p>
        )}
      </div>
      <button
        type="button"
        role="switch"
        aria-checked={props.checked}
        disabled={props.disabled}
        class={`relative inline-flex h-5 w-9 items-center rounded-full shrink-0 ${
          props.checked
            ? "bg-blue-500"
            : isDark()
              ? "bg-gray-600"
              : "bg-gray-300"
        } ${props.disabled ? "opacity-50 cursor-not-allowed" : ""}`}
        onClick={() => !props.disabled && props.onChange(!props.checked)}
      >
        <span
          class={`inline-block h-3.5 w-3.5 rounded-full bg-white transition-transform shadow-sm ${
            props.checked ? "translate-x-4.5" : "translate-x-0.5"
          }`}
        />
      </button>
    </label>
  );
}
