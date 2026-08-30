import { describe, expect, it, vi } from "vitest";
import {
  clampGlobalBarSize,
  globalBarWindowSize,
  applyGlobalBarWidth,
  prepareNativeGlobalBarDocument,
  recordGlobalBarPosition,
} from "./globalBarWindow";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

describe("global bar window sizing", () => {
  it("keeps the transparent native hit area tightly bounded", () => {
    expect(clampGlobalBarSize({ width: 680.1, height: 104 })).toEqual({ width: 681, height: 104 });
    expect(clampGlobalBarSize({ width: 10_000, height: 1 })).toEqual({ width: 1024, height: 80 });
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

  it("maps familiar size choices to stable content widths", async () => {
    const { globalBarContentWidth } = await import("./globalBarWindow");
    expect(globalBarContentWidth("compact")).toBe(360);
    expect(globalBarContentWidth("standard")).toBe(640);
    expect(globalBarContentWidth("wide")).toBe(900);
  });

  it("keeps the chosen width in the native document while content changes", () => {
    const root = document.createElement("html");
    applyGlobalBarWidth("wide", root);
    expect(root).toHaveAttribute("data-cookbench-bar-size", "wide");
  });
});
