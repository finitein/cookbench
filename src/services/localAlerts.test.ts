import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

const { listen } = vi.hoisted(() => ({ listen: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

import {
  LOCAL_ALERT_DURATION_MS,
  LOCAL_ALERT_EVENT,
  isLocalAlertPayload,
  useLocalAlert,
} from "./localAlerts";

describe("local alerts", () => {
  it("accepts only the minimal native flash payload", () => {
    expect(isLocalAlertPayload({ stoveId: "stove-1", project: "sample", event: "cooked" })).toBe(true);
    expect(isLocalAlertPayload({ stoveId: "stove-1", event: "cooked" })).toBe(false);
    expect(isLocalAlertPayload({ stoveId: 1, project: "sample", event: "cooked" })).toBe(false);
  });

  it("keeps a completed Stove flashing until the matching Stove is dismissed", async () => {
    vi.useFakeTimers();
    let handler: ((value: { payload: { stoveId: string; project: string; event: string } }) => void) | undefined;
    const stop = vi.fn();
    listen.mockImplementation(async (event, next) => {
      expect(event).toBe(LOCAL_ALERT_EVENT);
      handler = next;
      return stop;
    });

    const view = renderHook(() => useLocalAlert());
    await act(async () => undefined);
    act(() => handler?.({ payload: { stoveId: "stove-1", project: "sample", event: "cooked" } }));
    expect(view.result.current.activeStoveId).toBe("stove-1");

    act(() => vi.advanceTimersByTime(LOCAL_ALERT_DURATION_MS * 10));
    expect(view.result.current.activeStoveId).toBe("stove-1");

    act(() => view.result.current.dismiss("another-stove"));
    expect(view.result.current.activeStoveId).toBe("stove-1");

    act(() => view.result.current.dismiss("stove-1"));
    expect(view.result.current.activeStoveId).toBeNull();

    view.unmount();
    expect(stop).toHaveBeenCalledOnce();
    vi.useRealTimers();
  });

  it("keeps non-completion alerts bounded", async () => {
    vi.useFakeTimers();
    let handler: ((value: { payload: { stoveId: string; project: string; event: string } }) => void) | undefined;
    listen.mockImplementation(async (_event, next) => {
      handler = next;
      return () => undefined;
    });

    const view = renderHook(() => useLocalAlert());
    await act(async () => undefined);
    act(() => handler?.({ payload: { stoveId: "stove-2", project: "sample", event: "failed" } }));
    expect(view.result.current.activeStoveId).toBe("stove-2");

    act(() => vi.advanceTimersByTime(LOCAL_ALERT_DURATION_MS));
    expect(view.result.current.activeStoveId).toBeNull();

    view.unmount();
    vi.useRealTimers();
  });
});
