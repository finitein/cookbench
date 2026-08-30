import { describe, expect, it, vi } from "vitest";
import {
  clampGlobalBarSize,
  globalBarWindowSize,
  prepareNativeGlobalBarDocument,
  recordGlobalBarPosition,
} from "./globalBarWindow";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

describe("global bar window sizing", () => {
  it("keeps the transparent native hit area tightly bounded", () => {
    expect(clampGlobalBarSize({ width: 680.1, height: 104 })).toEqual({ width: 681, height: 104 });
    expect(clampGlobalBarSize({ width: 10_000, height: 1 })).toEqual({ width: 900, height: 80 });
    expect(clampGlobalBarSize({ width: 900, height: 10_000 })).toEqual({ width: 900, height: 720 });
  });

  it("records a drag as a Cookbench window preference", async () => {
    invoke.mockResolvedValue(undefined);
    await recordGlobalBarPosition(42, 84);
    expect(invoke).toHaveBeenCalledWith("record_global_bar_position", { x: 42, y: 84 });
  });

  it("marks the native document so viewport resizing cannot shrink the bar repeatedly", () => {
    const root = document.createElement("html");
    prepareNativeGlobalBarDocument(root);
    expect(root).toHaveAttribute("data-cookbench-native", "true");
  });

  it("keeps room for the shell edge and shadow without native scrollbars", () => {
    expect(globalBarWindowSize({ width: 766, height: 616 })).toEqual({ width: 790, height: 640 });
  });
});
