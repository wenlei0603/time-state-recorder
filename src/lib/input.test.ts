import { describe, expect, it, vi } from "vitest";
import { fetchInputEvents, fetchInputSummary, fetchTextSegments } from "./input";

describe("fetchInputEvents", () => {
  it("loads input events from the collector REST API", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        events: [
          {
            id: 1,
            eventTs: "2026-05-23T09:00:01Z",
            eventType: "keydown",
            vkCode: 70,
            scanCode: 33,
            character: "f",
            segmentId: "seg-1",
            foregroundHwnd: 1111,
            foregroundPid: 100,
            processName: "Code",
            windowTitle: "main.rs",
          },
          {
            id: 2,
            eventTs: "2026-05-23T09:00:02Z",
            eventType: "keyup",
            vkCode: 70,
            scanCode: 33,
            character: null,
            segmentId: "seg-1",
            foregroundHwnd: 1111,
            foregroundPid: 100,
            processName: "Code",
            windowTitle: "main.rs",
          },
        ],
      }),
    });

    const events = await fetchInputEvents(fetcher);
    expect(events).toHaveLength(2);
    expect(events[0]).toEqual({
      id: 1,
      eventTs: "2026-05-23T09:00:01Z",
      eventType: "keydown",
      vkCode: 70,
      scanCode: 33,
      character: "f",
      segmentId: "seg-1",
      foregroundHwnd: 1111,
      foregroundPid: 100,
      processName: "Code",
      windowTitle: "main.rs",
    });
    expect(fetcher).toHaveBeenCalledWith("/api/input-events");
  });

  it("reports collector API failures", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: false,
      status: 503,
      statusText: "Service Unavailable",
    });

    await expect(fetchInputEvents(fetcher)).rejects.toThrow(
      "Collector API failed: 503 Service Unavailable",
    );
  });

  it("reports invalid response shape", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ notEvents: [] }),
    });

    await expect(fetchInputEvents(fetcher)).rejects.toThrow(
      "Collector API returned an invalid input-events response",
    );
  });
});

describe("fetchInputSummary", () => {
  it("loads input summary from the collector REST API", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        date: "2026-05-23",
        totalEvents: 4821,
        keydownCount: 2410,
        keyupCount: 2411,
        segmentCount: 87,
        totalChars: 3420,
        lastActivity: "2026-05-23T17:30:00Z",
        topApps: [
          { processName: "Code", charCount: 2100 },
          { processName: "WindowsTerminal", charCount: 890 },
        ],
      }),
    });

    const summary = await fetchInputSummary("2026-05-23", fetcher);
    expect(summary).toEqual({
      date: "2026-05-23",
      totalEvents: 4821,
      keydownCount: 2410,
      keyupCount: 2411,
      segmentCount: 87,
      totalChars: 3420,
      lastActivity: "2026-05-23T17:30:00Z",
      topApps: [
        { processName: "Code", charCount: 2100 },
        { processName: "WindowsTerminal", charCount: 890 },
      ],
    });
    expect(fetcher).toHaveBeenCalledWith(
      "/api/input-summary?date=2026-05-23",
    );
  });

  it("handles missing lastActivity", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        date: "2026-05-23",
        totalEvents: 0,
        keydownCount: 0,
        keyupCount: 0,
        segmentCount: 0,
        totalChars: 0,
        topApps: [],
      }),
    });

    const summary = await fetchInputSummary("2026-05-23", fetcher);
    expect(summary.lastActivity).toBeUndefined();
    expect(summary.topApps).toEqual([]);
  });
});

describe("fetchTextSegments", () => {
  it("loads text segments from the collector REST API", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        segments: [
          {
            id: "seg-1",
            startedAt: "2026-05-23T09:00:00Z",
            endedAt: "2026-05-23T09:00:05Z",
            textContent: "fn main() {\n",
            keyCount: 12,
            backspaceCount: 2,
            deleteCount: 0,
            foregroundHwnd: 1111,
            foregroundPid: 100,
            processName: "Code",
            windowTitle: "main.rs",
          },
        ],
      }),
    });

    const segments = await fetchTextSegments("2026-05-23", fetcher);
    expect(segments).toHaveLength(1);
    expect(segments[0].id).toBe("seg-1");
    expect(segments[0].textContent).toBe("fn main() {\n");
    expect(segments[0].keyCount).toBe(12);
    expect(segments[0].backspaceCount).toBe(2);
    expect(segments[0].deleteCount).toBe(0);
    expect(fetcher).toHaveBeenCalledWith(
      "/api/text-segments?date=2026-05-23",
    );
  });

  it("reports invalid response shape", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ notSegments: [] }),
    });

    await expect(fetchTextSegments("2026-05-23", fetcher)).rejects.toThrow(
      "Collector API returned an invalid text-segments response",
    );
  });
});
