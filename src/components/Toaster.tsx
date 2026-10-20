import { useEffect } from "react";
import { useToastStore, type Toast, type ToastKind } from "../store/toastStore";
import { Button, IconButton } from "./Button";

/** DESIGN.md section 13: toasts close by themselves after 5 seconds. */
export const TOAST_TIMEOUT_MS = 5000;

const ICON: Record<ToastKind, { glyph: string; color: string; label: string }> = {
  error: { glyph: "!", color: "var(--danger)", label: "Error" },
  success: { glyph: "✓", color: "var(--success)", label: "Done" },
  info: { glyph: "i", color: "var(--accent)", label: "Info" },
};

/** One Fluent InfoBar-style toast. */
function ToastItem({ toast }: { toast: Toast }) {
  const dismiss = useToastStore((store) => store.dismiss);
  const icon = ICON[toast.kind];

  useEffect(() => {
    // A toast that needs an action stays until it is used or closed.
    if (toast.action) return;
    const timer = window.setTimeout(() => dismiss(toast.id), TOAST_TIMEOUT_MS);
    return () => window.clearTimeout(timer);
  }, [toast.id, toast.action, dismiss]);

  return (
    <div
      role={toast.kind === "error" ? "alert" : undefined}
      className="pointer-events-auto flex w-[360px] max-w-full items-start gap-3 rounded-control border border-stroke-card bg-layer py-3 pr-2 pl-4 text-primary shadow-raised"
    >
      <span
        aria-label={icon.label}
        className="mt-0.5 flex size-4 shrink-0 items-center justify-center rounded-full type-caption font-semibold text-surface"
        style={{ background: icon.color }}
      >
        {icon.glyph}
      </span>
      <div className="min-w-0 flex-1">
        <p className="type-body-strong">{toast.title}</p>
        {toast.message && <p className="type-body text-secondary">{toast.message}</p>}
        {toast.action && (
          <div className="mt-2">
            <Button
              variant="secondary"
              onClick={() => {
                toast.action?.run();
                dismiss(toast.id);
              }}
            >
              {toast.action.label}
            </Button>
          </div>
        )}
      </div>
      <IconButton label="Close" onClick={() => dismiss(toast.id)}>
        <span aria-hidden="true">✕</span>
      </IconButton>
    </div>
  );
}

/**
 * Shows the queued toasts at the bottom-right of the window, newest at the
 * bottom, never wider than the window (the flyout is narrow). Render once
 * per window root.
 */
export function Toaster() {
  const toasts = useToastStore((store) => store.toasts);
  return (
    // A live region that exists before any toast, so screen readers
    // announce success and info toasts too; errors are also alerts.
    <div
      role="status"
      aria-live="polite"
      className="pointer-events-none fixed right-3 bottom-3 left-3 z-50 flex flex-col items-end gap-2"
    >
      {toasts.map((toast) => (
        <ToastItem key={toast.id} toast={toast} />
      ))}
    </div>
  );
}
