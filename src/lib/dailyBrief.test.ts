import { describe, expect, it, vi } from "vitest";
import { fetchDailyBrief } from "./dailyBrief";
import { fetchInsightReports } from "./insights";

function jsonResponse(body: unknown) {
  return {
    ok: true,
    status: 200,
    statusText: "OK",
    json: async () => body,
  };
}

describe("daily brief API", () => {
  it("loads date-scoped insight reports for the selected collector date", async () => {
    const fetcher = vi.fn().mockResolvedValue(
      jsonResponse({
        reports: [insightReport()],
      }),
    );

    const reports = await fetchInsightReports(
      { date: "2026-06-03", kind: "5h", limit: 10 },
      fetcher,
    );

    expect(String(fetcher.mock.calls[0][0])).toMatch(
      /^\/api\/insight-reports\?date=2026-06-03&tzOffsetMinutes=.*&kind=5h&limit=10$/,
    );
    expect(reports[0].summaryText).toBe("上午报告。");
  });

  it("parses daily brief response with stats, hourly heatmap, and same-day reports", async () => {
    const fetcher = vi.fn().mockResolvedValue(jsonResponse(dailyBriefResponse()));

    const response = await fetchDailyBrief("2026-06-03", fetcher);

    expect(String(fetcher.mock.calls[0][0])).toMatch(
      /^\/api\/daily-brief\?date=2026-06-03&tzOffsetMinutes=/,
    );
    expect(response.status).toBe("complete");
    expect(response.brief?.dailySummaryText).toBe("当天以编码和阅读窗口为主。");
    expect(response.descriptiveStats.activeSeconds).toBe(3600);
    expect(response.hourlyMetrics[0].hour).toBe(9);
    expect(response.hourlyMetrics[0].fiveHourReportIds).toEqual([2]);
    expect(response.fiveHourReports[0].summaryText).toBe("上午报告。");
  });
});

function insightReport() {
  return {
    id: 2,
    periodStart: "2026-06-03T05:00:00Z",
    periodEnd: "2026-06-03T10:00:00Z",
    generatedAt: "2026-06-03T10:01:00Z",
    reportKind: "5h",
    modelProvider: "local_insight",
    modelName: "trajectory-v1",
    summaryText: "上午报告。",
    categoryMix: [{ activityCategory: "coding", count: 3 }],
    projectHints: ["Time State Recorder"],
    evidenceCount: 3,
    error: null,
  };
}

function dailyBriefResponse() {
  return {
    date: "2026-06-03",
    status: "complete",
    nextRunAt: "2026-06-03T15:40:00Z",
    brief: {
      id: 5,
      date: "2026-06-03",
      periodStart: "2026-06-03T00:00:00Z",
      periodEnd: "2026-06-04T00:00:00Z",
      generatedAt: "2026-06-03T15:40:05Z",
      scheduledForLocal: "23:40",
      modelProvider: "local_insight",
      modelName: "daily-brief-local-v1",
      promptVersion: "daily-brief-v1",
      status: "complete",
      descriptiveStats: dailyStats(),
      hourlyMetrics: [hourlyMetric()],
      comparison: comparison(),
      fiveHourReportIds: [2],
      dailySummaryText: "当天以编码和阅读窗口为主。",
      actionTrajectory: "上午出现编码窗口，下午出现阅读窗口。",
      rawSummaryJson: {
        dailySummaryText: "当天以编码和阅读窗口为主。",
        actionTrajectory: "上午出现编码窗口，下午出现阅读窗口。",
      },
      error: null,
    },
    fiveHourReports: [insightReport()],
    descriptiveStats: dailyStats(),
    hourlyMetrics: [hourlyMetric()],
    comparison: comparison(),
  };
}

function dailyStats() {
  return {
    date: "2026-06-03",
    periodStart: "2026-06-03T00:00:00Z",
    periodEnd: "2026-06-04T00:00:00Z",
    activeSeconds: 3600,
    activeHours: 1,
    windowEventCount: 4,
    switchCount: 2,
    distinctAppCount: 2,
    topApps: [{ processName: "Code.exe", activeSeconds: 2400, share: 0.67 }],
    categoryMix: [{ activityCategory: "coding", count: 2 }],
    inputChars: 120,
    inputEvents: 140,
    screenshotCount: 6,
    highResScreenshotCount: 3,
    visualWindowCount: 4,
    fiveHourReportCount: 1,
    firstActivityAt: "2026-06-03T05:00:00Z",
    lastActivityAt: "2026-06-03T15:00:00Z",
  };
}

function hourlyMetric() {
  return {
    hour: 9,
    startAt: "2026-06-03T09:00:00Z",
    endAt: "2026-06-03T10:00:00Z",
    activeSeconds: 1800,
    activeRatio: 0.5,
    windowEventCount: 2,
    switchCount: 1,
    distinctAppCount: 2,
    dominantApp: "Code.exe",
    dominantCategory: "coding",
    inputChars: 60,
    screenshotCount: 2,
    highResScreenshotCount: 1,
    visualWindowCount: 1,
    fiveHourReportIds: [2],
  };
}

function comparison() {
  return {
    baselineDays: 7,
    comparedDates: ["2026-06-02"],
    activeSecondsDelta: 600,
    switchesPerHourDelta: 0.2,
    inputCharsDelta: 120,
    screenshotCoverageDelta: 0.1,
    dominantCategoryShift: "research -> coding",
    startTimeShiftMinutes: -10,
    endTimeShiftMinutes: 20,
    explanation: "编码窗口较前一日增加。",
  };
}
