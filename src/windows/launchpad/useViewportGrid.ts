import { useEffect, useState } from "react";
import { computeGrid, type GridDensity, type GridLayout } from "../../lib/grid";
import { useLaunchpadDensity } from "./launchpadGrid";

function measure(density: GridDensity): GridLayout {
  return computeGrid({ width: window.innerWidth, height: window.innerHeight }, density);
}

/**
 * The grid for the window's current size and the "Launchpad grid" and
 * "Icon size" settings. Rust moves the Launchpad to the monitor under the
 * cursor before each show, so this follows resizes and setting changes.
 */
export function useViewportGrid(): GridLayout {
  const density = useLaunchpadDensity();
  const [grid, setGrid] = useState(() => measure(density));
  useEffect(() => {
    const update = () =>
      setGrid((previous) => {
        const next = measure(density);
        return JSON.stringify(next) === JSON.stringify(previous) ? previous : next;
      });
    update();
    window.addEventListener("resize", update);
    return () => window.removeEventListener("resize", update);
  }, [density]);
  return grid;
}
