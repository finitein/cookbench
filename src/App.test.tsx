import "@testing-library/jest-dom/vitest";
import { render, screen } from "@testing-library/react";
import { vi } from "vitest";

const { useGlobalBarWindow, useLocalAlert } = vi.hoisted(() => ({ useGlobalBarWindow: vi.fn(), useLocalAlert: vi.fn(() => null) }));
vi.mock("./hooks/useGlobalBarWindow", () => ({ useGlobalBarWindow }));
vi.mock("./hooks/useDisplaySettings", () => ({ useDisplaySettings: () => null }));
vi.mock("./services/localAlerts", () => ({
  useLocalAlert,
  LOCAL_ALERT_TEST_STOVE_ID: "__cookbench_test__",
}));

import App from "./App";

describe("App", () => {
  it("renders the global bar without inventing an empty stove", () => {
    render(<App />);

    expect(screen.getByLabelText("Cookbench global bar with 0 stoves")).toBeInTheDocument();
    expect(screen.queryAllByRole("listitem")).toHaveLength(0);
    expect(useGlobalBarWindow).toHaveBeenCalledOnce();
    expect(useLocalAlert).toHaveBeenCalledOnce();
  });
});
