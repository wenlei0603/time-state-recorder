import { describe, expect, it } from "vitest";
import type { ActivityBucket } from "../types";
import {
  activityCategoryLabel,
  attentionStateLabel,
  formatBucketMinutes,
  summarizeActivityBuckets
} from "./activity";

describe("activity review model", () => {
  it("summarizes activity buckets for review", () => {
    const summary = summarizeActivityBuckets([
      bucket({
        id: "bucket-1",
        activityCategory: "coding",
        attentionState: "deep_focus",
        dominantDurationSeconds: 150,
        switchCount: 1
      }),
      bucket({
        id: "bucket-2",
        activityCategory: "communication",
        attentionState: "light_switching",
        dominantDurationSeconds: 90,
        switchCount: 3
      })
    ]);

    expect(summary.bucketCount).toBe(2);
    expect(summary.totalBucketSeconds).toBe(360);
    expect(summary.dominantSeconds).toBe(240);
    expect(summary.totalSwitches).toBe(4);
    expect(summary.categoryBreakdown).toEqual([
      { category: "coding", label: "Coding", bucketCount: 1, seconds: 180 },
      {
        category: "communication",
        label: "Communication",
        bucketCount: 1,
        seconds: 180
      }
    ]);
    expect(summary.attentionBreakdown).toEqual([
      { state: "deep_focus", label: "Deep focus", bucketCount: 1, seconds: 180 },
      {
        state: "light_switching",
        label: "Light switching",
        bucketCount: 1,
        seconds: 180
      }
    ]);
  });

  it("formats labels and durations", () => {
    expect(activityCategoryLabel("loafing")).toBe("Loafing");
    expect(activityCategoryLabel("unknown")).toBe("Unknown");
    expect(attentionStateLabel("fragmented")).toBe("Fragmented");
    expect(formatBucketMinutes(180)).toBe("3m");
    expect(formatBucketMinutes(90)).toBe("1.5m");
  });
});

function bucket(overrides: Partial<ActivityBucket>): ActivityBucket {
  return {
    id: "bucket",
    startAt: "2026-05-24T10:00:00Z",
    endAt: "2026-05-24T10:03:00Z",
    bucketSeconds: 180,
    dominantApp: "Code",
    dominantTitle: "main.rs",
    normalizedTitle: "main.rs",
    dominantDurationSeconds: 180,
    switchCount: 0,
    projectId: undefined,
    projectName: undefined,
    activityCategory: "coding",
    attentionState: "deep_focus",
    confidence: 1,
    evidence: [],
    visualSummaryId: undefined,
    ...overrides
  };
}
