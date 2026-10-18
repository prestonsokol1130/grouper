import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./styles.css";
import { FlyoutRoot } from "./windows/FlyoutRoot";
import { LaunchpadRoot } from "./windows/LaunchpadRoot";
import { ManagerRoot } from "./windows/ManagerRoot";

function readWindowLabel(): string {
  return new URLSearchParams(window.location.search).get("window") ?? "manager";
}

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

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(
    <StrictMode>
      <WindowRoot />
    </StrictMode>,
  );
}
