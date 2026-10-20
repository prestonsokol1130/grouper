import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";
import { resetTauriMock } from "./tauriMock";

// @tauri-apps/api talks to the Tauri runtime, which tests do not have.
vi.mock("@tauri-apps/api/core", async () => {
  const mock = await import("./tauriMock");
  return {
    invoke: (command: string, args?: unknown) => mock.invokeMock(command, args),
    isTauri: () => false,
    convertFileSrc: (path: string) => `asset://localhost/${path}`,
  };
});

vi.mock("@tauri-apps/api/event", async () => {
  const mock = await import("./tauriMock");
  return { listen: mock.listenMock };
});

vi.mock("@tauri-apps/api/path", () => ({ sep: () => "\\" }));

// jsdom has no matchMedia.
if (typeof window.matchMedia !== "function") {
  window.matchMedia = (query: string) =>
    ({
      matches: false,
      media: query,
      onchange: null,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
      addListener: () => undefined,
      removeListener: () => undefined,
      dispatchEvent: () => false,
    }) as MediaQueryList;
}

afterEach(() => {
  cleanup();
  resetTauriMock();
});
