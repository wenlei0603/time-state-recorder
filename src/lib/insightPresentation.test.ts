import { describe, expect, it } from "vitest";
import type { InsightReport, VisualWindowSummary } from "../types";
import {
  buildReportNote,
  buildWindowNote,
  stripJsonSummarySyntax,
} from "./insightPresentation";

describe("insightPresentation", () => {
  it("extracts a fenced JSON summary instead of returning raw object syntax", () => {
    const note = buildWindowNote({
      ...windowSummary(),
      summaryText:
        '```json\n{"summaryText":"Focused Overpayment analysis.","taskIntent":"Prepare regression table","continuity":"Continues formal analysis"}\n```',
    });

    expect(note.thesis).toBe("Focused Overpayment analysis.");
    expect(note.intent).toBe("Prepare regression table");
    expect(note.continuity).toBe("Continues formal analysis");
    expect(note.rawAvailable).toBe(true);
    expect(note.thesis).not.toMatch(/[{}]|```|summaryText/);
  });

  it("turns a long five-hour report into readable sections", () => {
    const note = buildReportNote({
      ...report(),
      summaryText:
        "5小时工作轨迹可分四个阶段。① 教学协调阶段(06:20-07:00)：处理课程材料。② Stata实证阶段(07:15-07:35)：推进do-file。③ Codex工程阶段(07:35-08:10)：整理worktree。整体呈现科研-工程并行。",
    });

    expect(note.mainThread).toContain("5小时工作轨迹");
    expect(note.phases).toHaveLength(3);
    expect(note.phases[0].label).toContain("教学协调阶段");
    expect(note.fullText).toContain("整体呈现");
  });

  it("removes JSON wrapper syntax without hiding plain prose", () => {
    expect(stripJsonSummarySyntax("Plain summary.")).toBe("Plain summary.");
    expect(stripJsonSummarySyntax('{"summaryText":"Plain JSON summary."}')).toBe(
      "Plain JSON summary.",
    );
  });

  it("uses known structured fields when the summary field is empty", () => {
    const note = buildWindowNote({
      ...windowSummary(),
      summaryText: '{"projectHints":["Overpayment"],"primaryActivity":"coding"}',
      taskIntent: "Write a formal analysis report",
    });

    expect(note.thesis).toBe("Write a formal analysis report");
    expect(note.projectHints).toEqual(["Overpayment"]);
  });
});

function windowSummary(): VisualWindowSummary {
  return {
    id: 9,
    windowStart: "2026-06-04T09:00:00Z",
    windowEnd: "2026-06-04T09:05:00Z",
    sampledScreenshotIds: [1, 3, 5],
    previousSummaryId: undefined,
    modelProvider: "minimax",
    modelName: "MiniMax-M3",
    promptVersion: "visual-window-minimax-m3-v1",
    summaryText: "Focused coding work.",
    continuity: "Continues the implementation thread.",
    primaryActivity: "coding",
    projectHints: ["Time State Recorder"],
    taskIntent: "Redesign review notes",
    trajectory: [
      {
        minuteMark: 1,
        screenshotId: 1,
        observation: "Opened the frontend component.",
        activityCategory: "coding",
      },
      {
        minuteMark: 3,
        screenshotId: 3,
        observation: "Adjusted summary presentation.",
        activityCategory: "coding",
      },
      {
        minuteMark: 5,
        screenshotId: 5,
        observation: "Checked the rendered note.",
        activityCategory: "coding",
      },
    ],
    switchingLevel: "low",
    switchingEvidence: "Stayed inside the same project.",
    loafingLevel: "none",
    loafingEvidence: "No unrelated browsing visible.",
    visibleApps: ["Code"],
    visibleTextHints: ["InsightFeedback.tsx"],
    riskFlags: [],
    confidence: 0.86,
    rawSummaryJson: null,
    createdAt: "2026-06-04T09:05:03Z",
    error: undefined,
  };
}

function report(): InsightReport {
  return {
    id: 2,
    periodStart: "2026-06-04T04:00:00Z",
    periodEnd: "2026-06-04T09:00:00Z",
    generatedAt: "2026-06-04T09:02:00Z",
    reportKind: "5h",
    modelProvider: "minimax",
    modelName: "MiniMax-M3",
    summaryText: "Five-hour trajectory summary.",
    categoryMix: [{ activityCategory: "coding", count: 8 }],
    projectHints: ["Time State Recorder"],
    evidenceCount: 17,
    error: undefined,
  };
}
