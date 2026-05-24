import { describe, expect, it } from "vitest";
import {
  summarizeByApplication,
  summarizeDurations,
  toDurationSeconds
} from "./statistics";
import type { TimeEvent } from "../types";

const events: TimeEvent[] = [
  {
    id: "evt-1",
    app: "VS Code",
    title: "feature1.ts",
    startedAt: "2026-05-23T09:00:00.000Z",
    endedAt: "2026-05-23T09:10:00.000Z"
  },
  {
    id: "evt-2",
    app: "Browser",
    title: "Docs",
    startedAt: "2026-05-23T09:10:00.000Z",
    endedAt: "2026-05-23T09:25:00.000Z"
  },
  {
    id: "evt-3",
    app: "VS Code",
    title: "README.md",
    startedAt: "2026-05-23T09:25:00.000Z",
    endedAt: "2026-05-23T09:30:00.000Z"
  }
];

describe("toDurationSeconds", () => {
  it("converts valid event intervals into seconds", () => {
    expect(toDurationSeconds(events[0])).toBe(600);
  });

  it("returns 0 for reversed or invalid intervals", () => {
    expect(
      toDurationSeconds({
        ...events[0],
        startedAt: "2026-05-23T09:10:00.000Z",
        endedAt: "2026-05-23T09:00:00.000Z"
      })
    ).toBe(0);
  });

  it("uses explicit duration seconds when an end timestamp is absent", () => {
    expect(
      toDurationSeconds({
        id: "evt-duration",
        app: "VS Code",
        title: "README.md",
        startedAt: "2026-05-23T09:00:00.000Z",
        durationSeconds: 120
      })
    ).toBe(120);
  });
});

describe("summarizeDurations", () => {
  it("returns descriptive statistics for event durations", () => {
    expect(summarizeDurations(events)).toEqual({
      count: 3,
      total: 1800,
      mean: 600,
      median: 600,
      min: 300,
      max: 900,
      q1: 450,
      q3: 750,
      standardDeviation: 244.95
    });
  });

  it("excludes lifecycle intervals from active duration summaries", () => {
    expect(summarizeDurations([...events, lockedInterval()]).total).toBe(1800);
  });
});

describe("summarizeByApplication", () => {
  it("groups feature1 window events by application share", () => {
    expect(summarizeByApplication(events)).toEqual([
      {
        app: "VS Code",
        eventCount: 2,
        totalSeconds: 900,
        averageSeconds: 450,
        share: 0.5
      },
      {
        app: "Browser",
        eventCount: 1,
        totalSeconds: 900,
        averageSeconds: 900,
        share: 0.5
      }
    ]);
  });

  it("excludes lifecycle intervals from active application summaries", () => {
    expect(summarizeByApplication([...events, lockedInterval()]).map((row) => row.app)).not.toContain(
      "System"
    );
  });
});

function lockedInterval(): TimeEvent {
  return {
    id: "lifecycle-10",
    app: "System",
    title: "Locked",
    kind: "lifecycle",
    status: "windows_lock",
    startedAt: "2026-05-23T09:05:00.000Z",
    endedAt: "2026-05-23T09:20:00.000Z",
    durationSeconds: 900
  };
}
