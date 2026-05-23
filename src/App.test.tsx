import { render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";

function healthResponse() {
  return {
    ok: true,
    status: 200,
    statusText: "OK",
    json: async () => ({
      status: "ok",
      startedAt: "2026-05-24T00:00:00Z",
      uptimeSeconds: 120,
      version: "0.1.0",
      windowCollector: { status: "running", errorCount: 0 },
      inputCollector: { status: "running", errorCount: 0 },
      screenshotCollector: { status: "running", errorCount: 0 },
      dbStats: { windowEvents: 10, inputEvents: 0, textSegments: 0, screenshots: 5, blockerHits: 0 },
    }),
  };
}

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

  it("exposes collector data button as a keyboard-focusable control", () => {
    render(<App />);

    expect(
      screen.getByRole("button", { name: /collector data/i })
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
    expect(screen.getAllByText("5m").length).toBeGreaterThan(0);
  });

  it("renders CollectorMonitor when connected to health API", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(healthResponse())
    );

    render(<App />);

    expect(await screen.findByText("Collector Monitor")).toBeInTheDocument();
    expect(screen.getByText("Subsystems")).toBeInTheDocument();
    expect(screen.getByText("Window Collector")).toBeInTheDocument();
    expect(screen.getByText("Database")).toBeInTheDocument();
  });
});
