import { describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { closeDetachedBar, configureDisplaySettings, getDisplaySettings } from "./service";

describe("display settings service", () => {
  it("uses Cookbench-only display commands without a harness control surface", async () => {
    invoke.mockResolvedValue({});
    await getDisplaySettings();
    await configureDisplaySettings({ globalBarVisible: false, globalBarPlacement: "bottomCenter", globalBarSize: "compact" });
    await closeDetachedBar("remote-a:session-1");

    expect(invoke).toHaveBeenNthCalledWith(1, "get_display_settings");
    expect(invoke).toHaveBeenNthCalledWith(2, "configure_display_settings", {
      input: { globalBarVisible: false, globalBarPlacement: "bottomCenter", globalBarSize: "compact" },
    });
    expect(invoke).toHaveBeenNthCalledWith(3, "close_detached_bar", { stoveId: "remote-a:session-1" });
    expect(JSON.stringify(invoke.mock.calls)).not.toMatch(/prompt|command|terminal|password|token/i);
  });
});
