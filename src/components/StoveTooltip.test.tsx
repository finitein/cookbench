import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { makeStove } from "../stories/GlobalBar.fixture";
import { StoveTooltip } from "./StoveTooltip";

describe("StoveTooltip", () => {
  it("renders bounded presentation summaries and reports unknown elapsed time honestly", () => {
    render(
      <StoveTooltip
        id="tooltip"
        stove={makeStove(0, {
          taskTitle: "Verify the release boundary",
          currentAction: "Running a tool",
          nextAction: "Waiting for its result",
          elapsedMs: null,
        })}
      />,
    );

    const tooltip = screen.getByRole("tooltip");
    expect(tooltip).toHaveTextContent("Verify the release boundary");
    expect(tooltip).toHaveTextContent("Running a tool");
    expect(tooltip).toHaveTextContent("Waiting for its result");
    expect(tooltip).toHaveTextContent("Not reported");
    expect(tooltip).not.toHaveTextContent("0s");
  });
});
