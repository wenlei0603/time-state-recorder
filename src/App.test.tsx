import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "./App";

describe("App", () => {
  it("shows total active duration in the descriptive statistics cards", () => {
    render(<App />);

    expect(screen.getByText("Total")).toBeInTheDocument();
    expect(screen.getByText("87m")).toBeInTheDocument();
  });

  it("exposes CSV import as a keyboard-focusable button", () => {
    render(<App />);

    expect(
      screen.getByRole("button", { name: /import csv/i })
    ).toBeInTheDocument();
  });
});

