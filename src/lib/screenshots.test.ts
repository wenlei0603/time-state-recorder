import { describe, expect, it, vi } from "vitest";
import { fetchScreenshotSummary } from "./screenshots";

describe("fetchScreenshotSummary", () => {
  it("loads skipped screenshot reasons from /api/screenshot-summary", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        date: "2026-05-24",
        totalScreenshots: 12,
        hoursCovered: 2,
        topApps: [],
        skippedReasons: [
          {
            reason: "privacy_blocker",
            count: 4,
          },
        ],
      }),
    });

    const summary = await fetchScreenshotSummary("2026-05-24", fetcher);

    expect(summary.skippedReasons).toEqual([
      {
        reason: "privacy_blocker",
        count: 4,
      },
    ]);
  });

  it("falls back to no skipped reasons when the API omits the field", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        date: "2026-05-24",
        totalScreenshots: 12,
        hoursCovered: 2,
        topApps: [],
      }),
    });

    const summary = await fetchScreenshotSummary("2026-05-24", fetcher);

    expect(summary.skippedReasons).toEqual([]);
  });

  it("rejects invalid skipped reason rows", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        date: "2026-05-24",
        totalScreenshots: 12,
        hoursCovered: 2,
        topApps: [],
        skippedReasons: [
          {
            reason: "privacy_blocker",
            count: "4",
          },
        ],
      }),
    });

    await expect(fetchScreenshotSummary("2026-05-24", fetcher)).rejects.toThrow(
      "skippedReasons row",
    );
  });
});
