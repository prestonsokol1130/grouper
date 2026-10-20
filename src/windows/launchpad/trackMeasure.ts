import { getClientRect, type ClientRect } from "@dnd-kit/core";

/** The track's x translation right now (mid-slide too), in px. */
function currentShift(track: HTMLElement): number {
  if (typeof DOMMatrixReadOnly === "undefined") return 0;
  const transform = getComputedStyle(track).transform;
  if (transform === "" || transform === "none") return 0;
  try {
    return new DOMMatrixReadOnly(transform).m41;
  } catch {
    return 0;
  }
}

/**
 * Measures a tile for drag and drop where it will be once the page track
 * has finished sliding: a page that turns mid-drag measures its tiles
 * while the track is still moving, and they would otherwise keep the
 * rects of the start of the slide. PagedGrid keeps the page the track is
 * heading for in `data-page`; the tile's own sorting transform is ignored,
 * as dnd-kit's default measure does.
 */
export function measureSettled(element: HTMLElement): ClientRect {
  const rect = getClientRect(element, { ignoreTransform: true });
  const track = element.closest<HTMLElement>(".launchpad-track");
  const page = Number(track?.dataset.page);
  if (track === null || !Number.isFinite(page)) return rect;
  const shift = -page * track.clientWidth - currentShift(track);
  if (shift === 0) return rect;
  return { ...rect, left: rect.left + shift, right: rect.right + shift };
}
