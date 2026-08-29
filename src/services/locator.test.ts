import { describe, expect, it } from "vitest";
import { activateStove, type LocatorTransport } from "./locator";

describe("activateStove", () => {
  it("returns a focused result without exposing a locator", async () => {
    const transport: LocatorTransport = {
      activate: async (stoveId) => ({
        target: "exactPane",
        status: "focused",
        resumeSessionId: stoveId === "local:demo" ? null : "unexpected",
      }),
    };

    await expect(activateStove("local:demo", transport)).resolves.toEqual({
      target: "exactPane",
      status: "focused",
      resumeSessionId: null,
    });
  });

  it("preserves an honest visible resume fallback", async () => {
    const transport: LocatorTransport = {
      activate: async () => ({
        target: "resumeInstructions",
        status: "visibleFallback",
        resumeSessionId: "opaque-session-42",
      }),
    };

    await expect(activateStove("remote:demo", transport)).resolves.toMatchObject({
      target: "resumeInstructions",
      status: "visibleFallback",
      resumeSessionId: "opaque-session-42",
    });
  });

  it("reports an unavailable locator rather than inventing a focus target", async () => {
    const transport: LocatorTransport = {
      activate: async () => ({
        target: "unavailable",
        status: "unavailable",
        resumeSessionId: null,
      }),
    };

    await expect(activateStove("missing", transport)).resolves.toMatchObject({
      status: "unavailable",
      resumeSessionId: null,
    });
  });
});
