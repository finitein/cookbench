import { describe, expect, it, vi } from "vitest";
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
import { acknowledgeCookedStove, archiveStove, clearCookedStove, getArchivedSessions, restoreArchivedSession, setStovePinned, StoveSync, subscribeToStoves, type StoveTransport } from "./stoves";
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
  pinned: false,
};

describe("StoveSync", () => {
  it("normalizes canonical attention order without losing unknown or duplicate entries", () => {
    const second = { ...stove, id: "local:machine:codex:session-2" };
    const sync = new StoveSync();

    sync.replace({
      revision: 3,
      stoves: [stove, second],
      attentionOrder: [second.id, "unknown", second.id],
    });

    expect(sync.current().attentionOrder).toEqual([second.id, stove.id]);
    expect(sync.current().stoves.map((entry) => entry.id)).toEqual([second.id, stove.id]);
  });

  it("uses a deterministic legacy fallback when an old fixture omits attention order", () => {
    const second = { ...stove, id: "local:machine:codex:session-2" };
    const sync = new StoveSync();

    sync.replace({ revision: 3, stoves: [stove, second] });

    expect(sync.current().attentionOrder).toEqual([stove.id, second.id]);
  });

  it("does not roll back a newer revision when gap recovery snapshots resolve out of order", () => {
    const sync = new StoveSync();
    sync.replace({ revision: 8, stoves: [stove], attentionOrder: [stove.id] });

    const stale = sync.replace({ revision: 6, stoves: [], attentionOrder: [] });

    expect(stale).toEqual({ revision: 8, stoves: [stove], attentionOrder: [stove.id] });
    expect(sync.current()).toEqual({ revision: 8, stoves: [stove], attentionOrder: [stove.id] });
  });

  it("keeps only sanitized wire fields and applies ordered changes", () => {
    const sync = new StoveSync();
    sync.replace({ revision: 3, stoves: [stove] });
    expect(sync.apply({ revision: 4, stove: { ...stove, state: "cooking", retainedCompletion: false }, removedStoveId: null, attentionOrder: [stove.id] })).toBe("applied");
    expect(sync.current().stoves[0]).toMatchObject({ harness: { id: "codex" }, host: { kind: "local" }, state: "cooking" });
    expect(JSON.stringify(sync.current())).not.toMatch(/transcript|prompt|command/i);
  });

  it("detects an event revision gap instead of inferring a missing change", () => {
    const sync = new StoveSync();
    sync.replace({ revision: 1, stoves: [] });
    expect(sync.apply({ revision: 3, stove, removedStoveId: null, attentionOrder: [stove.id] })).toBe("gap");
    expect(sync.current()).toEqual({ revision: 1, stoves: [], attentionOrder: [] });
  });

  it("fetches another snapshot after a gap", async () => {
    let handler: ((change: { revision: number; stove: StoveWire | null; removedStoveId: string | null; attentionOrder?: string[] }) => void) | undefined;
    const snapshots: StoveSnapshot[] = [{ revision: 1, stoves: [] }, { revision: 3, stoves: [stove] }];
    const transport: StoveTransport = {
      snapshot: vi.fn(async () => snapshots.shift()!),
      listen: vi.fn(async (next) => { handler = next; return () => {}; }),
    };
    const received: StoveSnapshot[] = [];
    await subscribeToStoves((next) => received.push(next), transport);
    handler?.({ revision: 3, stove, removedStoveId: null, attentionOrder: [stove.id] });
    await vi.waitFor(() => expect(received).toHaveLength(2));
    expect(received[1]).toEqual({ revision: 3, stoves: [stove], attentionOrder: [stove.id] });
  });

  it("retries recovery when a newer gap arrives before the first snapshot resolves", async () => {
    let handler: ((change: { revision: number; stove: StoveWire | null; removedStoveId: string | null; attentionOrder?: string[] }) => void) | undefined;
    const resolvers: Array<(snapshot: StoveSnapshot) => void> = [];
    const transport: StoveTransport = {
      snapshot: vi.fn()
        .mockResolvedValueOnce({ revision: 1, stoves: [] })
        .mockImplementation(() => new Promise<StoveSnapshot>((resolve) => { resolvers.push(resolve); })),
      listen: vi.fn(async (next) => { handler = next; return () => {}; }),
    };
    const received: StoveSnapshot[] = [];
    await subscribeToStoves((next) => received.push(next), transport);

    handler?.({ revision: 3, stove, removedStoveId: null, attentionOrder: [stove.id] });
    handler?.({ revision: 5, stove, removedStoveId: null, attentionOrder: [stove.id] });
    expect(transport.snapshot).toHaveBeenCalledTimes(2);

    resolvers.shift()?.({ revision: 3, stoves: [stove], attentionOrder: [stove.id] });
    await vi.waitFor(() => expect(transport.snapshot).toHaveBeenCalledTimes(3));
    resolvers.shift()?.({ revision: 5, stoves: [stove], attentionOrder: [stove.id] });
    await vi.waitFor(() => expect(received.at(-1)).toEqual({ revision: 5, stoves: [stove], attentionOrder: [stove.id] }));
  });

  it("replays queued startup gaps through the same recovery loop", async () => {
    let handler: ((change: { revision: number; stove: StoveWire | null; removedStoveId: string | null; attentionOrder?: string[] }) => void) | undefined;
    const resolvers: Array<(snapshot: StoveSnapshot) => void> = [];
    const transport: StoveTransport = {
      snapshot: vi.fn().mockImplementation(() => new Promise<StoveSnapshot>((resolve) => { resolvers.push(resolve); })),
      listen: vi.fn(async (next) => { handler = next; return () => {}; }),
    };
    const received: StoveSnapshot[] = [];
    const subscription = subscribeToStoves((next) => received.push(next), transport);
    await vi.waitFor(() => expect(handler).toBeDefined());
    await vi.waitFor(() => expect(resolvers).toHaveLength(1));
    handler?.({ revision: 3, stove, removedStoveId: null, attentionOrder: [stove.id] });
    handler?.({ revision: 5, stove, removedStoveId: null, attentionOrder: [stove.id] });
    resolvers.shift()?.({ revision: 1, stoves: [] });
    await subscription;
    await vi.waitFor(() => expect(transport.snapshot).toHaveBeenCalledTimes(2));
    resolvers.shift()?.({ revision: 3, stoves: [stove], attentionOrder: [stove.id] });
    await vi.waitFor(() => expect(transport.snapshot).toHaveBeenCalledTimes(3));
    resolvers.shift()?.({ revision: 5, stoves: [stove], attentionOrder: [stove.id] });
    await vi.waitFor(() => expect(received.at(-1)?.revision).toBe(5));
  });

  it("delivers the authoritative snapshot when live event registration is unavailable", async () => {
    const transport: StoveTransport = {
      snapshot: vi.fn(async () => ({ revision: 56, stoves: [stove] })),
      listen: vi.fn(async () => { throw new Error("event listen denied"); }),
    };
    const received: StoveSnapshot[] = [];

    const unlisten = await subscribeToStoves((next) => received.push(next), transport);

    expect(transport.snapshot).toHaveBeenCalledOnce();
    expect(received).toEqual([{ revision: 56, stoves: [stove], attentionOrder: [stove.id] }]);
    expect(() => unlisten()).not.toThrow();
  });
});

describe("stove commands", () => {
  it("uses only Stove ids for pin, archive, and restore commands", async () => {
    invoke.mockResolvedValue(undefined);
    await clearCookedStove(stove.id);
    await setStovePinned(stove.id, true);
    await archiveStove(stove.id);
    await getArchivedSessions();
    await restoreArchivedSession(stove.id);
    await acknowledgeCookedStove(stove.id);

    expect(invoke).toHaveBeenNthCalledWith(1, "clear_cooked_stove", { stoveId: stove.id });
    expect(invoke).toHaveBeenNthCalledWith(2, "set_stove_pinned", { stoveId: stove.id, pinned: true });
    expect(invoke).toHaveBeenNthCalledWith(3, "archive_stove", { stoveId: stove.id });
    expect(invoke).toHaveBeenNthCalledWith(4, "get_archived_sessions");
    expect(invoke).toHaveBeenNthCalledWith(5, "restore_archived_session", { stoveId: stove.id });
    expect(invoke).toHaveBeenNthCalledWith(6, "acknowledge_cooked_stove", { stoveId: stove.id });
  });
});
