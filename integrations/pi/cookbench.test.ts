import { describe, expect, it } from "vitest";

import { createPiLifecycleEmitter } from "./cookbench";

describe("Pi Cookbench extension", () => {
  it("emits a bounded, content-free lifecycle envelope", async () => {
    const received: unknown[] = [];
    const emit = createPiLifecycleEmitter("pi-synthetic-001", (envelope) => received.push(envelope));

    await emit({ type: "todo_progress", completed: 1, total: 2 });

    expect(received).toEqual([{
      version: 1,
      sessionId: "pi-synthetic-001",
      event: { type: "todo_progress", completed: 1, total: 2 },
    }]);
    expect(JSON.stringify(received)).not.toContain("prompt");
  });

  it("drops malformed lifecycle events without invoking the sink", async () => {
    const received: unknown[] = [];
    const emit = createPiLifecycleEmitter("pi-synthetic-001", (envelope) => received.push(envelope));

    await emit({ type: "todo_progress", completed: 3, total: 2 });

    expect(received).toEqual([]);
  });

  it("does not propagate a delivery failure into Pi", async () => {
    const emit = createPiLifecycleEmitter("pi-synthetic-001", async () => {
      throw new Error("Cookbench is unavailable");
    });

    await expect(emit({ type: "tool_started" })).resolves.toBeUndefined();
  });
});
