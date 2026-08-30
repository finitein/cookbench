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
  });
});
