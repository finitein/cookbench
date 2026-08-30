import { describe, expect, it, vi } from "vitest";
import { StoveSync, subscribeToStoves, type StoveTransport } from "./stoves";
import type { StoveSnapshot, StoveWire } from "../types/stove";

const stove: StoveWire = {
  id: "local:machine:codex:session-1",
  harness: { id: "codex", label: "Codex" },
  host: { kind: "local", id: "machine" },
  projectRoot: "/workspace/demo",
  projectLabel: "demo",
  projectRootDisplay: "/workspace/demo",
  taskTitle: "Synthetic task",
  currentAction: "Checking fixture",
  nextAction: null,
  elapsedMs: 1000,
  state: "cooked",
  progress: { completed: 2, total: 2, provenance: "structuredSession" },
  locatorCapability: "available",
  retainedCompletion: true,
};

describe("StoveSync", () => {
  it("keeps only sanitized wire fields and applies ordered changes", () => {
    const sync = new StoveSync();
    sync.replace({ revision: 3, stoves: [stove] });
    expect(sync.apply({ revision: 4, stove: { ...stove, state: "cooking", retainedCompletion: false }, removedStoveId: null })).toBe("applied");
    expect(sync.current().stoves[0]).toMatchObject({ harness: { id: "codex" }, host: { kind: "local" }, state: "cooking" });
    expect(JSON.stringify(sync.current())).not.toMatch(/transcript|prompt|command/i);
  });

  it("detects an event revision gap instead of inferring a missing change", () => {
    const sync = new StoveSync();
    sync.replace({ revision: 1, stoves: [] });
    expect(sync.apply({ revision: 3, stove, removedStoveId: null })).toBe("gap");
    expect(sync.current()).toEqual({ revision: 1, stoves: [] });
  });

  it("fetches another snapshot after a gap", async () => {
    let handler: ((change: { revision: number; stove: StoveWire | null; removedStoveId: string | null }) => void) | undefined;
    const snapshots: StoveSnapshot[] = [{ revision: 1, stoves: [] }, { revision: 3, stoves: [stove] }];
    const transport: StoveTransport = {
      snapshot: vi.fn(async () => snapshots.shift()!),
      listen: vi.fn(async (next) => { handler = next; return () => {}; }),
    };
    const received: StoveSnapshot[] = [];
    await subscribeToStoves((next) => received.push(next), transport);
    handler?.({ revision: 3, stove, removedStoveId: null });
    await vi.waitFor(() => expect(received).toHaveLength(2));
    expect(received[1]).toEqual({ revision: 3, stoves: [stove] });
  });

  it("delivers the authoritative snapshot when live event registration is unavailable", async () => {
    const transport: StoveTransport = {
      snapshot: vi.fn(async () => ({ revision: 56, stoves: [stove] })),
      listen: vi.fn(async () => { throw new Error("event listen denied"); }),
    };
    const received: StoveSnapshot[] = [];

    const unlisten = await subscribeToStoves((next) => received.push(next), transport);

    expect(transport.snapshot).toHaveBeenCalledOnce();
    expect(received).toEqual([{ revision: 56, stoves: [stove] }]);
    expect(() => unlisten()).not.toThrow();
  });
});
