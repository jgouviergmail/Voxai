import { type JSX, Show } from "solid-js";
import { appStore } from "../../lib/stores";

interface SectionProps {
  title: string;
  children: JSX.Element;
  action?: JSX.Element;
}

export default function Section(props: SectionProps) {
  const isDark = () => appStore.theme() === "dark";

  return (
    <div
      class={`rounded-xl border ${
        isDark()
          ? "bg-surface-raised border-border-subtle"
          : "bg-surface-raised-light border-border-subtle-lt shadow-card-lt"
      }`}
    >
      <div
        class={`flex items-center justify-between px-4 py-2.5 border-b ${
          isDark() ? "border-border-subtle" : "border-border-subtle-lt"
        }`}
      >
        <div class="flex items-center gap-2">
          <div class="w-1 h-3.5 rounded-full bg-accent" />
          <h3
            class={`text-xs font-semibold uppercase tracking-wider ${
              isDark() ? "text-white/64" : "text-black/50"
            }`}
          >
            {props.title}
          </h3>
        </div>
        <Show when={props.action}>{props.action}</Show>
      </div>
      <div class="px-4 py-3">{props.children}</div>
    </div>
  );
}
