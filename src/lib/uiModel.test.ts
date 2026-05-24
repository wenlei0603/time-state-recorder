import { describe, expect, it } from "vitest";
import type { TextSegment, TimeEvent } from "../types";
import {
  buildDashboardSummary,
  buildHourlyTimelineItems,
  filterTimelineEvents,
  listSegmentApps,
  summarizeInputInsights,
  toVisibleDashboardEvents
} from "./uiModel";

const events: TimeEvent[] = [
  {
    id: "a1",
    app: "Code.exe",
    title: "main.ts",
    kind: "active_window",
    startedAt: "2026-05-24T01:00:00.000Z",
    endedAt: "2026-05-24T01:30:00.000Z"
  },
  {
    id: "l1",
    app: "System",
    title: "Locked",
    kind: "lifecycle",
    status: "windows_lock",
    startedAt: "2026-05-24T01:30:00.000Z",
    endedAt: "2026-05-24T01:45:00.000Z",
    durationSeconds: 900
  },
  {
    id: "a2",
    app: "Browser.exe",
    title: "docs",
    kind: "active_window",
    startedAt: "2026-05-24T01:45:00.000Z",
    endedAt: "2026-05-24T01:55:00.000Z"
  }
];

const segments: TextSegment[] = [
  {
    id: "s1",
    startedAt: "2026-05-24T01:00:00.000Z",
    endedAt: "2026-05-24T01:04:00.000Z",
    textContent: "nihao",
    keyCount: 30,
    backspaceCount: 6,
    deleteCount: 0,
    foregroundHwnd: 1,
    foregroundPid: 1,
    processName: "Code.exe",
    windowTitle: "main.ts"
  },
  {
    id: "s2",
    startedAt: "2026-05-24T01:05:00.000Z",
    endedAt: "2026-05-24T01:05:10.000Z",
    textContent: "",
    keyCount: 0,
    backspaceCount: 0,
    deleteCount: 0,
    foregroundHwnd: 2,
    foregroundPid: 2,
    processName: "Browser.exe",
    windowTitle: "docs"
  }
];

describe("buildDashboardSummary", () => {
  it("separates active and lifecycle time and counts focus blocks", () => {
    const summary = buildDashboardSummary(events, segments);

    expect(summary.activeSeconds).toBe(2400);
    expect(summary.lifecycleSeconds).toBe(900);
    expect(summary.focusBlockCount).toBe(1);
    expect(summary.contextSwitchCount).toBe(1);
    expect(summary.topApp?.app).toBe("Code.exe");
    expect(summary.input.correctionRatio).toBe(0.2);
  });

  it("uses visible layer events for dashboard active and lifecycle metrics", () => {
    const visible = toVisibleDashboardEvents(events, {
      windows: false,
      lifecycle: true,
      input: true,
      screenshots: true
    });
    const summary = buildDashboardSummary(visible, segments);

    expect(summary.activeSeconds).toBe(0);
    expect(summary.lifecycleSeconds).toBe(900);
    expect(summary.contextSwitchCount).toBe(0);
  });
});

describe("filterTimelineEvents", () => {
  it("hides lifecycle rows when the lifecycle layer is disabled", () => {
    const visible = filterTimelineEvents(events, {
      windows: true,
      lifecycle: false,
      input: true,
      screenshots: true
    });

    expect(visible.map((event) => event.id)).toEqual(["a1", "a2"]);
  });

  it("hides active window rows when the windows layer is disabled", () => {
    const visible = filterTimelineEvents(events, {
      windows: false,
      lifecycle: true,
      input: true,
      screenshots: true
    });

    expect(visible.map((event) => event.id)).toEqual(["l1"]);
  });
});

describe("buildHourlyTimelineItems", () => {
  it("aggregates active and lifecycle seconds by local hour", () => {
    const items = buildHourlyTimelineItems(events, 0);

    expect(items).toHaveLength(1);
    expect(items[0]).toMatchObject({
      id: "hour-2026-05-24T01",
      app: "1 hour bucket",
      activeSeconds: 2400,
      lifecycleSeconds: 900
    });
  });

  it("splits cross-hour events into local-hour buckets", () => {
    const items = buildHourlyTimelineItems(
      [
        {
          id: "cross-hour",
          app: "Code.exe",
          title: "cross-hour.ts",
          kind: "active_window",
          startedAt: "2026-05-24T01:50:00.000Z",
          endedAt: "2026-05-24T02:10:00.000Z"
        }
      ],
      480
    );

    expect(items.map((item) => item.id)).toEqual([
      "hour-2026-05-24T09",
      "hour-2026-05-24T10"
    ]);
    expect(items.map((item) => item.activeSeconds)).toEqual([600, 600]);
  });
});

describe("summarizeInputInsights", () => {
  it("computes correction ratio and burst count safely", () => {
    expect(summarizeInputInsights(segments)).toMatchObject({
      totalKeys: 30,
      correctionCount: 6,
      correctionRatio: 0.2,
      burstCount: 1,
      activeAppCount: 2
    });
  });

  it("handles zero-key segments without NaN", () => {
    const summary = summarizeInputInsights([segments[1]]);

    expect(summary.totalKeys).toBe(0);
    expect(summary.correctionRatio).toBe(0);
  });
});

describe("listSegmentApps", () => {
  it("returns stable application filter values", () => {
    expect(listSegmentApps(segments)).toEqual(["Browser.exe", "Code.exe"]);
  });
});
