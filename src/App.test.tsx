import "@testing-library/jest-dom/vitest";
import { render, screen } from "@testing-library/react";
import App from "./App";

describe("App", () => {
  it("renders the global bar without inventing an empty stove", () => {
    render(<App />);

    expect(screen.getByLabelText("Cookbench global bar with 0 stoves")).toBeInTheDocument();
    expect(screen.queryAllByRole("listitem")).toHaveLength(0);
  });
});
