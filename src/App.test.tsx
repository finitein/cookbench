import "@testing-library/jest-dom/vitest";
import { render, screen } from "@testing-library/react";
import App from "./App";

describe("App", () => {
  it("renders the Cookbench shell and a stove", () => {
    render(<App />);

    expect(screen.getByText("Cookbench")).toBeInTheDocument();
    expect(screen.getByTestId("stove")).toBeInTheDocument();
  });
});
