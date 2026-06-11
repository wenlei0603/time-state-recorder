import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DataScreen } from "./DataScreen";

vi.mock("./EChartPanel", () => ({
  EChartPanel: ({ title }: { title: string }) => <div data-testid="chart">{title}</div>,
}));

describe("DataScreen", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("waits for the user to click update before fetching dashboard data", async () => {
    const fetcher = vi.fn(fetcherResponse);

    render(<DataScreen anchorDate="2026-06-11" fetcher={fetcher} />);

    expect(fetcher).not.toHaveBeenCalled();
    expect(screen.getByText("No dashboard snapshot loaded.")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /update screen/i }));

    expect(await screen.findAllByText("5.0h")).not.toHaveLength(0);
    expect(screen.getByText("1 hourly / 1 5h")).toBeInTheDocument();
    expect(screen.getAllByTestId("chart").map((node) => node.textContent)).toEqual(
      expect.arrayContaining(["Activity Trend", "Category Mix", "Hourly Heatmap"]),
    );
    expect(fetcher).toHaveBeenCalledWith(
      "/api/daily-brief?date=2026-06-11&tzOffsetMinutes=-480",
    );
    expect(fetcher).toHaveBeenCalledWith(
      "/api/visual-window-summaries?date=2026-06-11&tzOffsetMinutes=-480",
    );
  });

  it("loads the selected period only after the next manual update", async () => {
    const fetcher = vi.fn(fetcherResponse);

    render(<DataScreen anchorDate="2026-06-11" fetcher={fetcher} />);

    fireEvent.click(screen.getByRole("button", { name: /^week$/i }));
    expect(fetcher).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: /update screen/i }));

    await waitFor(() => {
      expect(fetcher).toHaveBeenCalledWith(
        "/api/daily-brief?date=2026-06-08&tzOffsetMinutes=-480",
      );
    });
    expect(fetcher).toHaveBeenCalledWith(
      "/api/daily-brief?date=2026-06-14&tzOffsetMinutes=-480",
    );
    expect(await screen.findByText("7/7 days loaded")).toBeInTheDocument();
  });
});

async function fetcherResponse(input: string) {
  const url = new URL(input, "http://127.0.0.1");
  const date = url.searchParams.get("date") ?? "2026-06-11";
  if (url.pathname === "/api/daily-brief") {
    return jsonResponse(dailyBrief(date));
  }
  if (url.pathname === "/api/visual-window-summaries") {
    return jsonResponse({
      summaries: [
        {
          id: Number(date.replaceAll("-", "")),
          windowStart: `${date}T01:00:00Z`,
          windowEnd: `${date}T01:05:00Z`,
          sampledScreenshotIds: [1, 2, 3],
          previousSummaryId: null,
          modelProvider: "minimax",
          modelName: "MiniMax-M3",
          promptVersion: "visual-window-v1",
          summaryText: "数据大屏轨迹",
          continuity: "继续 dashboard 工作",
          primaryActivity: "coding",
          projectHints: ["dashboard"],
          taskIntent: "构建 dashboard 大屏",
          trajectory: [
            {
              minuteMark: 1,
              screenshotId: 1,
              observation: "写 dashboard chart",
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
          confidence: 0.82,
          rawSummaryJson: null,
          createdAt: `${date}T01:05:00Z`,
          error: null,
        },
      ],
    });
  }
  return jsonResponse({});
}

function dailyBrief(date: string) {
  return {
    date,
    status: "complete",
    brief: null,
    hourlyReports: [
      {
        id: 1,
        periodStart: `${date}T01:00:00Z`,
        periodEnd: `${date}T02:00:00Z`,
        generatedAt: `${date}T02:05:00Z`,
        reportKind: "1h",
        modelProvider: "minimax",
        modelName: "MiniMax-M3",
        summaryText: "dashboard report coverage",
        categoryMix: [{ activityCategory: "coding", count: 1 }],
        projectHints: ["dashboard"],
        evidenceCount: 3,
        error: null,
      },
    ],
    fiveHourReports: [
      {
        id: 2,
        periodStart: `${date}T01:00:00Z`,
        periodEnd: `${date}T06:00:00Z`,
        generatedAt: `${date}T06:05:00Z`,
        reportKind: "5h",
        modelProvider: "minimax",
        modelName: "MiniMax-M3",
        summaryText: "Time State Recorder dashboard implementation",
        categoryMix: [{ activityCategory: "coding", count: 1 }],
        projectHints: ["dashboard"],
        evidenceCount: 9,
        error: null,
      },
    ],
    descriptiveStats: {
      date,
      periodStart: `${date}T00:00:00Z`,
      periodEnd: `${date}T23:59:59Z`,
      activeSeconds: 18_000,
      activeHours: 5,
      windowEventCount: 10,
      switchCount: 25,
      distinctAppCount: 2,
      topApps: [{ processName: "Code.exe", activeSeconds: 18_000, share: 1 }],
      categoryMix: [{ activityCategory: "coding", count: 2 }],
      inputChars: 12_000,
      inputEvents: 200,
      screenshotCount: 60,
      highResScreenshotCount: 24,
      visualWindowCount: 12,
      fiveHourReportCount: 1,
      firstActivityAt: `${date}T01:00:00Z`,
      lastActivityAt: `${date}T09:00:00Z`,
    },
    hourlyMetrics: [
      {
        hour: 9,
        startAt: `${date}T09:00:00Z`,
        endAt: `${date}T10:00:00Z`,
        activeSeconds: 1800,
        activeRatio: 0.5,
        windowEventCount: 5,
        switchCount: 2,
        distinctAppCount: 1,
        dominantApp: "Code.exe",
        dominantCategory: "coding",
        inputChars: 500,
        screenshotCount: 6,
        highResScreenshotCount: 2,
        visualWindowCount: 1,
        fiveHourReportIds: [2],
      },
    ],
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

function jsonResponse(body: unknown) {
  return {
    ok: true,
    status: 200,
    statusText: "OK",
    json: async () => body,
  };
}
