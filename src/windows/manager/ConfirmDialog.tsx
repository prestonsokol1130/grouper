import { useEffect, useId, useRef, type KeyboardEvent, type ReactNode } from "react";
import { Button } from "../../components/Button";

type Props = {
  title: string;
  children: ReactNode;
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
};

/**
 * A Fluent ContentDialog: a smoke layer over the window and a card with a
 * Subtitle title, the message and two buttons. Focus starts on Cancel (the
 * safe choice), Tab stays inside, Escape cancels, and focus goes back to
 * where it was when the dialog closes.
 */
export function ConfirmDialog({ title, children, confirmLabel, onConfirm, onCancel }: Props) {
  const panel = useRef<HTMLDivElement>(null);
  const titleId = useId();
  const bodyId = useId();

  useEffect(() => {
    const previous = document.activeElement;
    panel.current?.querySelector<HTMLElement>("[data-autofocus]")?.focus();
    return () => {
      if (previous instanceof HTMLElement) previous.focus();
    };
  }, []);

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      onCancel();
      return;
    }
    if (event.key !== "Tab") return;
    const buttons = [...(panel.current?.querySelectorAll<HTMLElement>("button") ?? [])];
    const first = buttons[0];
    const last = buttons[buttons.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last?.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first?.focus();
    }
  };

  return (
    <div className="dialog-smoke fixed inset-0 z-40 flex items-center justify-center p-4">
      <div
        ref={panel}
        role="alertdialog"
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={bodyId}
        className="w-[400px] max-w-full overflow-hidden rounded-window border border-stroke-card bg-layer text-primary shadow-raised"
        onKeyDown={onKeyDown}
      >
        <div className="p-6">
          <h2 id={titleId} className="mb-3 type-subtitle">
            {title}
          </h2>
          <div id={bodyId} className="type-body">
            {children}
          </div>
        </div>
        <div className="grid grid-cols-2 gap-2 border-t border-stroke-card bg-surface p-6">
          <Button variant="primary" onClick={onConfirm}>
            {confirmLabel}
          </Button>
          <Button variant="secondary" onClick={onCancel} data-autofocus="">
            Cancel
          </Button>
        </div>
      </div>
    </div>
  );
}
