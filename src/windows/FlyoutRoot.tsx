import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useEffect, useState } from "react";
import { commands, type AppState, type FlyoutShow } from "../bindings";

/** Calls `stop` once `pending` resolves, ignoring a failed registration. */
function unlistenLater(pending: Promise<() => void>): () => void {
  return () => {
    pending.then((stop) => stop()).catch(() => undefined);
  };
}

/** Keeps the shown group's payload and app names current. */
function useFlyoutData(): { show: FlyoutShow | null; names: Map<string, string> } {
  const [show, setShow] = useState<FlyoutShow | null>(null);
  const [names, setNames] = useState<Map<string, string>>(new Map());

  useEffect(() => {
    const applyState = (state: AppState) =>
      setNames(new Map((state.apps ?? []).map((app) => [app.id ?? "", app.name ?? ""])));
    // Listeners first, so nothing sent while the initial reads run is lost.
    const stopShow = unlistenLater(
      getCurrentWebviewWindow().listen<FlyoutShow>("flyout:show", (event) =>
        setShow(event.payload),
      ),
    );
    const stopState = unlistenLater(
      listen<AppState>("state:changed", (event) => applyState(event.payload)),
    );
    commands
      .getFlyout()
      .then((current) => setShow((previous) => previous ?? current))
      .catch((error: unknown) => console.error("flyout: get_flyout:", error));
    commands
      .getState()
      .then((result) => {
        if (result.status === "ok") applyState(result.data);
        else console.error(`flyout: get_state: ${result.error}`);
      })
      .catch((error: unknown) => console.error("flyout: get_state:", error));
    return () => {
      stopShow();
      stopState();
    };
  }, []);

  return { show, names };
}

/** Escape hides the flyout. */
function useEscapeToHide(): void {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      commands.hideFlyout().catch((error: unknown) => console.error("flyout: hide:", error));
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}

/** Starts app `appId`; the backend hides the flyout first. */
function launch(appId: string): void {
  commands
    .launchFromFlyout(appId)
    .then((result) => {
      if (result.status === "error") console.error(`flyout: launch: ${result.error}`);
    })
    .catch((error: unknown) => console.error("flyout: launch:", error));
}

/**
 * Placeholder flyout: a plain list of the group's app names. The real
 * flyout UI comes with S4-04 and DESIGN.md.
 */
export function FlyoutRoot() {
  const { show, names } = useFlyoutData();
  useEscapeToHide();

  return (
    <div className="h-screen overflow-auto bg-white p-4 text-black">
      {show === null || show.apps.length === 0 ? (
        <p>No apps</p>
      ) : (
        <ul>
          {show.apps.map((appId) => (
            <li key={appId}>
              <button type="button" onClick={() => launch(appId)}>
                {names.get(appId) ?? appId}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
