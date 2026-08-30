import "@testing-library/jest-dom/vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
const { getArchivedSessions, restoreArchivedSession } = vi.hoisted(() => ({
  getArchivedSessions: vi.fn(),
  restoreArchivedSession: vi.fn(),
}));
vi.mock("../../services/stoves", () => ({ getArchivedSessions, restoreArchivedSession }));
import { ArchiveSettingsPanel } from "./ArchiveSettingsPanel";

const archived = {
  id: "local:machine:codex:session-1",
  harness: { id: "codex", label: "Codex" },
  host: { kind: "local", id: "machine" },
  projectLabel: "demo",
  projectRootDisplay: "~/demo",
  sessionIdentity: "#session-1",
  lastState: "cooking",
  reason: "manual" as const,
  archivedAtMs: 1_725_000_000_000,
  sourceAvailable: true,
  pinned: false,
};

describe("ArchiveSettingsPanel", () => {
  it("shows only safe archive metadata and restores a session after success", async () => {
    getArchivedSessions.mockResolvedValueOnce([archived]);
    restoreArchivedSession.mockResolvedValueOnce(undefined);
    render(<ArchiveSettingsPanel />);

    await waitFor(() => expect(screen.getByRole("list", { name: "Archived sessions" })).toBeInTheDocument());
    expect(screen.getByText("demo")).toBeVisible();
    expect(screen.getByText(/Deleted manually/)).toBeVisible();
    fireEvent.click(screen.getByRole("button", { name: "Restore" }));
    await waitFor(() => expect(restoreArchivedSession).toHaveBeenCalledWith(archived.id));
    expect(screen.queryByRole("listitem")).not.toBeInTheDocument();
  });

  it("keeps an entry visible when the native source cannot be restored", async () => {
    getArchivedSessions.mockResolvedValueOnce([{ ...archived, sourceAvailable: false }]);
    restoreArchivedSession.mockRejectedValueOnce(new Error("missing"));
    render(<ArchiveSettingsPanel />);

    await waitFor(() => screen.getByRole("button", { name: "Restore" }));
    fireEvent.click(screen.getByRole("button", { name: "Restore" }));
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("no longer available"));
    expect(screen.getByRole("listitem")).toBeInTheDocument();
  });
});
