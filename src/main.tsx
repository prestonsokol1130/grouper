import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { initIconUrls } from "./lib/iconUrl";
import { installErrorForwarding } from "./logging";
import "./styles.css";
import { FlyoutRoot } from "./windows/FlyoutRoot";
import { LaunchpadRoot } from "./windows/LaunchpadRoot";
import { ManagerRoot } from "./windows/ManagerRoot";

/** Reads `window` from the page query, defaulting to `manager`. */
function readWindowLabel(): string {
  return new URLSearchParams(window.location.search).get("window") ?? "manager";
}

/** Renders the placeholder for the current window label. */
function WindowRoot() {
  const label = readWindowLabel();
  if (label === "launchpad") {
    return <LaunchpadRoot />;
  }
  if (label === "flyout") {
    return <FlyoutRoot />;
  }
  return <ManagerRoot />;
}

installErrorForwarding(readWindowLabel());
initIconUrls().catch((error: unknown) => console.error("icon URLs:", error));

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(
    <StrictMode>
      <WindowRoot />
    </StrictMode>,
  );
}
