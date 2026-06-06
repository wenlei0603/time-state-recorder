import { describe, expect, it } from "vitest";
import type { DailyBrief, InsightReport } from "../types";
import {
  normalizeVisibleTimeText,
  presentDailyNarrative,
  presentInsightReport,
  splitReportText,
  truncateText,
} from "./reportPresentation";

function report(overrides: Partial<InsightReport> = {}): InsightReport {
  return {
    id: 9,
    periodStart: "2026-06-07T02:00:00Z",
    periodEnd: "2026-06-07T03:00:00Z",
    generatedAt: "2026-06-07T03:00:05Z",
    reportKind: "1h",
    modelProvider: "local_insight",
    modelName: "trajectory-v1",
    summaryText:
      "1) Notion RAW 字段整理，补全 raw_link 与 note_product_link。2) Codex 前端计划，处理 hourlyReports 与 5h Reports。3) ERROR 待排查：长文本直接渲染造成巨大段落。",
    categoryMix: [
      { activityCategory: "coding", count: 8 },
      { activityCategory: "research", count: 4 },
    ],
    projectHints: ["Time State Recorder", "Notion OS"],
    evidenceCount: 12,
    error: null,
    ...overrides,
  };
}

function brief(overrides: Partial<DailyBrief> = {}): DailyBrief {
  return {
    id: 1,
    date: "2026-06-07",
    periodStart: "2026-06-07T00:00:00Z",
    periodEnd: "2026-06-08T00:00:00Z",
    generatedAt: "2026-06-07T23:50:00Z",
    scheduledForLocal: "23:40",
    modelProvider: "local_insight",
    modelName: "daily-brief-v1",
    promptVersion: "daily-brief-v1",
    status: "complete",
    descriptiveStats: {
      date: "2026-06-07",
      periodStart: "2026-06-07T00:00:00Z",
      periodEnd: "2026-06-08T00:00:00Z",
      activeSeconds: 3600,
      activeHours: 1,
      windowEventCount: 2,
      switchCount: 1,
      distinctAppCount: 1,
      topApps: [],
      categoryMix: [],
      inputChars: 0,
      inputEvents: 0,
      screenshotCount: 0,
      highResScreenshotCount: 0,
      visualWindowCount: 0,
      fiveHourReportCount: 1,
    },
    hourlyMetrics: [],
    comparison: {
      baselineDays: 7,
      comparedDates: [],
      activeSecondsDelta: 0,
      switchesPerHourDelta: 0,
      inputCharsDelta: 0,
      screenshotCoverageDelta: 0,
      explanation: "无显著变化。",
    },
    dailySummaryText:
      "上午：推进报告 cadence。下午：整理前端结构化呈现。晚上：验证 API 与 UI。",
    actionTrajectory:
      "09:00-10:00 处理 hourly 报告。15:00-20:00 处理 scheduled 5h 报告。待排查：移动端溢出。",
    fiveHourReportIds: [2],
    rawSummaryJson: {},
    error: null,
    ...overrides,
  };
}

describe("reportPresentation", () => {
  it("splits numbered long report text into bounded visible sections", () => {
    const presentation = presentInsightReport(report());

    expect(presentation.title).toBe("1h Report");
    expect(presentation.timeRange).toBe("10:00 - 11:00");
    expect(presentation.overview.length).toBeLessThanOrEqual(180);
    expect(presentation.phases.length).toBeGreaterThanOrEqual(3);
    expect(presentation.phases[0].body).toContain("Notion RAW");
    expect(presentation.chips).toContain("Time State Recorder");
    expect(presentation.chips).toContain("12 windows");
    expect(presentation.uncertainty.join(" ")).toContain("ERROR");
    expect(presentation.rawText).toContain("长文本直接渲染");
  });

  it("extracts sections from Chinese punctuation and time spans", () => {
    const sections = splitReportText(
      "10:00-15:00 文献与 Notion RAW；15:00-20:00 Codex 前端设计；20:00-01:00 验证与 PR 准备。",
    );

    expect(sections).toHaveLength(3);
    expect(sections[1]).toContain("Codex 前端设计");
  });

  it("structures daily narrative without losing raw text", () => {
    const presentation = presentDailyNarrative(brief());

    expect(presentation.title).toBe("Daily Action Trajectory");
    expect(presentation.phases.length).toBeGreaterThanOrEqual(3);
    expect(presentation.uncertainty.join(" ")).toContain("待排查");
    expect(presentation.rawText).toContain("09:00-10:00");
  });

  it("normalizes visible legacy UTC timestamp tokens but preserves raw text", () => {
    const presentation = presentInsightReport(
      report({
        periodStart: "2026-06-07T02:00:00Z",
        periodEnd: "2026-06-07T07:00:00Z",
        summaryText:
          "2026-06-07T02:00:00Z 到 2026-06-07T07:00:00Z 处理 Time State Recorder 报告。",
      }),
    );

    expect(normalizeVisibleTimeText("2026-06-07T02:00:00Z")).toBe("10:00");
    expect(presentation.overview).toContain("10:00");
    expect(presentation.overview).toContain("15:00");
    expect(presentation.overview).not.toContain("02:00:00Z");
    expect(presentation.rawText).toContain("2026-06-07T02:00:00Z");
  });

  it("truncates visible text deterministically", () => {
    expect(truncateText("a".repeat(400), 20)).toHaveLength(20);
  });
});
