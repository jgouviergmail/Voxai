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
    "rounded-lg font-medium inline-flex items-center justify-center";

  const variantClass = () => {
    switch (variant()) {
      case "primary":
        return "bg-blue-600 hover:bg-blue-700 text-white";
      case "secondary":
        return isDark()
          ? "bg-gray-700 hover:bg-gray-600 text-gray-200"
          : "bg-gray-100 hover:bg-gray-200 text-gray-700";
      case "danger":
        return "bg-red-600 hover:bg-red-700 text-white";
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
