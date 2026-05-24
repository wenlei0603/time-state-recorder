import { fireEvent, render, screen } from "@testing-library/react";
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
      dbStats: {
        windowEvents: 10,
        lifecycleEvents: 0,
        inputEvents: 0,
        textSegments: 0,
        screenshots: 5,
        blockerHits: 0,
      },
    }),
  };
}

function jsonResponse(body: unknown) {
  return {
    ok: true,
    status: 200,
    statusText: "OK",
    json: async () => body,
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

  it("renders dashboard as the default view", () => {
    render(<App />);

    expect(
      screen.getByRole("heading", { name: /dashboard/i })
    ).toBeInTheDocument();
    expect(screen.getByText(/active time/i)).toBeInTheDocument();
  });

  it("exposes Toggl-style source and privacy toggles", () => {
    render(<App />);

    expect(screen.getByRole("button", { name: /^sample$/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /^live$/i })).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /^redacted$/i })
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /^raw$/i })).toBeInTheDocument();
  });

  it("opens the timeline view from the tab bar", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: /timeline/i }));

    expect(
      screen.getByRole("heading", { name: /timeline/i })
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

  it("keeps input segment text hidden while privacy mode is redacted", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: /input activity/i }));
    fireEvent.click(screen.getAllByText("main.rs")[0]);

    expect(screen.getByText(/Raw text hidden in redacted mode/i)).toBeInTheDocument();
    expect(screen.queryByText(/fn main/i)).not.toBeInTheDocument();
  });

  it("does not request raw input segments while privacy mode is redacted", async () => {
    const fetcher = vi.fn(liveDataResponse);
    vi.stubGlobal("fetch", fetcher);

    render(<App />);

    expect(await screen.findByText("Live window")).toBeInTheDocument();

    expect(
      fetcher.mock.calls.some(([input]) => String(input).startsWith("/api/text-segments"))
    ).toBe(false);
  });

  it("loads live input segments after switching to raw privacy mode", async () => {
    vi.stubGlobal("fetch", vi.fn(liveDataResponse));

    render(<App />);

    expect(await screen.findByText("Live window")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /^raw$/i }));
    fireEvent.click(screen.getByRole("button", { name: /input activity/i }));

    expect(await screen.findByText("Live Input Window")).toBeInTheDocument();
    expect(screen.queryByText(/Showing sample data/i)).not.toBeInTheDocument();
  });

  it("hides screenshot evidence when screenshots layer is disabled", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: /^screenshots$/i }));
    fireEvent.click(screen.getByRole("button", { name: /daily tracking/i }));

    expect(screen.getByText(/Screenshots layer is hidden/i)).toBeInTheDocument();
    expect(screen.queryByAltText(/Screenshot at/i)).not.toBeInTheDocument();
  });
});

async function liveDataResponse(input: string) {
  if (input === "/api/time-events") {
    return jsonResponse({
      events: [
        {
          id: "live-window",
          app: "Code",
          title: "Live window",
          startedAt: "2026-05-24T10:00:00Z",
          endedAt: "2026-05-24T10:05:00Z",
          durationSeconds: 300
        }
      ]
    });
  }
  if (input.startsWith("/api/input-summary")) {
    return jsonResponse({
      date: "2026-05-24",
      totalEvents: 20,
      keydownCount: 10,
      keyupCount: 10,
      segmentCount: 1,
      totalChars: 8,
      lastActivity: "2026-05-24T10:05:00Z",
      topApps: [{ processName: "LiveApp", charCount: 8 }]
    });
  }
  if (input.startsWith("/api/text-segments")) {
    return jsonResponse({
      segments: [
        {
          id: "live-segment",
          startedAt: "2026-05-24T10:00:00Z",
          endedAt: "2026-05-24T10:01:00Z",
          textContent: "live text",
          keyCount: 8,
          backspaceCount: 1,
          deleteCount: 0,
          foregroundHwnd: 1,
          foregroundPid: 2,
          processName: "LiveApp",
          windowTitle: "Live Input Window"
        }
      ]
    });
  }
  if (input.startsWith("/api/screenshot-summary")) {
    return jsonResponse({
      date: "2026-05-24",
      totalScreenshots: 1,
      hoursCovered: 1,
      topApps: [{ processName: "Code", count: 1 }]
    });
  }
  if (input.startsWith("/api/screenshots")) {
    return jsonResponse({
      screenshots: [
        {
          id: 1,
          capturedAt: "2026-05-24T10:00:00Z",
          filePath: "2026-05-24/10-00.jpg",
          width: 640,
          height: 360,
          processName: "Code",
          windowTitle: "Live screenshot",
          captureStatus: "ok"
        }
      ]
    });
  }
  if (input === "/api/health") {
    return healthResponse();
  }
  throw new Error(`Unexpected request: ${input}`);
}
