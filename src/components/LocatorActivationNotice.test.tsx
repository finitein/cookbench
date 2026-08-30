import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { LocatorActivationNotice } from "./LocatorActivationNotice";

describe("LocatorActivationNotice", () => {
  it("stays silent after a successful return", () => {
    const { container } = render(
      <LocatorActivationNotice
        result={{ target: "exactPane", status: "focused", resumeSessionId: null }}
      />,
    );

    expect(container).toBeEmptyDOMElement();
  });

  it("renders a visible resume result", () => {
    render(
      <LocatorActivationNotice
        result={{
          target: "resumeInstructions",
          status: "visibleFallback",
          resumeSessionId: "opaque-session-42",
        }}
      />,
    );

    expect(screen.getByRole("status")).toHaveTextContent("Resume session: opaque-session-42");
  });

  it("does not imply an unavailable locator was focused", () => {
    render(
      <LocatorActivationNotice
        result={{ target: "unavailable", status: "unavailable", resumeSessionId: null }}
      />,
    );

    expect(screen.getByRole("status")).toHaveTextContent("unavailable");
  });
});
