import { act, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { LocatorActivationNotice } from "./LocatorActivationNotice";

describe("LocatorActivationNotice", () => {
  afterEach(() => vi.useRealTimers());

  it("stays silent after a successful return", () => {
    const { container } = render(
      <LocatorActivationNotice
        result={{ target: "exactPane", status: "focused", resumeSessionId: null }}
      />,
    );

    expect(container).toBeEmptyDOMElement();
  });

  it("explains the manual return path without pretending the original surface opened", () => {
    render(
      <LocatorActivationNotice
        result={{
          target: "resumeInstructions",
          status: "visibleFallback",
          resumeSessionId: "opaque-session-42",
        }}
      />,
    );

    expect(screen.getByRole("status")).toHaveTextContent("could not open the original session");
    expect(screen.getByRole("status")).toHaveTextContent("use this session ID in your original tool");
    expect(screen.getByRole("status")).toHaveTextContent("opaque-session-42");
  });

  it("describes an accepted Codex deep link as an exact task request", () => {
    render(
      <LocatorActivationNotice
        result={{ target: "exactThread", status: "visibleFallback", resumeSessionId: null }}
      />,
    );

    expect(screen.getByRole("status")).toHaveTextContent("Opening the matching Codex task");
    expect(screen.getByRole("status")).not.toHaveTextContent("could not open");
  });

  it("does not imply an unavailable locator was focused", () => {
    render(
      <LocatorActivationNotice
        result={{ target: "unavailable", status: "unavailable", resumeSessionId: null }}
      />,
    );

    expect(screen.getByRole("status")).toHaveTextContent("could not open the original session");
    expect(screen.getByRole("status")).toHaveTextContent("kept the Stove visible here");
  });

  it("removes a visible notice after twenty seconds without leaving a Bar row", () => {
    vi.useFakeTimers();
    const { container } = render(
      <LocatorActivationNotice
        result={{ target: "unavailable", status: "unavailable", resumeSessionId: null }}
      />,
    );

    expect(screen.getByRole("status")).toBeVisible();
    act(() => vi.advanceTimersByTime(19_999));
    expect(screen.getByRole("status")).toBeVisible();
    act(() => vi.advanceTimersByTime(1));
    expect(container).toBeEmptyDOMElement();
  });
});
