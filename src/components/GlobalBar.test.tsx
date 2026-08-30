import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import type { StoveWire } from "../types/stove";
import { globalBarFixture, makeStove } from "../stories/GlobalBar.fixture";
import { GlobalBar } from "./GlobalBar";

describe("GlobalBar", () => {
  it("keeps the empty state as a compact branded bar rather than a blank window", () => {
    render(<GlobalBar stoves={[]} />);

    const bar = screen.getByRole("region", { name: "Cookbench global bar with 0 stoves" });
    expect(bar).toHaveClass("global-bar--empty");
    expect(screen.getByRole("img", { name: "Cookbench" })).toHaveAttribute(
      "src",
      expect.stringContaining("cookbench-mark"),
    );
  });

  it("renders every stove at the same time and preserves the session count", () => {
    const stoves = globalBarFixture(10);
    render(<GlobalBar stoves={stoves} />);

    expect(screen.getAllByTestId("stove")).toHaveLength(10);
    expect(screen.getByRole("list", { name: "Stoves" })).toHaveAttribute("aria-label", "Stoves");
    expect(screen.getByRole("region", { name: "Cookbench global bar with 10 stoves" }))
      .toHaveStyle("--stove-grid-width: 688px");
  });

  it("makes Codex, Claude Code, and Pi visible on their burners", () => {
    render(<GlobalBar stoves={[makeStove(0), makeStove(1), makeStove(2)]} />);

    expect(screen.getByLabelText("Codex")).toBeVisible();
    expect(screen.getByLabelText("Claude Code")).toBeVisible();
    expect(screen.getByLabelText("Pi")).toBeVisible();
  });

  it("uses a determinate arc only for Cooking with structured progress", () => {
    render(<GlobalBar stoves={[makeStove(0, { state: "cooking" }), makeStove(3, { state: "cooking", progress: null })]} />);

    const rings = screen.getAllByTestId("progress-ring");
    expect(rings[0]).toHaveAttribute("data-ring-mode", "determinate");
    expect(rings[0]).toHaveAttribute("data-progress", "40");
    expect(rings[1]).toHaveAttribute("data-ring-mode", "indeterminate");
    expect(rings[1]).not.toHaveAttribute("data-progress");
    expect(rings[1]).not.toHaveTextContent("%");
  });

  it.each(["needsHuman", "cooked", "failed", "disconnected"] as const)("renders %s as a complete accessible ring", (state) => {
    render(<GlobalBar stoves={[makeStove(0, { state, progress: { completed: 1, total: 8, provenance: "structuredSession" } })]} />);

    const ring = screen.getByTestId("progress-ring");
    expect(ring).toHaveAttribute("data-ring-mode", "complete");
    expect(ring).not.toHaveAttribute("data-progress");
    expect(ring).toHaveAccessibleName();
  });

  it.each([1, 6, 10, 20, 30])("keeps all %i fixture burners in the layout", (count) => {
    render(<GlobalBar stoves={globalBarFixture(count)} />);

    expect(screen.getAllByTestId("stove")).toHaveLength(count);
    expect(document.querySelectorAll(".global-bar__item")).toHaveLength(count);
  });

  it("exposes local and remote host details in a focusable burner tooltip", () => {
    const remote = makeStove(0, { host: { kind: "ssh", id: "build-host" } });
    render(<GlobalBar stoves={[remote]} />);

    expect(screen.getByLabelText("Remote host: build-host")).toBeVisible();
    expect(screen.getByRole("tooltip")).toHaveTextContent("Remote: build-host");
    expect(screen.getByRole("tooltip")).toHaveTextContent("Project 1");
  });

  it("activates the original stove without controlling the harness", () => {
    let activated: StoveWire | undefined;
    const stove = makeStove(0);
    render(<GlobalBar stoves={[stove]} onActivateStove={(value) => { activated = value; }} />);

    screen.getByTestId("stove").click();
    expect(activated).toBe(stove);
  });

  it("offers independent detach, clear, and settings commands", () => {
    const stove = makeStove(0, { state: "cooked", retainedCompletion: true });
    const actions: string[] = [];
    render(
      <GlobalBar
        stoves={[stove]}
        onDetachStove={() => actions.push("detach")}
        onClearStove={() => actions.push("clear")}
        onOpenSettings={() => actions.push("settings")}
      />,
    );

    screen.getByRole("button", { name: "Detach Codex Stove" }).click();
    screen.getByRole("button", { name: "Clear Codex Stove" }).click();
    screen.getByRole("button", { name: "Open Cookbench settings" }).click();
    expect(actions).toEqual(["detach", "clear", "settings"]);
  });

  it("plays the completion presentation only for a live transition into Cooked", () => {
    const stove = makeStove(0, { state: "cooking", progress: null });
    const view = render(<GlobalBar stoves={[stove]} />);

    expect(screen.getByTestId("stove")).toHaveAttribute("data-completion", "none");
    view.rerender(<GlobalBar stoves={[{ ...stove, state: "cooked", retainedCompletion: true }]} />);
    expect(screen.getByTestId("stove")).toHaveAttribute("data-completion", "finishing");
  });
});
