import {
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type ReactNode,
  type WheelEvent,
} from "react";
import type { LaunchpadItem } from "../../bindings";
import type { GridLayout } from "../../lib/grid";
import { PAGE_SWIPE, REDUCED_FADE } from "../../lib/motion";
import { itemKey } from "./launchpadGrid";
import { WheelPager, type Step } from "./paging";
import { TileGrid } from "./TileGrid";
import { useDragPaging } from "./useDragPaging";
import type { Pager } from "./usePager";

type Props = {
  pages: readonly (readonly LaunchpadItem[])[];
  pager: Pager;
  grid: GridLayout;
  reduced: boolean;
  /** Changes on every show: the track jumps to its page without sliding. */
  showSeq: number;
  renderTile: (item: LaunchpadItem) => ReactNode;
};

/**
 * The paged app grid: every page side by side on one track that slides
 * with `transform` only (DESIGN.md section 10: 250ms ease-out, and with
 * reduced motion no slide, the new page fades in over 120ms). A slide is
 * a CSS transition, so a new turn mid-slide carries on from where the
 * track is. The wheel, a drag or a flick turn pages; only the current
 * page, its neighbours and the page being left are rendered.
 */
export function PagedGrid({ pages, pager, grid, reduced, showSeq, renderTile }: Props) {
  const track = useRef<HTMLDivElement>(null);
  const [wheel] = useState(() => new WheelPager());
  const placedSeq = useRef(showSeq);
  const { page, from, turn } = pager;

  /** Puts the track at the current page plus `offset` px. */
  const place = (offset: number, animate: boolean) => {
    const element = track.current;
    if (element === null) return;
    element.classList.toggle("is-sliding", animate && !reduced);
    element.style.transform = `translate3d(calc(${-page * 100}% + ${offset}px), 0, 0)`;
  };

  useLayoutEffect(() => {
    const fresh = placedSeq.current !== showSeq;
    placedSeq.current = showSeq;
    place(0, !fresh);
    if (!reduced || fresh) return;
    const shown = track.current?.children.item(page);
    if (shown instanceof HTMLElement && typeof shown.animate === "function") {
      shown.animate([{ opacity: 0 }, { opacity: 1 }], {
        duration: REDUCED_FADE.durationMs,
        easing: REDUCED_FADE.easing,
      });
    }
    // `place` reads the props this effect depends on.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [page, showSeq, reduced]);

  const drag = useDragPaging({
    page,
    count: pages.length,
    follow: (offset) => place(offset, false),
    release: (step: Step) => {
      if (step === 0 || page + step < 0 || page + step >= pages.length) place(0, true);
      else turn(step);
    },
  });

  const onWheel = (event: WheelEvent<HTMLDivElement>) => {
    const step = wheel.step(event.deltaX, event.deltaY, event.timeStamp);
    if (step !== 0) turn(step);
  };

  return (
    <div
      className="launchpad-pages relative min-h-0 flex-1 overflow-hidden"
      onWheel={onWheel}
      {...drag}
    >
      <div
        ref={track}
        className="launchpad-track flex h-full"
        style={
          {
            "--page-duration": `${PAGE_SWIPE.durationMs}ms`,
            "--page-easing": PAGE_SWIPE.easing,
          } as CSSProperties
        }
      >
        {pages.map((items, index) => {
          const near = Math.abs(index - page) <= 1 || index === from;
          return (
            <div
              key={index}
              className="launchpad-page flex h-full w-full shrink-0 items-center justify-center"
              aria-hidden={index === page ? undefined : true}
              inert={index !== page}
            >
              {near && (
                <TileGrid grid={grid} label="Apps">
                  {items.map((item) => (
                    <div key={itemKey(item)} className="flex items-center justify-center">
                      {renderTile(item)}
                    </div>
                  ))}
                </TileGrid>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
