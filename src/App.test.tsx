import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";

describe("App", () => {
  beforeEach(() => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockRejectedValue(new Error("collector offline"))
    );
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("shows total active duration in the descriptive statistics cards", () => {
    render(<App />);

    expect(screen.getByText("Total")).toBeInTheDocument();
    expect(screen.getByText("87m")).toBeInTheDocument();
  });

  it("exposes collector refresh as a keyboard-focusable button", () => {
    render(<App />);

    expect(
      screen.getByRole("button", { name: /refresh collector/i })
    ).toBeInTheDocument();
  });

  it("renders collector rows returned by the API", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        statusText: "OK",
        json: async () => ({
          events: [
            {
              id: "collector-1",
              app: "Arc",
              title: "Planner",
              startedAt: "2026-05-23T10:00:00Z",
              endedAt: "2026-05-23T10:05:00Z",
              durationSeconds: 300
            }
          ]
        })
      })
    );

    render(<App />);

    expect(await screen.findByText("Planner")).toBeInTheDocument();
    expect(screen.getByText("Connected")).toBeInTheDocument();
    expect(screen.getAllByText("5m").length).toBeGreaterThan(0);
  });
});
