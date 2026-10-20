import { useCallback, useState } from "react";
import type { Point } from "./useFolderMotion";

/** The open folder and the tile it was opened from. */
export type OpenFolder = { readonly groupId: string; readonly origin: Point | null };

/** The folder tile for `groupId` on screen (not on a page off screen). */
function tileOf(groupId: string): HTMLElement | null {
  const tiles = document.querySelectorAll<HTMLElement>(".launchpad-main [data-folder]");
  for (const tile of tiles) {
    if (tile.dataset.folder === groupId && tile.closest("[inert]") === null) return tile;
  }
  return null;
}

/**
 * Which Launchpad folder is open. `open` remembers the centre of the
 * folder's tile (the open animation's origin); `closed` forgets it and
 * gives the tile its focus back; `reset` forgets it without touching
 * focus (the Launchpad was hidden).
 */
export function useOpenFolder() {
  const [folder, setFolder] = useState<OpenFolder | null>(null);

  const open = useCallback((groupId: string) => {
    const rect = tileOf(groupId)?.getBoundingClientRect();
    const origin = rect ? { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 } : null;
    setFolder({ groupId, origin });
  }, []);

  const groupId = folder?.groupId ?? null;
  const closed = useCallback(() => {
    setFolder(null);
    if (groupId === null) return;
    // After the panel has gone, so the tile is focusable again.
    requestAnimationFrame(() => tileOf(groupId)?.focus({ preventScroll: true }));
  }, [groupId]);

  const reset = useCallback(() => setFolder(null), []);
  return { folder, open, closed, reset };
}
