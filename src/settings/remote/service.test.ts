import { describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { configureRemoteSource, getRemoteSources, removeRemoteSource } from "./service";

describe("remote source service", () => {
  it("sends only an SSH alias and read-only roots", async () => {
    invoke.mockResolvedValue([]);
    await getRemoteSources();
    await configureRemoteSource({
      id: null,
      alias: "fixture-host",
      sessionRoots: ["/srv/sessions"],
      enabled: true,
      bridgeEnabled: false,
      bridgeBinaryPath: null,
    });
    await removeRemoteSource("fixture-host");

    expect(invoke).toHaveBeenNthCalledWith(1, "get_remote_sources");
    expect(invoke).toHaveBeenNthCalledWith(2, "configure_remote_source", {
      input: {
        id: null,
        alias: "fixture-host",
        sessionRoots: ["/srv/sessions"],
        enabled: true,
        bridgeEnabled: false,
        bridgeBinaryPath: null,
      },
    });
    expect(invoke).toHaveBeenNthCalledWith(3, "remove_remote_source", { id: "fixture-host" });
    expect(JSON.stringify(invoke.mock.calls)).not.toMatch(/password|privateKey|token/i);
  });
});
