import { convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { sep } from "@tauri-apps/api/path";
import { commands } from "../bindings";
import { useAppStore } from "../store/appStore";

/** The icon cache folder, `{app_data_dir}/cache/icons`, once it is known. */
let cacheDir: string | null = null;

/**
 * Bumps the one icon cache-buster, the app store's `iconVersion`, on every
 * `icons:updated`, then fetches the icon cache folder once. The listener
 * is registered first so an event that arrives while the folder is being
 * fetched is not missed. Once the folder is known the version is bumped
 * again so icons drawn before it re-render. Call once at startup; failures
 * are logged.
 */
export async function initIconUrls(): Promise<void> {
  const bump = () => useAppStore.getState().bumpIconVersion();
  await listen<string[]>("icons:updated", bump);
  const result = await commands.getCacheDir();
  if (result.status === "error") {
    console.error(`could not get the icon cache dir: ${result.error}`);
    return;
  }
  cacheDir = result.data;
  bump();
}

/**
 * Stand-in icon URLs for the dev style guide outside Tauri, where there is
 * no icon cache. Never set in the app.
 */
let previewUrls: ReadonlyMap<string, string> | null = null;

/** Sets the style guide's stand-in icons (outside Tauri only). */
export function setPreviewIconUrls(urls: ReadonlyMap<string, string>): void {
  previewUrls = urls;
}

/**
 * The asset URL of `appId`'s cached icon PNG, or `null` before
 * {@link initIconUrls} has finished. `iconUrl("_fallback")` is the fallback
 * icon shown for apps with no icon of their own. `cacheBuster` is the app
 * store's `iconVersion`.
 */
export function iconUrl(appId: string, cacheBuster: number): string | null {
  if (previewUrls !== null) {
    return previewUrls.get(appId) ?? null;
  }
  if (cacheDir === null) {
    return null;
  }
  return `${convertFileSrc(`${cacheDir}${sep()}${appId}.png`)}?v=${cacheBuster}`;
}
