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
            durationSeconds: 600
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
        durationSeconds: 600
      }
    ]);
    expect(fetcher).toHaveBeenCalledWith("/api/time-events");
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

