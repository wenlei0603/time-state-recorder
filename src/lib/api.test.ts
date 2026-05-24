import { describe, expect, it, vi } from "vitest";
import { fetchTimeEvents } from "./api";

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
