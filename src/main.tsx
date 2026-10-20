import { lazy, StrictMode, Suspense } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { createRoot } from "react-dom/client";
import { Toaster } from "./components/Toaster";
import { iconUrl, initIconUrls } from "./lib/iconUrl";
import { applyMaterial, applyTheme, followAccent } from "./lib/theme";
import { installErrorForwarding } from "./logging";
import { useAppStore } from "./store/appStore";
import "./styles.css";
import { FlyoutRoot } from "./windows/FlyoutRoot";
import { LaunchpadRoot } from "./windows/LaunchpadRoot";
import { ManagerRoot } from "./windows/ManagerRoot";

/** Dev only (`?window=styleguide`); the import is dropped from builds. */
const StyleguideRoot = import.meta.env.DEV
  ? lazy(() => import("./windows/styleguide/StyleguideRoot"))
  : null;

/** Reads `window` from the page query, defaulting to `manager`. */
function readWindowLabel(): string {
  return new URLSearchParams(window.location.search).get("window") ?? "manager";
}

const label = readWindowLabel();

/** Renders the root for the current window label. */
function WindowRoot() {
  if (label === "styleguide" && StyleguideRoot !== null) {
    return (
      <Suspense>
        <StyleguideRoot />
      </Suspense>
    );
  }
  if (label === "launchpad") {
    return <LaunchpadRoot />;
  }
  if (label === "flyout") {
    return <FlyoutRoot />;
  }
  return <ManagerRoot />;
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
