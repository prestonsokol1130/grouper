import type { AppState } from "../../bindings";

/** Sample apps: [id, name, icon colour, flags]. Demo content only. */
const APPS: [string, string, string, { missing?: boolean; hidden?: boolean }?][] = [
  ["mail", "Mail", "#2B7CD3"],
  ["music", "Music", "#E3405F"],
  ["photos", "Photos", "#F2A33A"],
  ["notes", "Notes", "#F5D547"],
  ["terminal", "Terminal", "#3C3C3C"],
  ["maps", "Maps", "#3FAE5A"],
  ["long", "Visual Studio Code Insiders Edition", "#7E57C2"],
  ["gone", "Spotify", "#1DB954", { missing: true }],
  ["hidden", "Hidden app", "#999999", { hidden: true }],
  ["broken", "Broken icon", "#000000"],
];

/** A sample `AppState` covering 0-5 app groups, a missing and a hidden app. */
export const SAMPLE_STATE: AppState = {
  schemaVersion: 1,
  apps: APPS.map(([id, name, , flags]) => ({
    id,
    name,
    target: `C:\\Apps\\${id}.exe`,
    source: "startMenu",
    missing: flags?.missing ?? false,
    hidden: flags?.hidden ?? false,
  })),
  groups: [
    { id: "work", name: "Work", appIds: ["mail", "notes", "terminal", "maps", "photos"] },
    { id: "three", name: "Creative", appIds: ["photos", "music", "notes"] },
    { id: "two", name: "Two apps", appIds: ["mail", "music"] },
    { id: "one", name: "Just one", appIds: ["maps"] },
    { id: "skips", name: "Skips missing and hidden", appIds: ["gone", "hidden", "mail"] },
    { id: "empty", name: "Empty folder", appIds: [] },
  ],
  launchpad: { pages: [] },
  settings: {},
};

/** A rounded-square SVG icon with the app's initial, as a data URL. */
function sampleIcon(letter: string, colour: string): string {
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 256 256">` +
    `<rect x="16" y="16" width="224" height="224" rx="52" fill="${colour}"/>` +
    `<text x="128" y="168" font-family="Segoe UI, sans-serif" font-size="120" ` +
    `font-weight="600" fill="#fff" text-anchor="middle">${letter}</text></svg>`;
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}

/** Stand-in icon URLs by app id, plus the fallback icon. */
export function sampleIconUrls(): Map<string, string> {
  const urls = new Map(APPS.map(([id, name, colour]) => [id, sampleIcon(name[0] ?? "?", colour)]));
  urls.set("_fallback", sampleIcon("?", "#8A8A8A"));
  // A URL that fails to load, to show the fallback icon in action.
  urls.set("broken", "data:image/png;base64,broken");
  return urls;
}
