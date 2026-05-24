import { describe, expect, it } from "vitest";
import { buildTodayFlowModel } from "./flowModel";
import type { InputSummary, ScreenshotSummary, TimeEvent } from "../types";

const events: TimeEvent[] = [
  {
    id: "event-1",
    app: "Code.exe",
    title: "main.rs",
    kind: "active_window",
    startedAt: "2026-05-24T09:00:00.000Z",
    endedAt: "2026-05-24T09:20:00.000Z",
    durationSeconds: 1200,
  },
  {
    id: "gap-1",
    app: "System",
    title: "Collector gap",
    kind: "lifecycle",
    status: "collector_gap",
    startedAt: "2026-05-24T09:20:00.000Z",
    endedAt: "2026-05-24T09:30:00.000Z",
    durationSeconds: 600,
  },
  {
    id: "event-2",
    app: "Browser.exe",
    title: "Task 2 spec",
    kind: "active_window",
    startedAt: "2026-05-24T09:30:00.000Z",
    endedAt: "2026-05-24T10:00:00.000Z",
    durationSeconds: 1800,
  },
];

const screenshotSummary: ScreenshotSummary = {
  date: "2026-05-24",
  totalScreenshots: 12,
  hoursCovered: 1,
  topApps: [],
  skippedReasons: [{ reason: "idle", count: 3 }],
};

const inputSummary: InputSummary = {
  date: "2026-05-24",
  totalEvents: 120,
  keydownCount: 120,
  keyupCount: 0,
  segmentCount: 3,
  totalChars: 120,
  topApps: [],
};

describe("buildTodayFlowModel", () => {
  it("builds deterministic buckets and counters from activity, lifecycle, screenshots, and input", () => {
    const model = buildTodayFlowModel({
      events,
      screenshotSummary,
      inputSummary,
      privacyMode: "raw",
    });

    expect(model.activeSeconds).toBe(3000);
    expect(model.uncertainSeconds).toBe(600);
    expect(model.screenshotCount).toBe(12);
    expect(model.screenshotSkippedCount).toBe(3);
    expect(model.inputChars).toBe(120);
    expect(model.buckets.map((bucket) => bucket.confidence)).toEqual([
      "high",
      "uncertain",
      "high",
    ]);
  });

  it("hides evidence titles and screenshots in redacted mode", () => {
    const model = buildTodayFlowModel({
      events,
      screenshotSummary,
      inputSummary,
      privacyMode: "redacted",
    });

    expect(model.evidence[0].title).toBe("Hidden in redacted mode");
    expect(model.evidence[0].screenshotVisible).toBe(false);
  });

  it("keeps evidence titles and screenshots visible in raw mode", () => {
    const model = buildTodayFlowModel({
      events,
      screenshotSummary,
      inputSummary,
      privacyMode: "raw",
    });

    expect(model.evidence[0].title).toBe("main.rs");
    expect(model.evidence[0].screenshotVisible).toBe(true);
  });

  it("keeps raw evidence visible even when no screenshot summary is loaded", () => {
    const model = buildTodayFlowModel({
      events,
      inputSummary,
      privacyMode: "raw",
    });

    expect(model.screenshotCount).toBe(0);
    expect(model.screenshotSkippedCount).toBe(0);
    expect(model.evidence[0].screenshotVisible).toBe(true);
  });
});
