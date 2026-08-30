import { describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { getLocalSourceStatus } from "./service";

describe("local source status service", () => {
  it("requests only the redacted source status snapshot", async () => {
    invoke.mockResolvedValue({ sources: [] });
    await getLocalSourceStatus();

    expect(invoke).toHaveBeenCalledWith("get_local_source_status");
    expect(JSON.stringify(invoke.mock.calls)).not.toMatch(/prompt|command|terminal|password|token|error/i);
  });
});
