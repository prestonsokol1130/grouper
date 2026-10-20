import { useEffect, useState } from "react";
import { computeGrid, type GridLayout } from "../../lib/grid";
import { launchpadDensity } from "./launchpadGrid";

function measure(): GridLayout {
  return computeGrid({ width: window.innerWidth, height: window.innerHeight }, launchpadDensity());
}

/**
 * The grid for the window's current size. Rust moves the Launchpad to the
 * monitor under the cursor before each show, so this follows resizes.
 */
export function useViewportGrid(): GridLayout {
  const [grid, setGrid] = useState(measure);
  useEffect(() => {
    const onResize = () =>
      setGrid((previous) => {
        const next = measure();
        return JSON.stringify(next) === JSON.stringify(previous) ? previous : next;
      });
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);
  return grid;
}
