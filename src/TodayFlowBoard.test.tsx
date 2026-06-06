import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { TodayFlowBoard } from "./TodayFlowBoard";
import type {
  CollectorHealth,
  InputSummary,
  ScreenshotSummary,
  TimeEvent,
} from "./types";

const events: TimeEvent[] = [
  {
    id: "code-1",
    app: "Code.exe",
    title: "time-state-recorder TodayFlowBoard.tsx",
    kind: "active_window",
    startedAt: "2026-06-05T01:00:00.000Z",
    endedAt: "2026-06-05T02:15:00.000Z",
  },
  {
    id: "browser-1",
    app: "msedge.exe",
    title: "MiniMax request timing notes",
    kind: "active_window",
    startedAt: "2026-06-05T02:15:00.000Z",
    endedAt: "2026-06-05T02:45:00.000Z",
  },
  {
    id: "lock-1",
    app: "System",
    title: "Locked",
    kind: "lifecycle",
    status: "windows_lock",
    startedAt: "2026-06-05T02:45:00.000Z",
    endedAt: "2026-06-05T03:00:00.000Z",
    durationSeconds: 900,
  },
];

const screenshotSummary: ScreenshotSummary = {
  date: "2026-06-05",
  totalScreenshots: 18,
  hoursCovered: 3,
  topApps: [{ processName: "Code.exe", count: 12 }],
  skippedReasons: [{ reason: "privacy_blocker", count: 2 }],
};

const inputSummary: InputSummary = {
  date: "2026-06-05",
  totalEvents: 120,
  keydownCount: 70,
  keyupCount: 50,
  segmentCount: 4,
  totalChars: 1800,
  lastActivity: "2026-06-05T02:40:00.000Z",
  topApps: [{ processName: "Code.exe", charCount: 1600 }],
};

const health: CollectorHealth = {
  status: "ok",
  startedAt: "2026-06-05T00:00:00.000Z",
  uptimeSeconds: 3600,
  version: "1.4.0",
  windowCollector: { status: "running", errorCount: 0 },
  inputCollector: { status: "running", errorCount: 0 },
  screenshotCollector: { status: "running", errorCount: 0 },
  dbStats: {
    windowEvents: 12,
    lifecycleEvents: 1,
    inputEvents: 120,
    textSegments: 4,
    screenshots: 18,
    highResScreenshots: 6,
    blockerHits: 0,
    imageRetention: {
      retentionDays: 30,
      activeFiles: 18,
      expiredFiles: 0,
      activeBytes: 1024,
      expiredBytes: 0,
      pendingGoogleDriveUpload: false,
    },
  },
};

describe("TodayFlowBoard", () => {
  it("presents a stable human-readable Today overview before raw evidence", () => {
    render(
      <TodayFlowBoard
        events={events}
        screenshotSummary={screenshotSummary}
        inputSummary={inputSummary}
        health={health}
        privacyMode="redacted"
        screenshotsVisible={true}
        visualSummaries={[]}
        onAnalyzeScreenshot={vi.fn()}
        sourceLabel="Live collector"
      />,
    );

    expect(
      screen.getByRole("heading", { name: /today at a glance/i }),
    ).toBeInTheDocument();
    expect(screen.getAllByText(/known work time/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/1h 45m/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/main focus/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText("Code.exe").length).toBeGreaterThan(0);
    expect(
      screen.getByRole("region", { name: /current focus/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("region", { name: /evidence coverage/i }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("region", { name: /readable timeline/i }),
    ).toBeInTheDocument();
    expect(screen.getByText(/redacted evidence/i)).toBeInTheDocument();
  });

  it("shows dates for ranges that cross local calendar days", () => {
    render(
      <TodayFlowBoard
        events={[
          {
            id: "overnight-1",
            app: "Code.exe",
            title: "overnight collector review",
            kind: "active_window",
            startedAt: "2026-06-05T17:40:00",
            endedAt: "2026-06-06T16:02:00",
          },
        ]}
        screenshotSummary={screenshotSummary}
        inputSummary={inputSummary}
        health={health}
        privacyMode="redacted"
        screenshotsVisible={true}
        visualSummaries={[]}
        onAnalyzeScreenshot={vi.fn()}
        sourceLabel="Live collector"
      />,
    );

    expect(
      screen.getAllByText("06/05 17:40 - 06/06 16:02").length,
    ).toBeGreaterThan(0);
    expect(screen.queryByText("17:40 - 16:02")).not.toBeInTheDocument();
  });
});
