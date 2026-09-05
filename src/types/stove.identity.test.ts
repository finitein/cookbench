import { describe, expect, it } from "vitest";

import { compactSessionIdentity, stoveSessionIdentity } from "./stove";

describe("stoveSessionIdentity", () => {
  it("keeps a distinctive trailing token for compound ids", () => {
    expect(stoveSessionIdentity({ id: "local:host:codex:session-12345678" })).toBe("#12345678");
  });

  it("prefers a readable prefix over mid-word suffix chops for short names", () => {
    expect(compactSessionIdentity("session-001")).toBe("session-");
    expect(stoveSessionIdentity({ id: "local:host:codex:session-001" })).toBe("#session-");
  });

  it("shows a UUID prefix instead of an opaque hex tail", () => {
    expect(
      stoveSessionIdentity({
        id: "local:host:grok_cli:01999999-aaaa-7bbb-8ccc-ddddeeeeffff",
      }),
    ).toBe("#01999999");
    expect(compactSessionIdentity("01999999-aaaa-7bbb-8ccc-ddddeeeeffff")).toBe("01999999");
  });

  it("keeps short ids intact", () => {
    expect(stoveSessionIdentity({ id: "local:host:pi:abcd" })).toBe("#abcd");
  });
});
