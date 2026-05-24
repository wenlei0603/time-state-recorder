import { describe, expect, it, vi } from "vitest";
import { fetchCollectorHealth } from "./health";

describe("fetchCollectorHealth", () => {
  it("loads collector health from /api/health", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        status: "ok",
        startedAt: "2026-05-24T10:00:00Z",
        uptimeSeconds: 3600,
        version: "0.1.0",
        windowCollector: {
          status: "running",
          lastEventAt: "2026-05-24T10:59:00Z",
          errorCount: 0,
        },
        inputCollector: {
          status: "running",
          lastEventAt: "2026-05-24T10:58:30Z",
          errorCount: 0,
        },
        screenshotCollector: {
          status: "error",
          lastEventAt: "2026-05-24T09:12:00Z",
          errorCount: 3,
          lastError: "capture_thumbnail returned None",
          mode: "thumbnail",
          lastCaptureStatus: "skipped",
          lastSkipReason: "privacy_blocker",
        },
        dbStats: {
          windowEvents: 452,
          inputEvents: 3421,
          textSegments: 87,
          screenshots: 128,
          blockerHits: 3,
          lifecycleEvents: 4,
        },
      }),
    });

    const health = await fetchCollectorHealth(fetcher);
    expect(health.status).toBe("ok");
    expect(health.uptimeSeconds).toBe(3600);
    expect(health.version).toBe("0.1.0");
    expect(health.windowCollector.status).toBe("running");
    expect(health.inputCollector.lastEventAt).toBe("2026-05-24T10:58:30Z");
    expect(health.screenshotCollector.status).toBe("error");
    expect(health.screenshotCollector.errorCount).toBe(3);
    expect(health.screenshotCollector.lastError).toBe("capture_thumbnail returned None");
    expect(health.screenshotCollector.mode).toBe("thumbnail");
    expect(health.screenshotCollector.lastCaptureStatus).toBe("skipped");
    expect(health.screenshotCollector.lastSkipReason).toBe("privacy_blocker");
    expect(health.dbStats.windowEvents).toBe(452);
    expect(health.dbStats.blockerHits).toBe(3);
    expect(health.dbStats.lifecycleEvents).toBe(4);
    expect(fetcher).toHaveBeenCalledWith("/api/health");
  });

  it("reports API failures", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: false,
      status: 503,
      statusText: "Service Unavailable",
    });

    await expect(fetchCollectorHealth(fetcher)).rejects.toThrow(
      "Collector API failed: 503 Service Unavailable",
    );
  });

  it("handles missing optional fields", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        status: "ok",
        startedAt: "2026-05-24T10:00:00Z",
        uptimeSeconds: 0,
        version: "0.1.0",
        windowCollector: {
          status: "not_started",
          errorCount: 0,
        },
        inputCollector: {
          status: "not_started",
          errorCount: 0,
        },
        screenshotCollector: {
          status: "not_started",
          errorCount: 0,
        },
        dbStats: {
          windowEvents: 0,
          inputEvents: 0,
          textSegments: 0,
          screenshots: 0,
          blockerHits: 0,
          lifecycleEvents: 0,
        },
      }),
    });

    const health = await fetchCollectorHealth(fetcher);
    expect(health.windowCollector.lastEventAt).toBeUndefined();
    expect(health.windowCollector.lastError).toBeUndefined();
  });
});
