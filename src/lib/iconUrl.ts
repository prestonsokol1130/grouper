import { convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { sep } from "@tauri-apps/api/path";
import { commands } from "../bindings";

/** The icon cache folder, `{app_data_dir}/cache/icons`, once it is known. */
let cacheDir: string | null = null;

/** Cache-buster for icon URLs; bumped on every `icons:updated` event. */
let version = 0;

/**
 * Fetches the icon cache folder once and starts bumping the cache-buster on
 * `icons:updated`. Call once at startup; failures are logged.
 */
export async function initIconUrls(): Promise<void> {
  const result = await commands.getCacheDir();
  if (result.status === "error") {
    console.error(`could not get the icon cache dir: ${result.error}`);
    return;
  }
  cacheDir = result.data;
  await listen<string[]>("icons:updated", () => {
    version += 1;
  });
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
