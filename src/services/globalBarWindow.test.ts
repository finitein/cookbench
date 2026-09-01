import { describe, expect, it, vi } from "vitest";
import {
  attachGlobalBarDragHandle,
  clampGlobalBarSize,
  intrinsicGlobalBarMinimumHeight,
  prepareNativeGlobalBarDocument,
  recordGlobalBarPosition,
  recordGlobalBarSize,
  setGlobalBarMinimumSize,
} from "./globalBarWindow";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
const { startDragging } = vi.hoisted(() => ({ startDragging: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ startDragging }),
}));

describe("global bar window sizing", () => {
  it("keeps the transparent native hit area usable without imposing a preset width", () => {
    expect(clampGlobalBarSize({ width: 680.1, height: 104 })).toEqual({ width: 681, height: 104 });
    expect(clampGlobalBarSize({ width: 10_000, height: 1 })).toEqual({ width: 10_000, height: 80 });
    expect(clampGlobalBarSize({ width: 900, height: 10_000 })).toEqual({ width: 900, height: 10_000 });
  });

  it("records a drag as a Cookbench window preference", async () => {
    invoke.mockResolvedValue(undefined);
    await recordGlobalBarPosition(42, 84);
    expect(invoke).toHaveBeenCalledWith("record_global_bar_position", { x: 42, y: 84 });
  });

  it("drags from blank white surface without stealing interactive controls", () => {
    const surface = document.createElement("section");
    const blank = document.createElement("div");
    const button = document.createElement("button");
    surface.append(blank, button);
    const detach = attachGlobalBarDragHandle(surface);

    blank.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true }));
    button.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true }));

    expect(startDragging).toHaveBeenCalledTimes(1);
    detach();
  });

  it("marks the native document so viewport resizing cannot shrink the bar repeatedly", () => {
    const root = document.createElement("html");
    prepareNativeGlobalBarDocument(root);
    expect(root).toHaveAttribute("data-cookbench-native", "true");
  });

  it("persists a completed native resize without resetting the user width", async () => {
    invoke.mockResolvedValue(undefined);
    await recordGlobalBarSize({ width: 812, height: 248 });
    expect(invoke).toHaveBeenCalledWith("record_global_bar_size", { width: 812, height: 248 });
  });

  it("raises the native minimum height for all visible stoves", async () => {
    invoke.mockResolvedValue(undefined);
    await setGlobalBarMinimumSize({ width: 280, height: 248 });
    expect(invoke).toHaveBeenCalledWith("set_global_bar_minimum_size", {
      width: 280,
      height: 248,
      preferredHeight: undefined,
    });
  });

  it("measures content rather than locking the current native window height", () => {
    const bar = document.createElement("section");
    const brand = document.createElement("div");
    const benches = document.createElement("div");
    const tooltip = document.createElement("aside");
    brand.className = "global-bar__brand";
    benches.className = "global-bar__benches";
    tooltip.className = "global-bar__tooltip";
    bar.append(brand, benches, tooltip);
    vi.spyOn(bar, "getBoundingClientRect").mockReturnValue({ top: 20 } as DOMRect);
    vi.spyOn(brand, "getBoundingClientRect").mockReturnValue({ bottom: 92 } as DOMRect);
    vi.spyOn(benches, "getBoundingClientRect").mockReturnValue({ bottom: 176 } as DOMRect);
    vi.spyOn(tooltip, "getBoundingClientRect").mockReturnValue({ bottom: 302 } as DOMRect);

    expect(intrinsicGlobalBarMinimumHeight(bar)).toBe(293);
  });

  it("keeps a long priority menu measurable even from a 92px native viewport", () => {
    const bar = document.createElement("section");
    const minimal = document.createElement("div");
    const menu = document.createElement("div");
    minimal.className = "global-bar__minimal";
    menu.className = "stove-priority-menu";
    bar.append(minimal, menu);
    vi.spyOn(bar, "getBoundingClientRect").mockReturnValue({ top: 0, height: 92 } as DOMRect);
    vi.spyOn(minimal, "getBoundingClientRect").mockReturnValue({ bottom: 92 } as DOMRect);
    vi.spyOn(menu, "getBoundingClientRect").mockReturnValue({ bottom: 352, height: 260 } as DOMRect);
    expect(menu.getBoundingClientRect().height).toBeGreaterThan(0);
    expect(intrinsicGlobalBarMinimumHeight(bar)).toBe(363);
  });
});
