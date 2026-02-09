import type { JSX } from "solid-js";
import { appStore } from "../../lib/stores";

interface ButtonProps {
  children: JSX.Element;
  onClick?: () => void;
  variant?: "primary" | "secondary" | "danger";
  size?: "sm" | "md";
  disabled?: boolean;
  loading?: boolean;
}

export default function Button(props: ButtonProps) {
  const variant = () => props.variant ?? "primary";
  const size = () => props.size ?? "md";
  const isDark = () => appStore.theme() === "dark";

  const baseClass =
    "rounded-md font-medium inline-flex items-center justify-center transition-all press-scale";

  const variantClass = () => {
    switch (variant()) {
      case "primary":
        return "bg-blue-600 hover:bg-blue-500 text-white shadow-sm shadow-blue-600/20";
      case "secondary":
        return isDark()
          ? "bg-surface-overlay text-white/64 border border-border-subtle hover:bg-white/10"
          : "bg-surface-overlay-light text-black/60 border border-border-subtle-lt hover:bg-black/5";
      case "danger":
        return "bg-red-600 hover:bg-red-500 text-white shadow-sm shadow-red-600/20";
    }
  };

  const sizeClass = () =>
    size() === "sm" ? "px-3 py-1.5 text-xs" : "px-4 py-2 text-sm";

  return (
    <button
      class={`${baseClass} ${variantClass()} ${sizeClass()} ${
        props.disabled || props.loading ? "opacity-50 cursor-not-allowed" : ""
      }`}
      onClick={props.onClick}
      disabled={props.disabled || props.loading}
    >
      {props.loading ? "..." : props.children}
    </button>
  );
}
