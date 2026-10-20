import { create } from "zustand";
import type { HotkeyFailed } from "../lib/events";

type HotkeyStatus = {
  /** The shortcut Windows refused at startup, until a new one works. */
  failed: HotkeyFailed | null;
};

/** Whether the Launchpad shortcut is working, for the Settings banner. */
export const useHotkeyStatus = create<HotkeyStatus>()(() => ({ failed: null }));

/** Records that `failed.hotkey` could not be registered. */
export function markHotkeyFailed(failed: HotkeyFailed): void {
  useHotkeyStatus.setState({ failed });
}

/** A new shortcut was registered: the banner goes away. */
export function clearHotkeyFailed(): void {
  useHotkeyStatus.setState({ failed: null });
}
