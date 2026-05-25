import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
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

  it("renders Today Flow Board as the default view", () => {
    render(<App />);

    expect(
      screen.getByRole("heading", { name: /today flow board/i })
    ).toBeInTheDocument();
    expect(screen.getByText(/time flow/i)).toBeInTheDocument();
    expect(screen.getByText(/evidence drawer/i)).toBeInTheDocument();
  });

  it("opens the dashboard view from the tab bar", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: /dashboard/i }));

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
    fireEvent.click(screen.getByRole("button", { name: /dashboard/i }));

    expect(await screen.findByText("Planner")).toBeInTheDocument();
    expect(screen.getAllByText("5m").length).toBeGreaterThan(0);
  });

  it("renders CollectorMonitor when connected to health API", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(healthResponse())
    );

    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /dashboard/i }));

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

    expect(await screen.findAllByText("Hidden in redacted mode")).not.toHaveLength(0);

    expect(
      fetcher.mock.calls.some(([input]) => String(input).startsWith("/api/text-segments"))
    ).toBe(false);
  });

  it("does not apply delayed raw input rows after switching back to redacted", async () => {
    const segmentsResponse = createDeferred<ReturnType<typeof jsonResponse>>();
    const fetcher = vi.fn((input: string) => {
      if (input.startsWith("/api/text-segments")) {
        return segmentsResponse.promise;
      }
      return liveDataResponse(input);
    });
    vi.stubGlobal("fetch", fetcher);

    render(<App />);

    expect(await screen.findAllByText("Hidden in redacted mode")).not.toHaveLength(0);
    fireEvent.click(screen.getByRole("button", { name: /input activity/i }));
    fireEvent.click(screen.getByRole("button", { name: /^raw$/i }));
    await waitFor(() =>
      expect(
        fetcher.mock.calls.some(([input]) =>
          String(input).startsWith("/api/text-segments")
        )
      ).toBe(true)
    );

    fireEvent.click(screen.getByRole("button", { name: /^redacted$/i }));
    await act(async () => {
      segmentsResponse.resolve(
        jsonResponse({
          segments: [
            {
              id: "delayed-raw-segment",
              startedAt: "2026-05-24T10:02:00Z",
              endedAt: "2026-05-24T10:03:00Z",
              textContent: "delayed raw text",
              keyCount: 12,
              backspaceCount: 0,
              deleteCount: 0,
              foregroundHwnd: 1,
              foregroundPid: 2,
              processName: "DelayedApp",
              windowTitle: "Delayed Raw Window"
            }
          ]
        })
      );
      await Promise.resolve();
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(screen.queryByText("Delayed Raw Window")).not.toBeInTheDocument();
    expect(screen.queryByText("delayed raw text")).not.toBeInTheDocument();
  });

  it("keeps sample data when a delayed live refresh resolves after switching to sample", async () => {
    const timeEventsResponse = createDeferred<ReturnType<typeof jsonResponse>>();
    const fetcher = vi.fn((input: string) => {
      if (input === "/api/time-events") {
        return timeEventsResponse.promise;
      }
      return liveDataResponse(input);
    });
    vi.stubGlobal("fetch", fetcher);

    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: /^raw$/i }));
    fireEvent.click(screen.getByRole("button", { name: /^sample$/i }));
    await act(async () => {
      timeEventsResponse.resolve(
        jsonResponse({
          events: [
            {
              id: "stale-live-event",
              app: "StaleLiveApp",
              title: "Stale live title",
              startedAt: "2026-05-24T10:00:00Z",
              endedAt: "2026-05-24T10:05:00Z",
              durationSeconds: 300
            }
          ]
        })
      );
      await Promise.resolve();
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(screen.getByText("Sample workspace")).toBeInTheDocument();
    expect(screen.queryByText("Live collector")).not.toBeInTheDocument();
    expect(screen.queryByText("StaleLiveApp")).not.toBeInTheDocument();
    expect(screen.queryByText("Stale live title")).not.toBeInTheDocument();
  });

  it("keeps raw live titles hidden on the default redacted flow board", async () => {
    vi.stubGlobal("fetch", vi.fn(liveDataResponse));

    render(<App />);

    expect(await screen.findAllByText("Hidden in redacted mode")).not.toHaveLength(0);
    expect(screen.queryByText("Sensitive client roadmap")).not.toBeInTheDocument();
  });

  it("shows raw live evidence titles on the flow board after switching privacy mode", async () => {
    vi.stubGlobal("fetch", vi.fn(liveDataResponse));

    render(<App />);

    expect(await screen.findAllByText("Hidden in redacted mode")).not.toHaveLength(0);
    fireEvent.click(screen.getByRole("button", { name: /^raw$/i }));

    expect(await screen.findAllByText("Sensitive client roadmap")).not.toHaveLength(0);
  });

  it("does not request raw row endpoints when raw is selected on Today", async () => {
    const fetcher = vi.fn(liveDataResponse);
    vi.stubGlobal("fetch", fetcher);

    render(<App />);

    expect(await screen.findAllByText("Hidden in redacted mode")).not.toHaveLength(0);
    fireEvent.click(screen.getByRole("button", { name: /^raw$/i }));

    expect(await screen.findAllByText("Sensitive client roadmap")).not.toHaveLength(0);
    expect(
      fetcher.mock.calls.some(([input]) =>
        String(input).startsWith("/api/text-segments")
      )
    ).toBe(false);
    expect(
      fetcher.mock.calls.some(([input]) =>
        String(input).startsWith("/api/screenshots?")
      )
    ).toBe(false);
  });

  it("keeps newer raw input rows when an older non-row refresh resolves later", async () => {
    const olderSummaryResponse = createDeferred<ReturnType<typeof jsonResponse>>();
    let inputSummaryCalls = 0;
    const fetcher = vi.fn((input: string) => {
      if (input.startsWith("/api/input-summary")) {
        inputSummaryCalls += 1;
        if (inputSummaryCalls === 2) {
          return olderSummaryResponse.promise;
        }
      }
      return liveDataResponse(input);
    });
    vi.stubGlobal("fetch", fetcher);

    render(<App />);

    expect(await screen.findAllByText("Hidden in redacted mode")).not.toHaveLength(0);
    fireEvent.click(screen.getByRole("button", { name: /^raw$/i }));
    await waitFor(() => expect(inputSummaryCalls).toBe(2));
    fireEvent.click(screen.getByRole("button", { name: /input activity/i }));

    expect(await screen.findByText("Live Input Window")).toBeInTheDocument();
    await act(async () => {
      olderSummaryResponse.resolve(
        jsonResponse({
          date: "2026-05-24",
          totalEvents: 20,
          keydownCount: 10,
          keyupCount: 10,
          segmentCount: 1,
          totalChars: 8,
          lastActivity: "2026-05-24T10:05:00Z",
          topApps: [{ processName: "LiveApp", charCount: 8 }]
        })
      );
      await Promise.resolve();
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(screen.getByText("Live Input Window")).toBeInTheDocument();
  });

  it("updates the evidence drawer when a flow bucket is selected", async () => {
    vi.stubGlobal("fetch", vi.fn(twoBucketLiveDataResponse));

    render(<App />);

    const codeBucket = await screen.findByRole("button", { name: /Code, 5m/i });
    const browserBucket = await screen.findByRole("button", { name: /Browser, 4m/i });
    const drawer = screen.getByRole("region", { name: /evidence drawer/i });

    expect(codeBucket).toHaveAttribute("aria-pressed", "true");
    expect(within(drawer).getByText("Code")).toBeInTheDocument();
    expect(within(drawer).queryByText("Browser")).not.toBeInTheDocument();

    fireEvent.click(browserBucket);

    expect(browserBucket).toHaveAttribute("aria-pressed", "true");
    expect(within(drawer).getByText("Browser")).toBeInTheDocument();
    expect(within(drawer).queryByText("Code")).not.toBeInTheDocument();
  });

  it("queries live collector data for the selected date", async () => {
    const fetcher = vi.fn(liveDataResponse);
    vi.stubGlobal("fetch", fetcher);

    render(<App />);

    const queryInput = screen.getByLabelText(/query date/i);
    fireEvent.change(queryInput, { target: { value: "2026-05-24" } });
    fireEvent.click(screen.getByRole("button", { name: /query/i }));

    expect(await screen.findAllByText("Hidden in redacted mode")).not.toHaveLength(0);
    expect(
      fetcher.mock.calls.some(
        ([input]) => input === "/api/screenshot-summary?date=2026-05-24"
      )
    ).toBe(true);
    expect(
      fetcher.mock.calls.some(
        ([input]) => input === "/api/input-summary?date=2026-05-24"
      )
    ).toBe(true);
  });

  it("loads live input segments after switching to raw privacy mode", async () => {
    vi.stubGlobal("fetch", vi.fn(liveDataResponse));

    render(<App />);

    expect(await screen.findAllByText("Hidden in redacted mode")).not.toHaveLength(0);
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
          title: "Sensitive client roadmap",
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
      topApps: [{ processName: "Code", count: 1 }],
      skippedReasons: [{ reason: "privacy_blocked", count: 2 }]
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

async function twoBucketLiveDataResponse(input: string) {
  if (input === "/api/time-events") {
    return jsonResponse({
      events: [
        {
          id: "live-code",
          app: "Code",
          title: "Code editor",
          startedAt: "2026-05-24T10:00:00Z",
          endedAt: "2026-05-24T10:05:00Z",
          durationSeconds: 300
        },
        {
          id: "live-browser",
          app: "Browser",
          title: "Research notes",
          startedAt: "2026-05-24T10:05:00Z",
          endedAt: "2026-05-24T10:09:00Z",
          durationSeconds: 240
        }
      ]
    });
  }
  return liveDataResponse(input);
}

function createDeferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((promiseResolve, promiseReject) => {
    resolve = promiseResolve;
    reject = promiseReject;
  });

  return { promise, resolve, reject };
}
