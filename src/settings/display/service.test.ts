import { describe, expect, it, vi } from "vitest";

const { invoke, listen } = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

import {
  closeDetachedBar,
  configureDisplaySettings,
  getDisplaySettings,
  subscribeToDisplaySettings,
} from "./service";

describe("display settings service", () => {
  it("uses Cookbench-only display commands without a harness control surface", async () => {
    invoke.mockResolvedValue({});
    await getDisplaySettings();
    await configureDisplaySettings({
      globalBarVisible: false,
      globalBarPlacement: "bottomCenter",
      hoverDetailsEnabled: false,
    });
    await closeDetachedBar("remote-a:session-1");

    expect(invoke).toHaveBeenNthCalledWith(1, "get_display_settings");
    expect(invoke).toHaveBeenNthCalledWith(2, "configure_display_settings", {
      input: {
        globalBarVisible: false,
        globalBarPlacement: "bottomCenter",
        hoverDetailsEnabled: false,
      },
    });
    expect(invoke).toHaveBeenNthCalledWith(3, "close_detached_bar", { stoveId: "remote-a:session-1" });
    expect(JSON.stringify(invoke.mock.calls)).not.toMatch(/prompt|command|terminal|password|token/i);
  });

  it("delivers the saved preference and subsequent live updates", async () => {
    const initial = {
      globalBarVisible: true,
      globalBarPlacement: "topCenter" as const,
      hoverDetailsEnabled: false,
      detachedBars: [],
    };
    const changed = { ...initial, hoverDetailsEnabled: true };
    invoke.mockResolvedValue(initial);
    let handler: ((event: { payload: typeof initial }) => void) | undefined;
    const unlisten = vi.fn();
    listen.mockImplementation(async (_event, next) => {
      handler = next;
      return unlisten;
    });
    const delivered = vi.fn();

    const cleanup = await subscribeToDisplaySettings(delivered);
    handler?.({ payload: changed });

    expect(delivered).toHaveBeenNthCalledWith(1, initial);
    expect(delivered).toHaveBeenNthCalledWith(2, changed);
    cleanup();
    expect(unlisten).toHaveBeenCalledOnce();
  });
});
