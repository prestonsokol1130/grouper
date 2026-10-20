import { convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { sep } from "@tauri-apps/api/path";
import { commands } from "../bindings";

/** The icon cache folder, `{app_data_dir}/cache/icons`, once it is known. */
let cacheDir: string | null = null;

/**
 * Cache-buster for icon URLs. It starts at this page load's time, so a
 * reloaded page never reuses an image cached by an earlier load, and is
 * bumped on every `icons:updated` event.
 */
let version = Date.now();

/**
 * Starts bumping the cache-buster on `icons:updated`, then fetches the icon
 * cache folder once. The listener is registered first so an event that
 * arrives while the folder is being fetched is not missed. Call once at
 * startup; failures are logged.
 */
export async function initIconUrls(): Promise<void> {
  await listen<string[]>("icons:updated", () => {
    version += 1;
  });
  const result = await commands.getCacheDir();
  if (result.status === "error") {
    console.error(`could not get the icon cache dir: ${result.error}`);
    return;
  }
  cacheDir = result.data;
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
 * icon shown for apps with no icon of their own. `cacheBuster` defaults to
 * this module's own counter; components pass the app store's
 * `iconVersion` so they re-render when it changes.
 */
export function iconUrl(appId: string, cacheBuster: number = version): string | null {
  if (previewUrls !== null) {
    return previewUrls.get(appId) ?? null;
  }
  if (cacheDir === null) {
    return null;
  }
  return `${convertFileSrc(`${cacheDir}${sep()}${appId}.png`)}?v=${cacheBuster}`;
}
