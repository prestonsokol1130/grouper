import type { ClientRect, CollisionDetection } from "@dnd-kit/core";
import { describe, expect, it } from "vitest";
import { centreTarget, inCentre, intentCollision, REORDER_SETTLE_MS } from "./mergeIntent";

/** A 100px square cell at column `index` of one row, 100px apart. */
const cell = (index: number): ClientRect => {
  const left = index * 100;
  return { left, top: 0, width: 100, height: 100, right: left + 100, bottom: 100 };
};

const IDS = ["a", "b", "c", "d"];
const RECTS = new Map(IDS.map((id, index) => [id, cell(index)]));

describe("inCentre", () => {
  it("is the middle half of the tile both ways, edges included", () => {
    expect(inCentre(cell(0), { x: 50, y: 50 })).toBe(true);
    expect(inCentre(cell(0), { x: 25, y: 75 })).toBe(true);
    expect(inCentre(cell(0), { x: 24, y: 50 })).toBe(false);
    expect(inCentre(cell(0), { x: 50, y: 76 })).toBe(false);
  });
});

describe("centreTarget", () => {
  it("names the tile drawn under the pointer, not the one measured there", () => {
    // Nothing sorted yet: the tile at each slot is the one measured there.
    expect(centreTarget({ x: 150, y: 50 }, IDS, "a", "a", RECTS)).toBe("b");
    // "a" sorted over "c": "b" and "c" moved left one slot each.
    expect(centreTarget({ x: 150, y: 50 }, IDS, "a", "c", RECTS)).toBe("c");
    expect(centreTarget({ x: 50, y: 50 }, IDS, "a", "c", RECTS)).toBe("b");
  });

  it("is null over the dragged tile's slot, an edge or a gap", () => {
    expect(centreTarget({ x: 250, y: 50 }, IDS, "a", "c", RECTS)).toBeNull();
    expect(centreTarget({ x: 110, y: 50 }, IDS, "a", null, RECTS)).toBeNull();
    expect(centreTarget({ x: 150, y: 150 }, IDS, "a", null, RECTS)).toBeNull();
  });
});

describe("intentCollision", () => {
  const args = (x: number, active = "a") =>
    ({
      active: {
        id: active,
        data: { current: { sortable: { items: IDS } } },
        rect: { current: {} },
      },
      collisionRect: { ...cell(0), left: x - 50, right: x + 50 },
      droppableRects: RECTS,
      droppableContainers: IDS.map((id) => ({ id, key: id, data: { current: {} } })),
      pointerCoordinates: { x, y: 50 },
    }) as unknown as Parameters<CollisionDetection>[0];

  it("holds the reorder target still while over a centre, else closestCenter", () => {
    const { detect, centre, reset } = intentCollision(() => true);
    expect(detect(args(105)).map((hit) => hit.id)[0]).toBe("b");
    expect(centre()).toBeNull();
    // "a" over "b" swaps them on screen: the centre of slot 0 is "b"'s.
    expect(detect(args(50)).map((hit) => hit.id)).toEqual(["b"]);
    expect(centre()).toBe("b");
    expect(detect(args(250)).map((hit) => hit.id)).toEqual(["b"]);
    expect(centre()).toBe("c");
    // A new drag starts unsorted: slot 0 is "a"'s own again.
    reset();
    expect(detect(args(50)).map((hit) => hit.id)[0]).toBe("a");
    expect(centre()).toBeNull();
  });

  it("lets a new reorder target settle before the tiles slide", () => {
    let time = 0;
    const { detect, settling } = intentCollision(
      () => true,
      () => time,
    );
    expect(detect(args(50)).map((hit) => hit.id)[0]).toBe("a");
    // Past "a"'s edge, closest to "b": "a" holds until "b" has settled.
    expect(detect(args(110)).map((hit) => hit.id)).toEqual(["a"]);
    expect(settling()).toBe("b");
    time = REORDER_SETTLE_MS - 1;
    expect(detect(args(112)).map((hit) => hit.id)).toEqual(["a"]);
    time = REORDER_SETTLE_MS;
    expect(detect(args(112)).map((hit) => hit.id)[0]).toBe("b");
    expect(settling()).toBeNull();
  });

  it("restarts the wait while the pointer is still moving", () => {
    let time = 0;
    const { detect } = intentCollision(
      () => true,
      () => time,
    );
    detect(args(50));
    detect(args(110));
    time = 100;
    expect(detect(args(118)).map((hit) => hit.id)).toEqual(["a"]);
    time = 200;
    expect(detect(args(119)).map((hit) => hit.id)).toEqual(["a"]);
    time = 250;
    expect(detect(args(119)).map((hit) => hit.id)[0]).toBe("b");
  });

  it("never merges what cannot merge", () => {
    const { detect, centre } = intentCollision(() => false);
    expect(detect(args(250)).map((hit) => hit.id)[0]).toBe("c");
    expect(centre()).toBeNull();
  });
});
