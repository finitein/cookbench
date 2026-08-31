import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { HookHealthPanel } from "./HookHealthPanel";

const status = [{
  harness: "codex" as const,
  label: "Codex",
  tier: "full" as const,
  integration: "automatic" as const,
  health: "notInstalled" as const,
  configDisplay: "~/.codex/config.toml",
  detail: "No Cookbench notify hook is installed.",
  canInstall: true,
  canRepair: false,
  canUninstall: false,
}];

const mocks = vi.hoisted(() => ({
  getHookStatus: vi.fn(),
  manageHook: vi.fn(),
}));

vi.mock("./service", () => ({ getHookStatus: mocks.getHookStatus, manageHook: mocks.manageHook }));

describe("HookHealthPanel", () => {
  it("offers preview and install without hiding configuration details", async () => {
    mocks.getHookStatus.mockResolvedValue(status);
    mocks.manageHook.mockResolvedValue({ status: { ...status[0], health: "healthy" as const }, changed: true, preview: "notify = [\\\"cookbench-hook\\\"]", backupDisplay: null });
    render(<HookHealthPanel />);
    expect(await screen.findByText("Codex")).toBeInTheDocument();
    expect(screen.getByText("~/.codex/config.toml")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Preview" }));
    await waitFor(() => expect(mocks.manageHook).toHaveBeenCalledWith("codex", "previewInstall"));
    expect(screen.getByLabelText("Hook configuration preview")).toHaveTextContent("cookbench-hook");
    expect(screen.getByText(/No harness configuration was changed/)).toBeInTheDocument();
  });

  it("distinguishes automatic, manual, and presence-only integrations", async () => {
    mocks.getHookStatus.mockResolvedValue([
      status[0],
      {
        ...status[0],
        harness: "qwen_code",
        label: "Qwen Code",
        tier: "full",
        integration: "manual",
        health: "noRecentEvents",
        canInstall: false,
      },
      {
        ...status[0],
        harness: "workbuddy",
        label: "Tencent WorkBuddy",
        tier: "experimental",
        integration: "presenceOnly",
        health: "detected",
        canInstall: false,
      },
    ]);
    render(<HookHealthPanel />);
    expect(await screen.findByText("Qwen Code")).toBeInTheDocument();
    expect(screen.getByText("Manual setup")).toBeInTheDocument();
    expect(screen.getByText("Presence only")).toBeInTheDocument();
    expect(screen.getByText("Experimental")).toBeInTheDocument();
    expect(screen.queryAllByRole("button", { name: "Install" })).toHaveLength(1);
  });
});
