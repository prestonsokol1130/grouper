import type { ButtonHTMLAttributes, ReactNode } from "react";
import type { PreviewState } from "./activation";

/** Fluent button styles: accent, standard and subtle. */
export type ButtonVariant = "primary" | "secondary" | "ghost";

type Common = Omit<ButtonHTMLAttributes<HTMLButtonElement>, "className" | "type"> & {
  variant?: ButtonVariant;
  /** Style guide only: shows a state without a pointer or keyboard. */
  preview?: PreviewState;
  /** Submit buttons in forms; plain buttons otherwise. */
  type?: "button" | "submit";
};

function classes(variant: ButtonVariant, extra: string): string {
  return `btn btn-${variant} focus-ring rounded-control type-body-strong ${extra}`;
}

/**
 * A text button, Fluent sizing: 32px tall, 4px corners, Body Strong text
 * (DESIGN.md sections 5 and 6). Primary uses the accent.
 */
export function Button({
  variant = "secondary",
  preview,
  type = "button",
  disabled,
  children,
  ...rest
}: Common & { children: ReactNode }) {
  return (
    <button
      {...rest}
      type={type}
      disabled={disabled || preview === "disabled"}
      data-preview={preview}
      className={classes(variant, "inline-flex h-8 items-center justify-center gap-2 px-3")}
    >
      {children}
    </button>
  );
}

/**
 * A square 32px icon-only button. `label` names it for screen readers and
 * the tooltip. Ghost (Fluent subtle) by default.
 */
export function IconButton({
  variant = "ghost",
  preview,
  type = "button",
  disabled,
  label,
  children,
  ...rest
}: Common & { label: string; children: ReactNode }) {
  return (
    <button
      {...rest}
      type={type}
      aria-label={label}
      title={label}
      disabled={disabled || preview === "disabled"}
      data-preview={preview}
      className={classes(variant, "inline-flex size-8 items-center justify-center")}
    >
      {children}
    </button>
  );
}
