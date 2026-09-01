import "@testing-library/jest-dom/vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { DisplaySettingsPanel } from "./DisplaySettingsPanel";
import { I18nProvider } from "../../i18n/i18n";

const {
  getDisplaySettings,
  patchDisplaySettings,
  closeDetachedBar,
  getLaunchAtLogin,
  setLaunchAtLogin,
} = vi.hoisted(() => ({
  getDisplaySettings: vi.fn(),
  patchDisplaySettings: vi.fn(),
  closeDetachedBar: vi.fn(),
  getLaunchAtLogin: vi.fn(),
  setLaunchAtLogin: vi.fn(),
}));

vi.mock("./service", () => ({
  getDisplaySettings,
  patchDisplaySettings,
  closeDetachedBar,
  getLaunchAtLogin,
  setLaunchAtLogin,
}));

describe("DisplaySettingsPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getDisplaySettings.mockResolvedValue({
      globalBarVisible: true,
      globalBarPlacement: "topCenter",
      globalBarMode: "full",
      macStatusStoveCount: 3,
      macStatusAvailable: false,
      hoverDetailsEnabled: false,
      locale: "system",
      detachedBars: [{ stoveId: "host-a:session-1" }],
    });
    patchDisplaySettings.mockImplementation(async (input) => ({
      globalBarVisible: true, globalBarPlacement: "topCenter", globalBarMode: "full", macStatusStoveCount: 3, hoverDetailsEnabled: false, locale: "system",
      ...input,
      macStatusAvailable: false,
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
    expect(screen.getByRole("checkbox", { name: "Show details on hover" })).not.toBeChecked();
    expect(screen.getByRole("combobox", { name: "Placement" })).toHaveValue("topCenter");
    expect(screen.queryByText("macOS status bar")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Close independent Bar host-a:session-1" })).toBeInTheDocument();
  });

  it("persists hiding the global Bar without closing independent Bars", async () => {
    render(<DisplaySettingsPanel />);
    const globalBar = await screen.findByRole("checkbox", { name: "Show global Bar" });
    fireEvent.click(globalBar);

    await waitFor(() => {
      expect(patchDisplaySettings).toHaveBeenCalledWith({ globalBarVisible: false });
    });
    expect(screen.getByRole("button", { name: "Close independent Bar host-a:session-1" })).toBeInTheDocument();
  });

  it("keeps hover details off by default and saves an explicit opt-in", async () => {
    render(<DisplaySettingsPanel />);

    const hoverDetails = await screen.findByRole("checkbox", { name: "Show details on hover" });
    expect(hoverDetails).not.toBeChecked();
    fireEvent.click(hoverDetails);

    await waitFor(() => {
      expect(patchDisplaySettings).toHaveBeenCalledWith({ hoverDetailsEnabled: true });
    });
  });

  it("persists an explicit interface language selection", async () => {
    render(<DisplaySettingsPanel />);

    const language = await screen.findByRole("combobox", { name: "Language" });
    fireEvent.change(language, { target: { value: "zh-CN" } });

    await waitFor(() => expect(patchDisplaySettings).toHaveBeenCalledWith({ locale: "zh-CN" }));
  });

  it("persists the selected global Bar mode", async () => {
    render(<DisplaySettingsPanel />);
    fireEvent.click(await screen.findByRole("radio", { name: "Minimal" }));

    await waitFor(() => expect(patchDisplaySettings).toHaveBeenCalledWith({ globalBarMode: "minimal" }));
  });

  it("uses the native capability rather than browser detection for macOS status settings", async () => {
    getDisplaySettings.mockResolvedValue({
      globalBarVisible: true,
      globalBarPlacement: "topCenter",
      globalBarMode: "full",
      macStatusStoveCount: 3,
      macStatusAvailable: true,
      hoverDetailsEnabled: false,
      locale: "system",
      detachedBars: [],
    });
    render(<DisplaySettingsPanel />);
    const count = await waitFor(() => {
      const input = document.getElementById("mac-status-stove-count");
      expect(input).not.toBeNull();
      return input as HTMLInputElement;
    });
    expect(count).toHaveAttribute("min", "0");
    expect(count).toHaveAttribute("max", "8");
    expect(count).toHaveValue(3);
    fireEvent.change(count, { target: { value: "5" } });

    await waitFor(() => expect(patchDisplaySettings).toHaveBeenCalledWith(expect.objectContaining({
      macStatusStoveCount: 5,
    })));
  });

  it("serializes rapid display edits and sends each complete preference tuple", async () => {
    let resolveFirst: ((value: unknown) => void) | undefined;
    let resolveSecond: ((value: unknown) => void) | undefined;
    patchDisplaySettings
      .mockImplementationOnce(() => new Promise((resolve) => { resolveFirst = resolve; }))
      .mockImplementationOnce(() => new Promise((resolve) => { resolveSecond = resolve; }));
    render(<DisplaySettingsPanel />);

    fireEvent.click(await screen.findByRole("radio", { name: "Minimal" }));
    await waitFor(() => expect(screen.getByRole("radio", { name: "Minimal" })).toBeChecked());
    fireEvent.click(screen.getByRole("checkbox", { name: "Show details on hover" }));
    await waitFor(() => expect(patchDisplaySettings).toHaveBeenCalledTimes(1));
    resolveFirst?.({
      globalBarVisible: true, globalBarPlacement: "topCenter", globalBarMode: "minimal",
      macStatusStoveCount: 3, macStatusAvailable: false, hoverDetailsEnabled: false,
      locale: "system", detachedBars: [],
    });
    await waitFor(() => expect(patchDisplaySettings).toHaveBeenCalledTimes(2));
    expect(patchDisplaySettings).toHaveBeenLastCalledWith({ hoverDetailsEnabled: true });
    resolveSecond?.({
      globalBarVisible: true, globalBarPlacement: "topCenter", globalBarMode: "minimal",
      macStatusStoveCount: 3, macStatusAvailable: false, hoverDetailsEnabled: true,
      locale: "system", detachedBars: [],
    });
    await waitFor(() => expect(screen.getByRole("checkbox", { name: "Show details on hover" })).toBeChecked());
  });

  it("lets a failed older save yield to a newer desired preference", async () => {
    let rejectFirst: ((reason?: unknown) => void) | undefined;
    let resolveSecond: ((value: unknown) => void) | undefined;
    patchDisplaySettings
      .mockImplementationOnce(() => new Promise((_resolve, reject) => { rejectFirst = reject; }))
      .mockImplementationOnce(() => new Promise((resolve) => { resolveSecond = resolve; }));
    render(<DisplaySettingsPanel />);

    fireEvent.click(await screen.findByRole("radio", { name: "Minimal" }));
    await waitFor(() => expect(screen.getByRole("radio", { name: "Minimal" })).toBeChecked());
    fireEvent.click(screen.getByRole("checkbox", { name: "Show details on hover" }));
    rejectFirst?.(new Error("first save failed"));

    await waitFor(() => expect(patchDisplaySettings).toHaveBeenCalledTimes(2));
    resolveSecond?.({
      globalBarVisible: true, globalBarPlacement: "topCenter", globalBarMode: "minimal",
      macStatusStoveCount: 3, macStatusAvailable: false, hoverDetailsEnabled: true,
      locale: "system", detachedBars: [],
    });
    await waitFor(() => {
      expect(screen.getByRole("checkbox", { name: "Show details on hover" })).toBeChecked();
      expect(screen.getByRole("status")).toHaveTextContent("");
    });
  });

  it("reloads native settings and reports a latest save failure", async () => {
    patchDisplaySettings.mockRejectedValueOnce(new Error("save failed"));
    render(<DisplaySettingsPanel />);

    fireEvent.click(await screen.findByRole("radio", { name: "Minimal" }));

    await waitFor(() => {
      expect(screen.getByRole("radio", { name: "Full" })).toBeChecked();
      expect(screen.getByRole("status")).toHaveTextContent("Display settings could not be saved.");
    });
    expect(getDisplaySettings).toHaveBeenCalledTimes(2);
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

  it("renders an async error in the currently selected language", async () => {
    let rejectSettings: ((reason?: unknown) => void) | undefined;
    getDisplaySettings.mockReturnValueOnce(new Promise((_resolve, reject) => {
      rejectSettings = reject;
    }));
    const { rerender } = render(
      <I18nProvider preference="en">
        <DisplaySettingsPanel />
      </I18nProvider>,
    );

    rerender(
      <I18nProvider preference="zh-CN">
        <DisplaySettingsPanel />
      </I18nProvider>,
    );
    rejectSettings?.(new Error("unavailable"));

    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("显示设置不可用"));
  });
});
