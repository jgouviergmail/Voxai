import { createSignal, onMount, onCleanup } from "solid-js";
import { listen } from "@tauri-apps/api/event";
import type { RecordingState } from "./types";

export default function Overlay() {
  const [state, setState] = createSignal<RecordingState>({ kind: "Idle" });

  onMount(async () => {
    const unlisten = await listen<RecordingState>(
      "recording-state-changed",
      (e) => setState(e.payload),
    );
    onCleanup(() => unlisten());
  });

  const dotColor = () => {
    switch (state().kind) {
      case "Idle":
        return "bg-green-500";
      case "Recording":
        return "bg-red-500 animate-pulse";
      case "Processing":
        return "bg-yellow-500 animate-pulse";
    }
  };

  const label = () => {
    switch (state().kind) {
      case "Idle":
        return "Ready";
      case "Recording":
        return "Rec";
      case "Processing":
        return "...";
    }
  };

  return (
    <div
      data-tauri-drag-region
      class="flex items-center gap-2 px-3 py-1.5 rounded-full bg-gray-900/80 backdrop-blur-sm border border-gray-700/50 shadow-lg cursor-move"
    >
      <div class={`w-2 h-2 rounded-full shrink-0 ${dotColor()}`} />
      <span class="text-xs font-medium text-gray-200 select-none whitespace-nowrap">
        {label()}
      </span>
    </div>
  );
}
