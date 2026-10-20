import { create } from "zustand";

/** InfoBar severities (Fluent). */
export type ToastKind = "error" | "success" | "info";

/** A button on a toast. A toast with one stays until it is used or closed. */
type ToastAction = { label: string; run: () => void };

/** What to show. Text follows DESIGN.md section 13: say what happened and what to do next. */
export type ToastInput = {
  kind: ToastKind;
  title: string;
  message?: string;
  action?: ToastAction;
};

export type Toast = ToastInput & { id: number };

type ToastStore = {
  toasts: Toast[];
  show: (toast: ToastInput) => number;
  dismiss: (id: number) => void;
};

let nextId = 1;

/** The toasts on screen, oldest first. */
export const useToastStore = create<ToastStore>()((set) => ({
  toasts: [],
  show: (toast) => {
    const id = nextId++;
    set((store) => ({ toasts: [...store.toasts, { ...toast, id }] }));
    return id;
  },
  dismiss: (id) => set((store) => ({ toasts: store.toasts.filter((toast) => toast.id !== id) })),
}));

/** Shows a toast from outside React (the app store, api callers). */
export function showToast(toast: ToastInput): number {
  return useToastStore.getState().show(toast);
}

/** `show` and `dismiss` for components. */
export function useToast(): Pick<ToastStore, "show" | "dismiss"> {
  const show = useToastStore((store) => store.show);
  const dismiss = useToastStore((store) => store.dismiss);
  return { show, dismiss };
}
