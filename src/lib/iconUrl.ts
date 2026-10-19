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
 * The asset URL of `appId`'s cached icon PNG, or `null` before
 * {@link initIconUrls} has finished. `iconUrl("_fallback")` is the fallback
 * icon shown for apps with no icon of their own.
 */
export function iconUrl(appId: string): string | null {
  if (cacheDir === null) {
    return null;
  }
  return `${convertFileSrc(`${cacheDir}${sep()}${appId}.png`)}?v=${version}`;
}
