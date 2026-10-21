import { lazy, StrictMode, Suspense } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { createRoot } from "react-dom/client";
import { Toaster } from "./components/Toaster";
import { followSettings } from "./lib/followSettings";
import { followHotkeyFailures } from "./lib/hotkeyToast";
import { iconUrl, initIconUrls } from "./lib/iconUrl";
import { applyMaterial, applyTheme, followAccent } from "./lib/theme";
import { installErrorForwarding } from "./logging";
import { useAppStore } from "./store/appStore";
import "./styles.css";

/*
 * Each window loads only its own root: the flyout and the Launchpad never
 * parse the Manager's code, and the other way round. The chunks load once,
 * when the hidden window is created at startup, so a show never waits.
 */
const FlyoutRoot = lazy(() =>
  import("./windows/flyout/FlyoutRoot").then((m) => ({ default: m.FlyoutRoot })),
);
const LaunchpadRoot = lazy(() =>
  import("./windows/launchpad/LaunchpadRoot").then((m) => ({ default: m.LaunchpadRoot })),
);
const ManagerRoot = lazy(() =>
  import("./windows/manager/ManagerRoot").then((m) => ({ default: m.ManagerRoot })),
);

/** Dev only (`?window=styleguide`); the import is dropped from builds. */
const StyleguideRoot = import.meta.env.DEV
  ? lazy(() => import("./windows/styleguide/StyleguideRoot"))
  : null;

/** Reads `window` from the page query, defaulting to `manager`. */
function readWindowLabel(): string {
  return new URLSearchParams(window.location.search).get("window") ?? "manager";
}

const label = readWindowLabel();

/** The root for the current window label. */
function Root() {
  if (label === "styleguide" && StyleguideRoot !== null) return <StyleguideRoot />;
  if (label === "launchpad") return <LaunchpadRoot />;
  if (label === "flyout") return <FlyoutRoot />;
  return <ManagerRoot />;
}

/** Renders the root for the current window label once its chunk loads. */
function WindowRoot() {
  return (
    <Suspense>
      <Root />
    </Suspense>
  );
}

installErrorForwarding(label);
// Light/dark and the accent follow Windows; the material decides whether
// the page paints its own background (always solid outside Tauri).
applyTheme();
void applyMaterial();
if (isTauri()) followAccent();
// The style guide brings its own sample data.
if (isTauri() && label !== "styleguide") {
  useAppStore.getState().init();
  followSettings();
  // Either window can tell the user the Launchpad shortcut is taken.
  if (label === "manager" || label === "launchpad") followHotkeyFailures();
  // Preload the fallback icon so it is warm for the first UI. The Image is
  // never attached to the DOM, so nothing is shown.
  initIconUrls()
    .then(() => {
      const fallback = iconUrl("_fallback", useAppStore.getState().iconVersion);
      if (fallback !== null) new Image().src = fallback;
    })
    .catch((error: unknown) => console.error("icon URLs:", error));
}

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(
    <StrictMode>
      <WindowRoot />
      <Toaster />
    </StrictMode>,
  );
}
