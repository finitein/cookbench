import "@testing-library/jest-dom/vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { DisplaySettingsPanel } from "./DisplaySettingsPanel";

const {
  getDisplaySettings,
  configureDisplaySettings,
  closeDetachedBar,
  getLaunchAtLogin,
  setLaunchAtLogin,
} = vi.hoisted(() => ({
  getDisplaySettings: vi.fn(),
  configureDisplaySettings: vi.fn(),
  closeDetachedBar: vi.fn(),
  getLaunchAtLogin: vi.fn(),
  setLaunchAtLogin: vi.fn(),
}));

vi.mock("./service", () => ({
  getDisplaySettings,
  configureDisplaySettings,
  closeDetachedBar,
  getLaunchAtLogin,
  setLaunchAtLogin,
}));

describe("DisplaySettingsPanel", () => {
  beforeEach(() => {
    getDisplaySettings.mockResolvedValue({
      globalBarVisible: true,
      globalBarPlacement: "topCenter",
      detachedBars: [{ stoveId: "host-a:session-1" }],
    });
    configureDisplaySettings.mockImplementation(async (input) => ({
      ...input,
      detachedBars: [{ stoveId: "host-a:session-1" }],
    }));
    closeDetachedBar.mockResolvedValue(true);
    getLaunchAtLogin.mockResolvedValue({ enabled: false, defaultEnabled: false });
    setLaunchAtLogin.mockImplementation(async (enabled) => ({ enabled, defaultEnabled: false }));
  });

  it("keeps the global and independent Bar controls available together", async () => {
    render(<DisplaySettingsPanel />);

    expect(await screen.findByRole("heading", { name: "Display" })).toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: "Show global Bar" })).toBeChecked();
    expect(screen.getByRole("combobox", { name: "Placement" })).toHaveValue("topCenter");
    expect(screen.getByRole("button", { name: "Close independent Bar host-a:session-1" })).toBeInTheDocument();
  });

  it("persists hiding the global Bar without closing independent Bars", async () => {
    render(<DisplaySettingsPanel />);
    const globalBar = await screen.findByRole("checkbox", { name: "Show global Bar" });
    fireEvent.click(globalBar);

    await waitFor(() => {
      expect(configureDisplaySettings).toHaveBeenCalledWith({
        globalBarVisible: false,
        globalBarPlacement: "topCenter",
      });
    });
    expect(screen.getByRole("button", { name: "Close independent Bar host-a:session-1" })).toBeInTheDocument();
  });

  it("closes only the selected independent Bar", async () => {
    render(<DisplaySettingsPanel />);
    fireEvent.click(await screen.findByRole("button", { name: "Close independent Bar host-a:session-1" }));

    await waitFor(() => expect(closeDetachedBar).toHaveBeenCalledWith("host-a:session-1"));
  });

  it("keeps launch at login opt-in and persists an explicit toggle", async () => {
    render(<DisplaySettingsPanel />);

    const launchAtLogin = await screen.findByRole("checkbox", { name: "Launch Cookbench at login" });
    expect(launchAtLogin).not.toBeChecked();
    fireEvent.click(launchAtLogin);

    await waitFor(() => expect(setLaunchAtLogin).toHaveBeenCalledWith(true));
    expect(launchAtLogin).toBeChecked();
  });
});
