import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import type { StoveWire } from "../types/stove";
import { StoveBurner } from "./StoveBurner";

const cookedStove: StoveWire = {
  id: "local:codex:session-42",
  harness: { id: "codex", label: "Codex" },
  host: { id: "local", kind: "local" },
  projectRoot: "/workspace/demo",
  projectLabel: "demo",
  projectRootDisplay: "~/workspace/demo",
  taskTitle: null,
  currentAction: null,
  nextAction: null,
  elapsedMs: null,
  state: "cooked",
  progress: null,
  locatorCapability: "available",
  retainedCompletion: true,
};

describe("StoveBurner completion presentation", () => {
  it("marks only a new Cooked transition as finishing", () => {
    const { rerender } = render(
      <StoveBurner stove={cookedStove} previousState="cooking" />,
    );

    expect(screen.getByTestId("stove")).toHaveAttribute("data-completion", "finishing");

    rerender(<StoveBurner stove={cookedStove} previousState="cooked" />);

    expect(screen.getByTestId("stove")).toHaveAttribute("data-completion", "settled");
  });

  it("renders Cooked as a settled complete ring, not a continuously rotating arc", () => {
    render(<StoveBurner stove={cookedStove} />);

    const burner = screen.getByTestId("stove");
    const ring = screen.getByTestId("progress-ring");
    expect(burner).toHaveAttribute("data-state", "cooked");
    expect(burner).toHaveAttribute("data-completion", "settled");
    expect(burner).toHaveAttribute("data-motion", "settled");
    expect(ring).toHaveAttribute("data-ring-mode", "complete");
  });

  it("settles a new completion immediately when reduced motion is enabled", () => {
    render(
      <StoveBurner
        stove={cookedStove}
        previousState="cooking"
        motionPreferences={{ reducedMotion: true, soundEnabled: false }}
      />,
    );

    const burner = screen.getByTestId("stove");
    expect(burner).toHaveAttribute("data-completion", "settled");
    expect(burner).toHaveAttribute("data-motion", "settled");
  });

  it("announces hover and keyboard detail visibility to its owning bar", () => {
    const visibility: boolean[] = [];
    render(
      <StoveBurner
        stove={cookedStove}
        onTooltipVisibilityChange={(visible) => visibility.push(visible)}
      />,
    );

    const burner = screen.getByTestId("stove");
    fireEvent.pointerEnter(burner);
    fireEvent.focus(burner);
    fireEvent.blur(burner);

    expect(visibility).toEqual([true, true, false]);
  });
});
