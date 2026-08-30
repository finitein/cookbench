import "@testing-library/jest-dom/vitest";
import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { SourcesStatusPanel } from "./SourcesStatusPanel";

const { getLocalSourceStatus } = vi.hoisted(() => ({ getLocalSourceStatus: vi.fn() }));

vi.mock("./service", () => ({ getLocalSourceStatus }));

describe("SourcesStatusPanel", () => {
  beforeEach(() => {
    getLocalSourceStatus.mockResolvedValue({
      sources: [
        { harness: "codex", label: "Codex", health: "healthy", rootDisplay: "~/.codex/sessions", discoveredSessions: 4, parserErrors: 0 },
        { harness: "claudeCode", label: "Claude Code", health: "degraded", rootDisplay: "~/.claude/projects", discoveredSessions: 1, parserErrors: 2 },
        { harness: "pi", label: "Pi", health: "unavailable", rootDisplay: "~/.pi/agent/sessions", discoveredSessions: 0, parserErrors: 0 },
      ],
    });
  });

  it("shows local adapter health and counts without rendering raw errors", async () => {
    render(<SourcesStatusPanel />);

    expect(await screen.findByRole("heading", { name: "Local Sources" })).toBeInTheDocument();
    expect(screen.getByText("Codex")).toBeInTheDocument();
    expect(screen.getByText("Watching")).toBeInTheDocument();
    expect(screen.getByText("4 sessions")).toBeInTheDocument();
    expect(screen.getByText("Claude Code")).toBeInTheDocument();
    expect(screen.getByText("Needs attention")).toBeInTheDocument();
    expect(screen.getByText("2 parsing issues")).toBeInTheDocument();
    expect(screen.getByText("Pi")).toBeInTheDocument();
    expect(screen.getByText("Unavailable")).toBeInTheDocument();
    expect(screen.getByText("~/.codex/sessions")).toBeInTheDocument();
    expect(screen.getByText("~/.claude/projects")).toBeInTheDocument();
  });

  it("reports a generic unavailable state when the status command cannot be read", async () => {
    getLocalSourceStatus.mockRejectedValueOnce(new Error("/Users/name/.claude secret prompt"));
    render(<SourcesStatusPanel />);

    expect(await screen.findByText("Local source status is unavailable.")).toBeInTheDocument();
    expect(screen.queryByText(/secret prompt/)).not.toBeInTheDocument();
  });
});
