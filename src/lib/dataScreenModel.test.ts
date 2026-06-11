import { describe, expect, it } from "vitest";
import type {
  DailyBriefResponse,
  HourlyActivityMetric,
  InsightReport,
  VisualWindowSummary,
} from "../types";
import {
  buildDataScreenModel,
  buildDateRange,
  extractReportTerms,
} from "./dataScreenModel";

describe("data screen model", () => {
  it("builds a single-day model from daily brief, 5-minute windows, and reports", () => {
    const model = buildDataScreenModel({
      anchorDate: "2026-06-11",
      period: "day",
      days: [
        day("2026-06-11", {
          activeSeconds: 18_000,
          switchCount: 36,
          inputChars: 12_000,
          visualWindowCount: 24,
          hourlyReports: [report(1, "1h", "Codex dashboard planning and dashboard charts")],
          fiveHourReports: [report(2, "5h", "Time State Recorder dashboard implementation")],
          hourlyMetrics: [
            { hour: 9, activeSeconds: 1800, activeRatio: 0.5, inputChars: 700 },
            { hour: 10, activeSeconds: 3000, activeRatio: 0.83, inputChars: 1400 },
          ],
        }),
      ],
      visualWindowsByDate: {
        "2026-06-11": [
          windowSummary(1, "2026-06-11T01:00:00Z", "2026-06-11T01:05:00Z", {
            taskIntent: "实现 dashboard 聚合模型",
            projectHints: ["time-state-recorder", "dashboard"],
            switchingLevel: "low",
            loafingLevel: "none",
          }),
          windowSummary(2, "2026-06-11T01:05:00Z", "2026-06-11T01:10:00Z", {
            taskIntent: "调整图表布局",
            projectHints: ["dashboard"],
            switchingLevel: "medium",
            loafingLevel: "possible",
          }),
        ],
      },
    });

    expect(model.dateRange).toEqual(["2026-06-11"]);
    expect(model.kpis.activeHours.value).toBe("5.0h");
    expect(model.kpis.switchesPerHour.value).toBe("7.2");
    expect(model.kpis.inputChars.value).toBe("12,000");
    expect(model.kpis.reportCoverage.value).toBe("2");
    expect(model.hourlyHeatmap).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ hour: 9, activeSeconds: 1800, activeRatio: 0.5 }),
        expect.objectContaining({ hour: 10, activeSeconds: 3000, activeRatio: 0.83 }),
      ]),
    );
    expect(model.projectRank.slice(0, 2)).toEqual([
      { label: "dashboard", count: 3 },
      { label: "time-state-recorder", count: 2 },
    ]);
    expect(model.trajectory).toEqual([
      expect.objectContaining({
        id: "window-1",
        intent: "实现 dashboard 聚合模型",
        switchingLevel: "low",
        loafingLevel: "none",
      }),
      expect.objectContaining({
        id: "window-2",
        intent: "调整图表布局",
        switchingLevel: "medium",
        loafingLevel: "possible",
      }),
    ]);
    expect(model.textSignals.topTerms[0]).toEqual({ label: "dashboard", count: 3 });
  });

  it("aggregates week and month ranges from the anchor date", () => {
    expect(buildDateRange("2026-06-11", "week")).toEqual([
      "2026-06-08",
      "2026-06-09",
      "2026-06-10",
      "2026-06-11",
      "2026-06-12",
      "2026-06-13",
      "2026-06-14",
    ]);
    expect(buildDateRange("2026-06-11", "month")[0]).toBe("2026-06-01");
    expect(buildDateRange("2026-06-11", "month").at(-1)).toBe("2026-06-30");
  });

  it("handles missing dates without inflating coverage", () => {
    const model = buildDataScreenModel({
      anchorDate: "2026-06-11",
      period: "week",
      days: [
        day("2026-06-10", { activeSeconds: 3600, switchCount: 3, inputChars: 1000 }),
        day("2026-06-11", { activeSeconds: 7200, switchCount: 8, inputChars: 2000 }),
      ],
      visualWindowsByDate: {},
    });

    expect(model.dateRange).toHaveLength(7);
    expect(model.availableDayCount).toBe(2);
    expect(model.kpis.activeHours.value).toBe("3.0h");
    expect(model.dailyTrend.map((point) => point.activeHours)).toEqual([
      0, 0, 3_600 / 3600, 7_200 / 3600, 0, 0, 0,
    ]);
  });

  it("extracts conservative report terms and ignores common filler", () => {
    expect(
      extractReportTerms([
        report(1, "1h", "这是 dashboard dashboard 的实现，包含 active hours 和 report coverage。"),
        report(2, "5h", "dashboard 轨迹分析，Time State Recorder report。"),
      ]).slice(0, 3),
    ).toEqual([
      { label: "dashboard", count: 3 },
      { label: "report", count: 2 },
      { label: "coverage", count: 1 },
    ]);
  });
});

