import "@testing-library/jest-dom/vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { RemoteSourcesPanel } from "./RemoteSourcesPanel";
import * as service from "./service";

vi.mock("./service", () => ({
  configureRemoteSource: vi.fn(async () => []),
  getRemoteSources: vi.fn(async () => []),
  removeRemoteSource: vi.fn(async () => []),
}));

beforeEach(() => vi.clearAllMocks());

describe("RemoteSourcesPanel", () => {
  it("treats an empty Session roots field as automatic discovery", async () => {
    render(<RemoteSourcesPanel />);
    fireEvent.change(screen.getByRole("textbox", { name: "SSH alias" }), {
      target: { value: "fixture-host" },
    });

    const add = screen.getByRole("button", { name: "Add" });
    expect(add).toBeEnabled();
    fireEvent.click(add);

    await waitFor(() => expect(service.configureRemoteSource).toHaveBeenCalledWith({
      id: null,
      alias: "fixture-host",
      sessionRoots: [],
      enabled: true,
      bridgeEnabled: false,
      bridgeBinaryPath: null,
    }));
    expect(await screen.findByRole("status")).toHaveTextContent("SSH source saved.");
  });

  it("surfaces backend validation errors instead of a generic failure", async () => {
    vi.mocked(service.configureRemoteSource).mockRejectedValueOnce(
      "SSH host aliases must be nonempty, option-free, and whitespace-free",
    );
    render(<RemoteSourcesPanel />);
    fireEvent.change(screen.getByRole("textbox", { name: "SSH alias" }), {
      target: { value: "bad alias" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Add" }));

    expect(await screen.findByRole("status")).toHaveTextContent(
      "SSH host aliases must be nonempty, option-free, and whitespace-free",
    );
  });

  it("removes a configured source and reports success", async () => {
    vi.mocked(service.getRemoteSources).mockResolvedValueOnce([
      {
        id: "fixture-host",
        alias: "fixture-host",
        sessionRoots: [],
        enabled: true,
        bridgeEnabled: false,
        bridgeBinaryPath: null,
      },
    ]);
    vi.mocked(service.removeRemoteSource).mockResolvedValueOnce([]);
    render(<RemoteSourcesPanel />);
    expect(await screen.findByText("fixture-host")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Remove" }));
    await waitFor(() => expect(service.removeRemoteSource).toHaveBeenCalledWith("fixture-host"));
    expect(await screen.findByRole("status")).toHaveTextContent("SSH source removed.");
  });
});
