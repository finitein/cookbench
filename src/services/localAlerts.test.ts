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

  it("shows the received Stove for a bounded interval and releases the native listener", async () => {
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
    expect(view.result.current).toBe("stove-1");

    act(() => vi.advanceTimersByTime(LOCAL_ALERT_DURATION_MS));
    expect(view.result.current).toBeNull();

    view.unmount();
    expect(stop).toHaveBeenCalledOnce();
    vi.useRealTimers();
  });
});
