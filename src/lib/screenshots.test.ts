import { describe, expect, it, vi } from "vitest";
import {
  analyzeScreenshot,
  fetchScreenshotSummary,
  fetchScreenshots,
  fetchVisualSummaries,
} from "./screenshots";

function expectedDateQuery(path: string, date: string): string {
  const offset = new Date(`${date}T00:00:00`).getTimezoneOffset();
  return `${path}?date=${date}&tzOffsetMinutes=${offset}`;
}

describe("screenshot date queries", () => {
  it("includes the browser timezone offset for local-day filtering", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        screenshots: [],
        summaries: [],
        date: "2026-05-24",
        totalScreenshots: 0,
        hoursCovered: 0,
        topApps: [],
        skippedReasons: [],
      }),
    });

    await fetchScreenshots("2026-05-24", fetcher);
    await fetchScreenshotSummary("2026-05-24", fetcher);
    await fetchVisualSummaries("2026-05-24", fetcher);

    expect(fetcher).toHaveBeenNthCalledWith(
      1,
      expectedDateQuery("/api/screenshots", "2026-05-24"),
    );
    expect(fetcher).toHaveBeenNthCalledWith(
      2,
      expectedDateQuery("/api/screenshot-summary", "2026-05-24"),
    );
    expect(fetcher).toHaveBeenNthCalledWith(
      3,
      expectedDateQuery("/api/visual-summaries", "2026-05-24"),
    );
  });
});

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

describe("fetchVisualSummaries", () => {
  it("loads visual summaries for a date", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        summaries: [
          {
            id: 1,
            screenshotId: 10,
            capturedAt: "2026-05-24T10:00:00Z",
            modelProvider: "local_stub",
            modelName: "metadata-v1",
            promptVersion: "visual-summary-v1",
            summaryText: "Metadata-only local summary",
            activityCategory: "coding",
            projectHints: ["Time State Recorder"],
            visibleApps: ["Code.exe"],
            visibleTextHints: ["main.rs"],
            riskFlags: [],
            confidence: 0.35,
            createdAt: "2026-05-24T10:01:00Z",
            error: null,
          },
        ],
      }),
    });

    const summaries = await fetchVisualSummaries("2026-05-24", fetcher);

    expect(fetcher).toHaveBeenCalledWith(
      expectedDateQuery("/api/visual-summaries", "2026-05-24"),
    );
    expect(summaries[0].modelProvider).toBe("local_stub");
    expect(summaries[0].visibleApps).toEqual(["Code.exe"]);
  });

  it("rejects invalid visual summary rows", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        summaries: [{ id: 1 }],
      }),
    });

    await expect(fetchVisualSummaries("2026-05-24", fetcher)).rejects.toThrow(
      /invalid visual summary/i,
    );
  });
});

describe("analyzeScreenshot", () => {
  it("posts to the collector screenshot analysis endpoint", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      statusText: "OK",
      json: async () => ({
        summary: {
          id: 2,
          screenshotId: 10,
          capturedAt: "2026-05-24T10:00:00Z",
          modelProvider: "minimax",
          modelName: "MiniMax-M3",
          promptVersion: "visual-summary-minimax-m3-v1",
          summaryText: "MiniMax summary",
          activityCategory: "coding",
          projectHints: ["Time State Recorder"],
          visibleApps: ["Code.exe"],
          visibleTextHints: ["main.rs"],
          riskFlags: [],
          confidence: 0.82,
          createdAt: "2026-05-24T10:01:00Z",
          error: null,
        },
      }),
    });

    const summary = await analyzeScreenshot(10, fetcher);

    expect(fetcher).toHaveBeenCalledWith("/api/screenshots/10/analyze", {
      method: "POST",
    });
    expect(summary.modelProvider).toBe("minimax");
    expect(summary.summaryText).toBe("MiniMax summary");
  });
});
