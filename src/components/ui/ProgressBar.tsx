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
        <div class={`flex justify-between text-xs mb-1 ${isDark() ? "text-white/44" : "text-black/40"}`}>
          <span>{props.label}</span>
          <span>{Math.round(props.percent)}%</span>
        </div>
      )}
      <div
        class={`h-2 rounded-full overflow-hidden ${
          isDark() ? "bg-white/8" : "bg-black/8"
        }`}
      >
        <div
          class="h-full bg-gradient-to-r from-blue-600 to-blue-400 rounded-full transition-all duration-300"
          style={{ width: `${Math.min(100, Math.max(0, props.percent))}%` }}
        />
      </div>
    </div>
  );
}
