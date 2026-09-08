import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  planner: vi.fn(),
  minimum: vi.fn().mockResolvedValue(undefined),
  observe: vi.fn(),
  unobserve: vi.fn(),
  workArea: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    outerSize: vi.fn().mockResolvedValue({ width: 280, height: 800 }),
    scaleFactor: vi.fn().mockResolvedValue(1),
    onResized: vi.fn().mockResolvedValue(() => {}),
  }),
}));
vi.mock("../services/globalBarWindow", async () => {
  const actual = await vi.importActual<typeof import("../services/globalBarWindow")>("../services/globalBarWindow");
  return {
    ...actual,
    getGlobalBarWorkArea: mocks.workArea,
    preferredFullWidthForGlobalBarWorkArea: mocks.planner,
    setGlobalBarMinimumSize: mocks.minimum,
  };
});

import { useGlobalBarWindow } from "./useGlobalBarWindow";

class TestResizeObserver {
  constructor(_callback: ResizeObserverCallback) {}
  observe = mocks.observe;
  unobserve = mocks.unobserve;
  disconnect = vi.fn();
}

function Fixture({ minimal = false }: { minimal?: boolean }) {
  useGlobalBarWindow();
  return <section className={`global-bar${minimal ? " global-bar--minimal" : ""}`}>
    <div className="global-bar__brand" />
    {minimal ? <div key="minimal" className="global-bar__minimal" /> : <div key="full" className="global-bar__benches" />}
  </section>;
}

describe("useGlobalBarWindow", () => {
  afterEach(() => { cleanup(); vi.useRealTimers(); vi.restoreAllMocks(); vi.unstubAllGlobals(); mocks.planner.mockReset(); mocks.minimum.mockClear(); mocks.observe.mockClear(); mocks.unobserve.mockClear(); mocks.workArea.mockReset(); });

  it("passes the planned Full width to native sizing and rebinds mode content", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("ResizeObserver", TestResizeObserver);
    mocks.workArea.mockResolvedValue({ width: 1000, height: 800 });
    mocks.invoke.mockResolvedValue({ phase: "undocked", docked: false, collapsed: false, bestEffort: false });
    mocks.planner.mockReturnValue(796);
    const view = render(<Fixture />);
    await vi.runAllTimersAsync();
    expect(mocks.planner).toHaveBeenCalled();
    expect(mocks.minimum).toHaveBeenLastCalledWith(expect.anything(), expect.anything(), 796);

    const previousBenches = view.container.querySelector(".global-bar__benches");
    view.rerender(<Fixture minimal />);
    await vi.runAllTimersAsync();
    expect(mocks.unobserve).toHaveBeenCalledWith(previousBenches);
    expect(mocks.observe).toHaveBeenCalledWith(view.container.querySelector(".global-bar__minimal"));
    expect(mocks.minimum).toHaveBeenLastCalledWith(expect.anything(), expect.anything(), 280);
    view.rerender(<Fixture />);
    await vi.runAllTimersAsync();
    expect(mocks.minimum).toHaveBeenLastCalledWith(expect.anything(), expect.anything(), 796);
    view.unmount();
  });

  it("queues a second monitor lookup after a drag settles", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("ResizeObserver", TestResizeObserver);
    mocks.invoke.mockResolvedValue({ phase: "undocked", docked: false, collapsed: false, bestEffort: false });
    mocks.minimum.mockResolvedValue(undefined);
    let resolveFirst!: (value: { width: number; height: number }) => void;
    mocks.workArea.mockReturnValueOnce(new Promise((resolve) => { resolveFirst = resolve; }));
    mocks.workArea.mockResolvedValueOnce({ width: 1600, height: 900 });
    const view = render(<Fixture />);
    fireEvent.pointerDown(view.container.querySelector(".global-bar")!);
    await Promise.resolve();
    fireEvent.pointerUp(window);
    await vi.runAllTimersAsync();
    resolveFirst({ width: 1000, height: 800 });
    await vi.runAllTimersAsync();
    expect(mocks.workArea).toHaveBeenCalledTimes(2);
    expect(mocks.planner).toHaveBeenLastCalledWith(expect.anything(), { width: 1600, height: 900 }, expect.anything());
    view.unmount();
  });
});
