import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { InsightFeedback } from "./InsightFeedback";
import type { AnalysisStatus, InsightReport, VisualWindowSummary } from "./types";

const longRawPhrase =
  "xhs_boundary_spanner_task1_5_rawdata_technical_design_v0_2";

describe("InsightFeedback", () => {
  it("keeps raw review notes readable with disclosure controls", () => {
    render(
      <InsightFeedback
        analysisStatus={analysisStatus()}
        reports={[report()]}
        privacyMode="raw"
        sourceMode="live"
        loading={false}
      />,
    );

    expect(
      screen.getByRole("region", { name: /window note summary/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /raw window evidence/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /raw session report/i }),
    ).toBeInTheDocument();
    expect(
      screen.getAllByText(/Main document schema audit/).length,
    ).toBeGreaterThan(0);
    expect(screen.queryByText(longRawPhrase)).not.toBeInTheDocument();
  });
});

function analysisStatus(): AnalysisStatus {
  return {
    visual: {
      status: "idle",
      lastStartedAt: "2026-06-06T01:00:00Z",
      lastFinishedAt: "2026-06-06T01:00:05Z",
      nextRunAt: "2026-06-06T01:05:00Z",
    },
    report: {
      status: "idle",
      lastStartedAt: "2026-06-06T01:00:05Z",
      lastFinishedAt: "2026-06-06T01:00:10Z",
      nextRunAt: "2026-06-06T06:00:00Z",
    },
    latestWindowSummary: windowSummary(),
    latestReport: report(),
  };
}

function windowSummary(): VisualWindowSummary {
  const raw = {
    summaryText: "Main document schema audit in Word.",
    taskIntent: `User is reviewing ${longRawPhrase} and raw_control.collection_run fields with a very long schema/codebook trail that should not dominate the card.`,
    continuity: `Continues from Obsidian tables into Word with ${longRawPhrase}, raw_public.note_snapshot, raw_link.note_product_link, and comment_thread_link references.`,
    projectHints: [
      "Main line: raw data schema",
      `Word ${longRawPhrase} document review with long visible title and row names`,
    ],
    trajectory: [
      {
        minuteMark: 1,
        screenshotId: 1,
        observation: `Metadata-only observation shows WINWORD.EXE focused on ${longRawPhrase} with many page counters and field names.`,
        activityCategory: "writing" as const,
      },
    ],
  };

  return {
    id: 41,
    windowStart: "2026-06-06T01:00:00Z",
    windowEnd: "2026-06-06T01:05:00Z",
    sampledScreenshotIds: [1, 3, 5],
    modelProvider: "minimax",
    modelName: "MiniMax-M3",
    promptVersion: "visual-window-minimax-m3-v1",
    summaryText: JSON.stringify(raw),
    continuity: "Long raw continuity fallback",
    primaryActivity: "writing",
    projectHints: raw.projectHints,
    taskIntent: "Audit raw data schema",
    trajectory: raw.trajectory,
    switchingLevel: "unknown",
    switchingEvidence: `Switching evidence mentions ${longRawPhrase} and should be disclosed without stretching the main grid.`,
    loafingLevel: "unknown",
    loafingEvidence: "",
    visibleApps: ["WINWORD.EXE"],
    visibleTextHints: [longRawPhrase],
    riskFlags: [],
    confidence: 0.74,
    rawSummaryJson: raw,
    createdAt: "2026-06-06T01:05:05Z",
  };
}

function report(): InsightReport {
  return {
    id: 11,
    periodStart: "2026-06-05T23:27:00Z",
    periodEnd: "2026-06-06T04:27:00Z",
    generatedAt: "2026-06-06T04:27:10Z",
    reportKind: "5h",
    modelProvider: "minimax",
    modelName: "MiniMax-M3",
    summaryText:
      "本5小时窗口以晚间自研工具元层复盘为主。① Word审阅阶段：持续检查raw_control.collection_run与raw_link相关字段。② Codex阶段：整理布局与可读性。③ 浏览器阶段：核对本地项目线索。整体呈现长文本证据较多但应先摘要展示。",
    categoryMix: [{ activityCategory: "writing", count: 3 }],
    projectHints: [
      "主线1:raw data schema",
      `主线2:${longRawPhrase} and associated raw_link tables`,
    ],
    evidenceCount: 16,
  };
}
