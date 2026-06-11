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

    expect(fetcher).toHaveBeenCalledWith(
      "/api/insight-reports?date=2026-06-03&tzOffsetMinutes=-480&kind=5h&limit=10",
    );
    expect(reports[0].summaryText).toBe("上午报告。");
  });

  it("parses daily brief response with stats, hourly heatmap, and same-day reports", async () => {
    const fetcher = vi.fn().mockResolvedValue(jsonResponse(dailyBriefResponse()));

    const response = await fetchDailyBrief("2026-06-03", fetcher);

    expect(fetcher).toHaveBeenCalledWith(
      "/api/daily-brief?date=2026-06-03&tzOffsetMinutes=-480",
    );
    expect(response.status).toBe("complete");
    expect(response.brief?.dailySummaryText).toBe("当天以编码和阅读窗口为主。");
    expect(response.descriptiveStats.activeSeconds).toBe(3600);
    expect(response.hourlyMetrics[0].hour).toBe(9);
    expect(response.hourlyMetrics[0].fiveHourReportIds).toEqual([2]);
    expect(response.hourlyReports[0].reportKind).toBe("1h");
    expect(response.hourlyReports[0].summaryText).toBe("09点小时报告。");
    expect(response.fiveHourReports[0].summaryText).toBe("上午报告。");
    expect(response.diaryDashboard?.overview).toBe(
      "当天围绕两个团队推进后端报告和论文阅读。",
    );
    expect(response.diaryDashboard?.teamCount).toBe(2);
    expect(response.diaryDashboard?.collaborators[0].label).toBe("课程助教");
    expect(response.diaryDashboard?.roleHeterogeneity.level).toBe("medium");
  });

  it("accepts old daily brief API responses before hourlyReports exists", async () => {
    const body = dailyBriefResponse();
    delete (body as Partial<typeof body>).hourlyReports;
    delete (body as Partial<typeof body>).diaryDashboard;
    const fetcher = vi.fn().mockResolvedValue(jsonResponse(body));

    const response = await fetchDailyBrief("2026-06-03", fetcher);

    expect(response.hourlyReports).toEqual([]);
    expect(response.diaryDashboard).toBeUndefined();
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

function hourlyReport() {
  return {
    id: 9,
    periodStart: "2026-06-03T09:00:00Z",
    periodEnd: "2026-06-03T10:00:00Z",
    generatedAt: "2026-06-03T10:00:05Z",
    reportKind: "1h",
    modelProvider: "local_insight",
    modelName: "trajectory-v1",
    summaryText: "09点小时报告。",
    categoryMix: [{ activityCategory: "coding", count: 12 }],
    projectHints: ["Time State Recorder"],
    evidenceCount: 12,
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
    hourlyReports: [hourlyReport()],
    fiveHourReports: [insightReport()],
    descriptiveStats: dailyStats(),
    hourlyMetrics: [hourlyMetric()],
    comparison: comparison(),
    diaryDashboard: diaryDashboard(),
  };
}

function diaryDashboard() {
  return {
    overview: "当天围绕两个团队推进后端报告和论文阅读。",
    collaborators: [
      {
        label: "课程助教",
        description: "从邮件和课程窗口推断存在教学协作。",
        activeSeconds: null,
        share: null,
        evidence: ["课程邮件"],
        confidence: 0.62,
      },
    ],
    locations: [
      {
        label: "VS Code",
        description: "主要数字工作场所。",
        activeSeconds: 2400,
        share: 0.67,
        evidence: ["Code.exe"],
        confidence: 0.8,
      },
    ],
    workTypes: [
      {
        label: "coding",
        description: "后端报告与 API 调试。",
        activeSeconds: 2400,
        share: 0.67,
        evidence: ["上午报告"],
        confidence: 0.8,
      },
    ],
    roles: [
      {
        label: "backend implementer",
        description: "实现和验证后端功能。",
        activeSeconds: 2400,
        share: 0.67,
        teams: ["Time State Recorder"],
        evidence: ["API 测试"],
      },
    ],
    roleHeterogeneity: {
      level: "medium",
      score: 0.5,
      summary: "编码和研究阅读并行。",
      distinctRoleCount: 2,
    },
    teamCount: 2,
    teams: [
      {
        name: "Time State Recorder",
        activeSeconds: 2400,
        share: 0.67,
        workTypes: ["coding"],
        role: "backend implementer",
        evidence: ["上午报告"],
      },
    ],
    timeDistribution: [
      {
        label: "morning",
        startAt: "2026-06-03T05:00:00Z",
        endAt: "2026-06-03T10:00:00Z",
        activeSeconds: 2400,
        primaryTeam: "Time State Recorder",
        primaryWorkType: "coding",
        role: "backend implementer",
        summary: "上午集中在后端报告。",
      },
    ],
    evidenceSummary: "基于 daily brief、hourly metrics 和 5h reports。",
    uncertainty: "无法从窗口数据可靠识别全部真实人员。",
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
