import { describe, expect, it, vi } from "vitest";
import {
  attachGlobalBarDragHandle,
  clampGlobalBarSize,
  createGlobalBarDockController,
  type GlobalBarDockTransport,
  intrinsicGlobalBarMinimumHeight,
  prepareNativeGlobalBarDocument,
  recordGlobalBarPosition,
  recordGlobalBarSize,
  setGlobalBarMinimumSize,
} from "./globalBarWindow";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

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
    const startDrag = vi.fn();
    const detach = attachGlobalBarDragHandle(surface, startDrag);

    blank.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true }));
    button.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true }));

    expect(startDrag).toHaveBeenCalledTimes(1);
    expect(surface).not.toHaveAttribute("data-tauri-drag-region");
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

describe("global bar dock controller", () => {
  const expanded = { phase: "dockedExpanded" as const, docked: true, collapsed: false, bestEffort: false };
  function transport(): GlobalBarDockTransport {
    return {
      getState: vi.fn().mockResolvedValue(expanded), listen: vi.fn().mockResolvedValue(() => {}),
      startDrag: vi.fn().mockResolvedValue({ token: 7, completed: false }), finishDrag: vi.fn().mockResolvedValue(expanded),
      setGuards: vi.fn().mockResolvedValue(expanded), collapse: vi.fn().mockResolvedValue({ ...expanded, phase: "dockedCollapsed", collapsed: true }),
      reveal: vi.fn().mockResolvedValue(expanded), refreshGeometry: vi.fn().mockResolvedValue(expanded),
    };
  }

  it("collapses only after 600ms with every guard clear", async () => {
    vi.useFakeTimers(); const native = transport(); const controller = createGlobalBarDockController(native);
    await controller.initialize();
    await vi.advanceTimersByTimeAsync(599); expect(native.collapse).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1); expect(native.collapse).toHaveBeenCalledOnce();
    controller.dispose(); vi.useRealTimers();
  });

  it("finishes a token exactly once when pointerup comes before native start resolves", async () => {
    let resolve!: (result: { token: number; completed: boolean }) => void;
    const native = transport(); native.startDrag = vi.fn(() => new Promise<{ token: number; completed: boolean }>((done) => { resolve = done; }));
    const controller = createGlobalBarDockController(native);
    controller.start(); controller.endDrag(); resolve({ token: 9, completed: false }); await Promise.resolve(); await Promise.resolve();
    expect(native.finishDrag).toHaveBeenCalledTimes(1); expect(native.finishDrag).toHaveBeenCalledWith(9);
    controller.dispose();
  });

  it("does not finish a platform-completed drag or accept duplicate starts", async () => {
    const native = transport();
    native.startDrag = vi.fn().mockResolvedValue({ token: 4, completed: true, state: expanded });
    const controller = createGlobalBarDockController(native);
    controller.start(); controller.start(); controller.endDrag();
    await Promise.resolve(); await Promise.resolve();
    expect(native.startDrag).toHaveBeenCalledTimes(1);
    expect(native.finishDrag).not.toHaveBeenCalled();
    controller.dispose();
  });

  it("does not refresh while a start or finish is pending", async () => {
    let resolveStart!: (result: { token: number; completed: boolean }) => void;
    const native = transport();
    native.startDrag = vi.fn(() => new Promise<{ token: number; completed: boolean }>((done) => { resolveStart = done; }));
    const controller = createGlobalBarDockController(native);
    controller.start(); controller.refresh();
    expect(native.refreshGeometry).not.toHaveBeenCalled();
    resolveStart({ token: 3, completed: false }); await Promise.resolve();
    controller.endDrag(); controller.refresh();
    expect(native.refreshGeometry).not.toHaveBeenCalled();
    await Promise.resolve(); controller.dispose();
  });
});
