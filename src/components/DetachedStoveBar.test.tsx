import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { StoveWire } from "../types/stove";
import { DetachedStoveBar } from "./DetachedStoveBar";

const stove: StoveWire = {
  id: "local:codex:session-safe-id",
  harness: { id: "codex", label: "Codex" },
  host: { kind: "local", id: "workstation" },
  projectRoot: "/workspace/sample",
  projectLabel: "sample",
  projectRootDisplay: "~/workspace/sample",
  taskTitle: "Current session",
  currentAction: "Preparing workspace",
  nextAction: null,
  elapsedMs: 3_000,
  state: "cooking",
  progress: { completed: 3, total: 8, provenance: "structuredSession" },
  locatorCapability: "available",
  retainedCompletion: false,
  pinned: false,
};

describe("DetachedStoveBar", () => {
  it("uses the same accessible burner for a single Stove", () => {
    render(<DetachedStoveBar stove={stove} />);

    expect(screen.getByRole("region", { name: "Detached Stove bar for Codex" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Detached Stove bar for Codex" }))
      .toHaveAttribute("data-tauri-drag-region");
    expect(screen.getByRole("button", { name: /Codex:/ })).toBeInTheDocument();
    expect(document.querySelector(".detached-stove-bar__identity")).not.toBeInTheDocument();
    expect(screen.getByLabelText("Codex")).toHaveTextContent("CX");
    expect(screen.getByTestId("stove")).toHaveAttribute("data-state", "cooking");
  });

  it("forwards activation and always exposes a close action", () => {
    const onActivate = vi.fn();
    const onClose = vi.fn();
    const onClear = vi.fn();
    render(<DetachedStoveBar stove={stove} onActivate={onActivate} onClose={onClose} onClear={onClear} />);

    fireEvent.click(screen.getByRole("button", { name: /Codex:/ }));
    fireEvent.click(screen.getByRole("button", { name: "Close detached Stove" }));
    fireEvent.click(screen.getByRole("button", { name: "Clear Codex Stove" }));

    expect(onActivate).toHaveBeenCalledWith(stove);
    expect(onClose).toHaveBeenCalledWith(stove);
    expect(onClear).toHaveBeenCalledWith(stove);
  });

  it("keeps pin and delete available in the detached view for a live Stove", () => {
    const onPin = vi.fn();
    const onArchive = vi.fn();
    render(<DetachedStoveBar stove={stove} onPin={onPin} onArchive={onArchive} />);

    fireEvent.click(screen.getByRole("button", { name: "Pin Codex Stove" }));
    fireEvent.click(screen.getByRole("button", { name: "Delete Codex Stove" }));
    expect(onPin).toHaveBeenCalledWith(stove);
    expect(onArchive).toHaveBeenCalledWith(stove);
  });

  it("does not render a clear control when manual clearing is unavailable", () => {
    render(<DetachedStoveBar stove={{ ...stove, state: "cooked", retainedCompletion: true }} />);

    expect(screen.queryByRole("button", { name: /Clear .* Stove/ })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Close detached Stove" })).toBeInTheDocument();
    expect(screen.getByTestId("stove")).toHaveAttribute("data-state", "cooked");
  });

  it("starts a native drag from the non-interactive surface", () => {
    const onStartDrag = vi.fn();
    render(<DetachedStoveBar stove={stove} onStartDrag={onStartDrag} />);

    fireEvent.pointerDown(screen.getByRole("region", { name: "Detached Stove bar for Codex" }), { button: 0 });

    expect(onStartDrag).toHaveBeenCalledOnce();
  });

  it("emphasizes its Stove only when the local alert targets it", () => {
    const view = render(<DetachedStoveBar stove={stove} activeAlertStoveId="another-stove" />);
    expect(document.querySelector(".stove-burner-wrap")).not.toHaveClass("stove-burner-wrap--alert");

    view.rerender(<DetachedStoveBar stove={stove} activeAlertStoveId={stove.id} />);
    expect(document.querySelector(".stove-burner-wrap")).toHaveClass("stove-burner-wrap--alert");
  });
});
