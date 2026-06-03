import { describe, expect, it, vi } from "vitest";
import {
  fetchAnalysisStatus,
  fetchInsightReports,
  fetchVisualObservations,
} from "./insights";

function jsonResponse(body: unknown) {
  return {
    ok: true,
    status: 200,
    statusText: "OK",
    json: async () => body,
  };
}

describe("insights API", () => {
  it("loads analysis status with latest observation and report", async () => {
    const fetcher = vi.fn().mockResolvedValue(
      jsonResponse({
        visual: {
          status: "idle",
          lastStartedAt: "2026-06-03T10:00:00Z",
          lastFinishedAt: "2026-06-03T10:00:04Z",
          nextRunAt: "2026-06-03T10:05:00Z",
          lastError: null,
        },
        report: {
          status: "idle",
          lastStartedAt: null,
          lastFinishedAt: null,
          nextRunAt: "2026-06-03T15:00:00Z",
          lastError: null,
        },
        latestObservation: visualObservation(),
        latestReport: insightReport(),
      }),
    );

    const status = await fetchAnalysisStatus(fetcher);

    expect(fetcher).toHaveBeenCalledWith("/api/analysis-status");
    expect(status.visual.nextRunAt).toBe("2026-06-03T10:05:00Z");
    expect(status.latestObservation?.summaryText).toBe("Focused coding work.");
    expect(status.latestReport?.evidenceCount).toBe(12);
  });

  it("loads report and visual observation query endpoints", async () => {
    const fetcher = vi.fn().mockResolvedValueOnce(
      jsonResponse({
        reports: [insightReport()],
      }),
    ).mockResolvedValueOnce(
      jsonResponse({
        observations: [visualObservation()],
      }),
    );

    const reports = await fetchInsightReports(3, fetcher);
    const observations = await fetchVisualObservations("2026-06-03", fetcher);

    expect(fetcher).toHaveBeenNthCalledWith(1, "/api/insight-reports?limit=3");
    expect(String(fetcher.mock.calls[1][0])).toMatch(
      /^\/api\/visual-observations\?date=2026-06-03&tzOffsetMinutes=/,
    );
    expect(reports[0].categoryMix[0].activityCategory).toBe("coding");
    expect(observations[0].highResScreenshotId).toBe(44);
  });
});

function visualObservation() {
  return {
    id: 7,
    highResScreenshotId: 44,
    capturedAt: "2026-06-03T10:00:00Z",
    filePath: "2026-06-03/10-00-00.jpg",
    modelProvider: "minimax",
    modelName: "MiniMax-M3",
    promptVersion: "visual-summary-minimax-m3-v1",
    summaryText: "Focused coding work.",
    activityCategory: "coding",
    projectHints: ["Time State Recorder"],
    visibleApps: ["Code"],
    visibleTextHints: ["collector/src/api.rs"],
    riskFlags: [],
    confidence: 0.82,
    createdAt: "2026-06-03T10:00:04Z",
    error: null,
  };
}

function insightReport() {
  return {
    id: 2,
    periodStart: "2026-06-03T05:00:00Z",
    periodEnd: "2026-06-03T10:00:00Z",
    generatedAt: "2026-06-03T10:01:00Z",
    reportKind: "5h",
    modelProvider: "local_insight",
    modelName: "trajectory-v1",
    summaryText: "Five-hour trajectory summary.",
    categoryMix: [{ activityCategory: "coding", count: 9 }],
    projectHints: ["Time State Recorder"],
    evidenceCount: 12,
    error: null,
  };
}
