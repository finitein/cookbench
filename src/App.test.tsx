import "@testing-library/jest-dom/vitest";
import { act, render, screen } from "@testing-library/react";
import { beforeEach, vi } from "vitest";

const { activateStove, dismissLocalAlert, useGlobalBarWindow, useLocalAlert, useStoves } = vi.hoisted(() => ({
  activateStove: vi.fn(async () => ({ target: "terminal", status: "focused", resumeSessionId: null })),
  dismissLocalAlert: vi.fn(),
  useGlobalBarWindow: vi.fn(),
  useLocalAlert: vi.fn(),
  useStoves: vi.fn(),
}));
vi.mock("./hooks/useGlobalBarWindow", () => ({ useGlobalBarWindow }));
vi.mock("./hooks/useDisplaySettings", () => ({ useDisplaySettings: () => null }));
vi.mock("./hooks/useStoves", () => ({ useStoves }));
vi.mock("./services/locator", () => ({ activateStove }));
vi.mock("./services/localAlerts", () => ({
  useLocalAlert,
  LOCAL_ALERT_TEST_STOVE_ID: "__cookbench_test__",
}));

import App from "./App";
import { makeStove } from "./stories/GlobalBar.fixture";

describe("App", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useLocalAlert.mockReturnValue({ activeStoveId: null, dismiss: dismissLocalAlert });
    useStoves.mockReturnValue({ revision: 0, stoves: [] });
  });

  it("renders the global bar without inventing an empty stove", () => {
    render(<App />);

    expect(screen.getByLabelText("Cookbench global bar with 0 stoves")).toBeInTheDocument();
    expect(screen.queryAllByRole("listitem")).toHaveLength(0);
    expect(useGlobalBarWindow).toHaveBeenCalledOnce();
    expect(useLocalAlert).toHaveBeenCalledOnce();
  });

  it("acknowledges a completion alert before returning to its Stove", async () => {
    const stove = makeStove(0, { state: "cooked" });
    useLocalAlert.mockReturnValue({ activeStoveId: stove.id, dismiss: dismissLocalAlert });
    useStoves.mockReturnValue({ revision: 1, stoves: [stove] });

    render(<App />);
    await act(async () => screen.getByTestId("stove").click());

    expect(dismissLocalAlert).toHaveBeenCalledWith(stove.id);
    expect(activateStove).toHaveBeenCalledWith(stove.id);
  });
});
