import { describe, expect, it } from "vitest";

import { makeStove } from "../stories/GlobalBar.fixture";
import { arrangeBenches, stoveCapacityForWidth } from "./benchLayout";

describe("stoveCapacityForWidth", () => {
  it("derives a whole Stove capacity from the usable bench width", () => {
    expect(stoveCapacityForWidth(0)).toBe(1);
    expect(stoveCapacityForWidth(85)).toBe(1);
    expect(stoveCapacityForWidth(86)).toBe(1);
    expect(stoveCapacityForWidth(175)).toBe(2);
    expect(stoveCapacityForWidth(262)).toBe(3);
  });
});

describe("arrangeBenches", () => {
  it("keeps harnesses together in one mixed bench when each harness fits one row", () => {
    const layout = arrangeBenches([
      makeStove(0),
      makeStove(1),
      makeStove(2),
      makeStove(3),
      makeStove(4),
      makeStove(5),
    ], 2);

    expect(layout.grouped).toBe(false);
    expect(layout.benches).toHaveLength(1);
    expect(layout.benches[0]).toMatchObject({ id: "all", label: "All sessions" });
    expect(layout.benches[0].stoves).toHaveLength(6);
  });

  it("splits into lightweight harness benches when one harness needs more than one row", () => {
    const layout = arrangeBenches([
      makeStove(0),
      makeStove(3),
      makeStove(6),
      makeStove(1),
      makeStove(2),
    ], 2);

    expect(layout.grouped).toBe(true);
    expect(layout.benches.map((bench) => [bench.id, bench.label, bench.stoves.length])).toEqual([
      ["codex", "Codex", 3],
      ["claudeCode", "Claude Code", 1],
      ["pi", "Pi", 1],
    ]);
  });

  it("preserves canonical input order rather than inventing a local state ranking", () => {
    const layout = arrangeBenches([
      makeStove(0, { state: "cooked" }),
      makeStove(3, { state: "needsHuman" }),
      makeStove(6, { state: "cooking" }),
      makeStove(9, { state: "planning" }),
      makeStove(12, { state: "failed" }),
      makeStove(15, { state: "disconnected" }),
    ], 2);

    expect(layout.benches[0].stoves.map((stove) => stove.id)).toEqual([
      "fixture:codex:0",
      "fixture:codex:3",
      "fixture:codex:6",
      "fixture:codex:9",
      "fixture:codex:12",
      "fixture:codex:15",
    ]);
  });
});
