import { appStore } from "../../lib/stores";

interface ProgressBarProps {
  percent: number;
  label?: string;
}

export default function ProgressBar(props: ProgressBarProps) {
  const isDark = () => appStore.theme() === "dark";

  return (
    <div>
      {props.label && (
        <div class="flex justify-between text-xs text-gray-500 mb-1">
          <span>{props.label}</span>
          <span>{Math.round(props.percent)}%</span>
        </div>
      )}
      <div
        class={`h-2 rounded-full overflow-hidden ${
          isDark() ? "bg-gray-700" : "bg-gray-200"
        }`}
      >
        <div
          class="h-full bg-blue-500 rounded-full transition-all duration-300"
          style={{ width: `${Math.min(100, Math.max(0, props.percent))}%` }}
        />
      </div>
    </div>
  );
}
