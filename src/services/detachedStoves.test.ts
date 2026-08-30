import { describe, expect, it, vi } from "vitest";
import type { StoveWire } from "../types/stove";
import { createPositionPersistence, detachedWindowLabel, stoveForDetachedWindow } from "./detachedStoves";

const stove = {
  id: "local:codex:session-42",
  harness: { id: "codex", label: "Codex" },
  host: { kind: "local", id: "machine" },
  projectRoot: "/safe/project",
  state: "cooked",
  progress: null,
  locatorCapability: "available",
  retainedCompletion: true,
  pinned: false,
} satisfies StoveWire;

describe("detached Stove routing", () => {
  it("routes a detached label to only its matching Stove", () => {
    const other = { ...stove, id: "local:pi:session-43", harness: { id: "pi", label: "Pi" } } satisfies StoveWire;
    expect(stoveForDetachedWindow([stove, other], detachedWindowLabel(stove.id))).toEqual(stove);
  });

  it("does not accept an arbitrary or non-matching label", () => {
    expect(stoveForDetachedWindow([stove], "main")).toBeUndefined();
    expect(detachedWindowLabel(stove.id)).toMatch(/^stove-[0-9a-f]+$/);
  });

  it("debounces drag positions and flushes the final position", () => {
    vi.useFakeTimers();
    const persist = vi.fn();
    const writer = createPositionPersistence(persist, 100);
    writer.schedule({ x: 1, y: 2 });
    writer.schedule({ x: 30, y: 40 });
    vi.advanceTimersByTime(99);
    expect(persist).not.toHaveBeenCalled();
    writer.flush();
    expect(persist).toHaveBeenLastCalledWith({ x: 30, y: 40 });
    vi.useRealTimers();
  });
});
