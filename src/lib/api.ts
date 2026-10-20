import { commands, type LaunchpadLayout, type SettingsPatch } from "../bindings";

/** What every {@link api} call resolves to. It never rejects. */
export type Result<T> = { ok: true; value: T } | { ok: false; error: string };

/** The shape tauri-specta gives commands that return `Result` in Rust. */
type Typed<T> = { status: "ok"; data: T } | { status: "error"; error: string };

/** Turns anything thrown by `invoke` into readable text. */
export function errorText(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  try {
    return JSON.stringify(error) ?? String(error);
  } catch {
    return String(error);
  }
}

/** Wraps a command that returns a Rust `Result`. */
async function typed<T>(call: () => Promise<Typed<T>>): Promise<Result<T>> {
  try {
    const result = await call();
    return result.status === "ok"
      ? { ok: true, value: result.data }
      : { ok: false, error: result.error };
  } catch (error) {
    return { ok: false, error: errorText(error) };
  }
}

/** Wraps a command that returns a plain value (and throws on failure). */
async function plain<T>(call: () => Promise<T>): Promise<Result<T>> {
  try {
    return { ok: true, value: await call() };
  } catch (error) {
    return { ok: false, error: errorText(error) };
  }
}

/**
 * Typed wrappers over every backend command in `src/bindings`. Each one
 * resolves to a {@link Result} and never throws to the caller.
 */
export const api = {
  getState: () => typed(() => commands.getState()),
  createGroup: (name: string) => typed(() => commands.createGroup(name)),
  renameGroup: (id: string, name: string) => typed(() => commands.renameGroup(id, name)),
  deleteGroup: (id: string) => typed(() => commands.deleteGroup(id)),
  setGroupApps: (id: string, appIds: string[]) => typed(() => commands.setGroupApps(id, appIds)),
  updateSettings: (patch: SettingsPatch) => typed(() => commands.updateSettings(patch)),
  setLaunchpadLayout: (layout: LaunchpadLayout) => typed(() => commands.setLaunchpadLayout(layout)),
  rescanApps: () => typed(() => commands.rescanApps()),
  getCacheDir: () => typed(() => commands.getCacheDir()),
  launchApp: (appId: string) => typed(() => commands.launchApp(appId)),
  launchGroup: (groupId: string) => typed(() => commands.launchGroup(groupId)),
  pickAndAddApps: () => typed(() => commands.pickAndAddApps()),
  addAppFromPath: (path: string) => typed(() => commands.addAppFromPath(path)),
  removeApp: (appId: string) => typed(() => commands.removeApp(appId)),
  createGroupShortcut: (groupId: string) => typed(() => commands.createGroupShortcut(groupId)),
  revealGroupShortcut: (groupId: string) => typed(() => commands.revealGroupShortcut(groupId)),
  hideFlyout: () => plain(() => commands.hideFlyout()),
  getFlyout: () => plain(() => commands.getFlyout()),
  launchFromFlyout: (appId: string) => typed(() => commands.launchFromFlyout(appId)),
  getAccentColor: () => plain(() => commands.getAccentColor()),
  getWindowMaterial: () => plain(() => commands.getWindowMaterial()),
};
