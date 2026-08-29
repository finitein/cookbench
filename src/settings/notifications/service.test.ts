import { describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { sendTestNotification } from "./service";

describe("sendTestNotification", () => {
  it("uses only an opaque destination id and never a secret", async () => {
    invoke.mockResolvedValueOnce(undefined);
    await sendTestNotification("slack");
    expect(invoke).toHaveBeenCalledWith("send_test_notification", { destination: "slack" });
    expect(JSON.stringify(invoke.mock.calls)).not.toMatch(/token|webhook|https?:/i);
  });
});
