import "@testing-library/jest-dom/vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { DisplaySettingsPanel } from "./DisplaySettingsPanel";

const { getDisplaySettings, configureDisplaySettings, closeDetachedBar } = vi.hoisted(() => ({
  getDisplaySettings: vi.fn(),
  configureDisplaySettings: vi.fn(),
  closeDetachedBar: vi.fn(),
}));

vi.mock("./service", () => ({
  getDisplaySettings,
  configureDisplaySettings,
  closeDetachedBar,
}));

describe("DisplaySettingsPanel", () => {
  beforeEach(() => {
    getDisplaySettings.mockResolvedValue({
      globalBarVisible: true,
      globalBarPlacement: "topCenter",
      globalBarSize: "standard",
      detachedBars: [{ stoveId: "host-a:session-1" }],
    });
    configureDisplaySettings.mockImplementation(async (input) => ({
      ...input,
      detachedBars: [{ stoveId: "host-a:session-1" }],
    }));
    closeDetachedBar.mockResolvedValue(true);
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
        globalBarSize: "standard",
      });
    });
    expect(screen.getByRole("button", { name: "Close independent Bar host-a:session-1" })).toBeInTheDocument();
  });

  it("lets people choose a comfortable global Bar width", async () => {
    render(<DisplaySettingsPanel />);
    const size = await screen.findByRole("combobox", { name: "Size" });
    fireEvent.change(size, { target: { value: "wide" } });

    await waitFor(() => {
      expect(configureDisplaySettings).toHaveBeenCalledWith({
        globalBarVisible: true,
        globalBarPlacement: "topCenter",
        globalBarSize: "wide",
      });
    });
  });

  it("closes only the selected independent Bar", async () => {
    render(<DisplaySettingsPanel />);
    fireEvent.click(await screen.findByRole("button", { name: "Close independent Bar host-a:session-1" }));

    await waitFor(() => expect(closeDetachedBar).toHaveBeenCalledWith("host-a:session-1"));
  });
});
