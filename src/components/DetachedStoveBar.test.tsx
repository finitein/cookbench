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
};

describe("DetachedStoveBar", () => {
  it("uses the same accessible burner for a single Stove", () => {
    render(<DetachedStoveBar stove={stove} />);

    expect(screen.getByRole("region", { name: "Detached Stove bar for Codex" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Codex:/ })).toBeInTheDocument();
    expect(document.querySelector(".detached-stove-bar__identity")).toHaveTextContent("Codex");
    expect(screen.getByTestId("stove")).toHaveAttribute("data-state", "cooking");
  });

  it("forwards activation and exposes an explicit manual clear action", () => {
    const onActivate = vi.fn();
    const onClear = vi.fn();
    render(<DetachedStoveBar stove={stove} onActivate={onActivate} onClear={onClear} />);

    fireEvent.click(screen.getByRole("button", { name: /Codex:/ }));
    fireEvent.click(screen.getByRole("button", { name: "Clear Codex Stove" }));

    expect(onActivate).toHaveBeenCalledWith(stove);
    expect(onClear).toHaveBeenCalledWith(stove);
  });

  it("does not render a clear control when manual clearing is unavailable", () => {
    render(<DetachedStoveBar stove={{ ...stove, state: "cooked", retainedCompletion: true }} />);

    expect(screen.queryByRole("button", { name: /Clear .* Stove/ })).not.toBeInTheDocument();
    expect(screen.getByTestId("stove")).toHaveAttribute("data-state", "cooked");
  });
});
