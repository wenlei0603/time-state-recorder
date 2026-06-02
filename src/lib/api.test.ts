import { describe, expect, it, vi } from "vitest";
import { fetchActivityBuckets, fetchTimeEvents } from "./api";

describe("fetchTimeEvents", () => {
  it("loads time events from the collector REST API", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        events: [
          {
            id: "raw-1",
            app: "Code.exe",
            title: "main.rs",
            startedAt: "2026-05-23T09:00:00.000Z",
            endedAt: "2026-05-23T09:10:00.000Z",
            durationSeconds: 600,
            kind: "active_window",
            sessionId: "session-1"
          }
        ]
      })
    });

    await expect(fetchTimeEvents(fetcher)).resolves.toEqual([
      {
        id: "raw-1",
        app: "Code.exe",
        title: "main.rs",
        startedAt: "2026-05-23T09:00:00.000Z",
        endedAt: "2026-05-23T09:10:00.000Z",
        durationSeconds: 600,
        kind: "active_window",
        sessionId: "session-1"
      }
    ]);
    expect(fetcher).toHaveBeenCalledWith("/api/time-events");
  });

  it("preserves lifecycle metadata from time events", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        events: [
          {
            id: "lifecycle-10",
            app: "System",
            title: "Locked",
            kind: "lifecycle",
            status: "windows_lock",
            sessionId: "session-1",
            startedAt: "2026-05-23T09:05:00.000Z",
            endedAt: "2026-05-23T09:20:00.000Z",
            durationSeconds: 900
          }
        ]
      })
    });

    await expect(fetchTimeEvents(fetcher)).resolves.toEqual([
      {
        id: "lifecycle-10",
        app: "System",
        title: "Locked",
        kind: "lifecycle",
        status: "windows_lock",
        sessionId: "session-1",
        startedAt: "2026-05-23T09:05:00.000Z",
        endedAt: "2026-05-23T09:20:00.000Z",
        durationSeconds: 900
      }
    ]);
  });

  it("reports collector API failures", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: false,
      status: 503,
      statusText: "Service Unavailable"
    });

    await expect(fetchTimeEvents(fetcher)).rejects.toThrow(
      "Collector API failed: 503 Service Unavailable"
    );
  });
});

describe("fetchActivityBuckets", () => {
  it("fetches activity buckets for a date", async () => {
    const fetcher = vi.fn().mockResolvedValue(
      jsonResponse({
        date: "2026-05-24",
        bucketSeconds: 180,
        buckets: [
          {
            id: "bucket-1",
            startAt: "2026-05-24T10:00:00Z",
            endAt: "2026-05-24T10:03:00Z",
            bucketSeconds: 180,
            dominantApp: "Code",
            dominantTitle: "main.rs",
            normalizedTitle: "main.rs",
            dominantDurationSeconds: 150,
            switchCount: 1,
            projectId: null,
            projectName: null,
            activityCategory: "coding",
            attentionState: "deep_focus",
            confidence: 0.83,
            evidence: [],
            visualSummaryId: null
          }
        ]
      })
    );

    const result = await fetchActivityBuckets("2026-05-24", 180, fetcher);

    expect(fetcher).toHaveBeenCalledWith(
      "/api/activity-buckets?date=2026-05-24&bucketSeconds=180"
    );
    expect(result.date).toBe("2026-05-24");
    expect(result.bucketSeconds).toBe(180);
    expect(result.buckets[0].dominantApp).toBe("Code");
  });

  it("rejects invalid activity bucket rows", async () => {
    const fetcher = vi.fn().mockResolvedValue(
      jsonResponse({
        date: "2026-05-24",
        bucketSeconds: 180,
        buckets: [{ id: "broken" }]
      })
    );

    await expect(fetchActivityBuckets("2026-05-24", 180, fetcher)).rejects.toThrow(
      /invalid activity bucket/i
    );
  });
});

function jsonResponse(body: unknown) {
  return {
    ok: true,
    status: 200,
    statusText: "OK",
    json: async () => body
  };
}