function day(
  date: string,
  overrides: {
    activeSeconds?: number;
    switchCount?: number;
    inputChars?: number;
    visualWindowCount?: number;
    hourlyReports?: InsightReport[];
    fiveHourReports?: InsightReport[];
    hourlyMetrics?: TestHourlyMetric[];
  } = {},
): DailyBriefResponse {
  return {
    date,
    status: "complete",
    brief: undefined,
    hourlyReports: overrides.hourlyReports ?? [],
    fiveHourReports: overrides.fiveHourReports ?? [],
    descriptiveStats: {
      date,
      periodStart: `${date}T00:00:00Z`,
      periodEnd: `${date}T23:59:59Z`,
      activeSeconds: overrides.activeSeconds ?? 0,
      activeHours: (overrides.activeSeconds ?? 0) / 3600,
      windowEventCount: 0,
      switchCount: overrides.switchCount ?? 0,
      distinctAppCount: 0,
      topApps: [
        { processName: "Code.exe", activeSeconds: overrides.activeSeconds ?? 0, share: 1 },
      ],
      categoryMix: [
        { activityCategory: "coding", count: 2 },
        { activityCategory: "research", count: 1 },
      ],
      inputChars: overrides.inputChars ?? 0,
      inputEvents: 0,
      screenshotCount: 0,
      highResScreenshotCount: 0,
      visualWindowCount: overrides.visualWindowCount ?? 0,
      fiveHourReportCount: overrides.fiveHourReports?.length ?? 0,
      firstActivityAt: `${date}T01:00:00Z`,
      lastActivityAt: `${date}T09:00:00Z`,
    },
    hourlyMetrics: overrides.hourlyMetrics?.map((metric) => ({
      startAt: `${date}T${String(metric.hour).padStart(2, "0")}:00:00Z`,
      endAt: `${date}T${String(metric.hour + 1).padStart(2, "0")}:00:00Z`,
      windowEventCount: 0,
      switchCount: 0,
      distinctAppCount: 0,
      dominantCategory: "coding",
      screenshotCount: 0,
      highResScreenshotCount: 0,
      visualWindowCount: 0,
      fiveHourReportIds: [],
      dominantApp: "Code.exe",
      ...metric,
    })) ?? [],
    comparison: {
      baselineDays: 0,
      comparedDates: [],
      activeSecondsDelta: 0,
      switchesPerHourDelta: 0,
      inputCharsDelta: 0,
      screenshotCoverageDelta: 0,
      explanation: "",
    },
  };
}

type TestHourlyMetric = Pick<
  HourlyActivityMetric,
  "hour" | "activeSeconds" | "activeRatio" | "inputChars"
> &
  Partial<HourlyActivityMetric>;

function report(id: number, kind: string, text: string): InsightReport {
  return {
    id,
    periodStart: "2026-06-11T01:00:00Z",
    periodEnd: "2026-06-11T02:00:00Z",
    generatedAt: "2026-06-11T02:05:00Z",
    reportKind: kind,
    modelProvider: "minimax",
    modelName: "MiniMax-M3",
    summaryText: text,
    categoryMix: [{ activityCategory: "coding", count: 1 }],
    projectHints: ["dashboard"],
    evidenceCount: 3,
  };
}

function windowSummary(
  id: number,
  windowStart: string,
  windowEnd: string,
  overrides: Partial<VisualWindowSummary>,
): VisualWindowSummary {
  return {
    id,
    windowStart,
    windowEnd,
    sampledScreenshotIds: [id],
    modelProvider: "minimax",
    modelName: "MiniMax-M3",
    promptVersion: "visual-window-v1",
    summaryText: "窗口摘要",
    continuity: "",
    primaryActivity: "coding",
    projectHints: [],
    taskIntent: "",
    trajectory: [
      {
        minuteMark: 1,
        screenshotId: id,
        observation: "继续工作",
        activityCategory: "coding",
      },
    ],
    switchingLevel: "low",
    switchingEvidence: "",
    loafingLevel: "none",
    loafingEvidence: "",
    visibleApps: ["Code.exe"],
    visibleTextHints: [],
    riskFlags: [],
    confidence: 0.8,
    rawSummaryJson: null,
    createdAt: windowEnd,
    ...overrides,
  };
}
