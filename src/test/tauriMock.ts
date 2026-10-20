import { vi } from "vitest";

/** Stands in for `invoke`: tests set what each command returns or throws. */
export const invokeMock = vi.fn<(command: string, args?: unknown) => Promise<unknown>>();

type Handler = (event: { payload: unknown }) => void;
const listeners = new Map<string, Set<Handler>>();

/** Stands in for `listen` from `@tauri-apps/api/event`. */
export function listenMock(name: string, handler: Handler): Promise<() => void> {
  const set = listeners.get(name) ?? new Set<Handler>();
  set.add(handler);
  listeners.set(name, set);
  return Promise.resolve(() => {
    set.delete(handler);
  });
}

/** Delivers backend event `name` to every listener. */
export function emitMock(name: string, payload: unknown): void {
  for (const handler of listeners.get(name) ?? []) handler({ payload });
}

/** Forgets every listener and invoke behaviour. */
export function resetTauriMock(): void {
  listeners.clear();
  invokeMock.mockReset();
}
