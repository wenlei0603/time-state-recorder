import type { ActivityBucket } from "../types";

export const activitySampleBuckets: ActivityBucket[] = [
  {
    id: "sample-activity-1",
    startAt: "2026-05-24T09:00:00Z",
    endAt: "2026-05-24T09:03:00Z",
    bucketSeconds: 180,
    dominantApp: "Code",
    dominantTitle: "Activity review PRD",
    normalizedTitle: "Activity review PRD",
    dominantDurationSeconds: 156,
    switchCount: 1,
    projectId: undefined,
    projectName: "Time State Recorder",
    activityCategory: "coding",
    attentionState: "deep_focus",
    confidence: 0.87,
    evidence: [
      {
        eventId: "sample-raw-1",
        app: "Code",
        title: "Activity review PRD",
        normalizedTitle: "Activity review PRD",
        kind: "active_window",
        startedAt: "2026-05-24T09:00:00Z",
        endedAt: "2026-05-24T09:02:36Z",
        durationSeconds: 156
      },
      {
        eventId: "sample-raw-2",
        app: "Chrome",
        title: "React testing notes",
        normalizedTitle: "React testing notes",
        kind: "active_window",
        startedAt: "2026-05-24T09:02:36Z",
        endedAt: "2026-05-24T09:03:00Z",
        durationSeconds: 24
      }
    ],
    visualSummaryId: undefined
  },
  {
    id: "sample-activity-2",
    startAt: "2026-05-24T09:03:00Z",
    endAt: "2026-05-24T09:06:00Z",
    bucketSeconds: 180,
    dominantApp: "Chrome",
    dominantTitle: "ActivityWatch docs",
    normalizedTitle: "ActivityWatch docs",
    dominantDurationSeconds: 132,
    switchCount: 3,
    projectId: undefined,
    projectName: "Time State Recorder",
    activityCategory: "research",
    attentionState: "light_switching",
    confidence: 0.73,
    evidence: [
      {
        eventId: "sample-raw-3",
        app: "Chrome",
        title: "ActivityWatch docs",
        normalizedTitle: "ActivityWatch docs",
        kind: "active_window",
        startedAt: "2026-05-24T09:03:00Z",
        endedAt: "2026-05-24T09:05:12Z",
        durationSeconds: 132
      },
      {
        eventId: "sample-raw-4",
        app: "WeChat",
        title: "Project coordination",
        normalizedTitle: "Project coordination",
        kind: "active_window",
        startedAt: "2026-05-24T09:05:12Z",
        endedAt: "2026-05-24T09:06:00Z",
        durationSeconds: 48
      }
    ],
    visualSummaryId: undefined
  },
  {
    id: "sample-activity-3",
    startAt: "2026-05-24T09:06:00Z",
    endAt: "2026-05-24T09:09:00Z",
    bucketSeconds: 180,
    dominantApp: "WeChat",
    dominantTitle: "Project coordination",
    normalizedTitle: "Project coordination",
    dominantDurationSeconds: 120,
    switchCount: 2,
    projectId: undefined,
    projectName: undefined,
    activityCategory: "communication",
    attentionState: "steady",
    confidence: 0.67,
    evidence: [
      {
        eventId: "sample-raw-5",
        app: "WeChat",
        title: "Project coordination",
        normalizedTitle: "Project coordination",
        kind: "active_window",
        startedAt: "2026-05-24T09:06:00Z",
        endedAt: "2026-05-24T09:08:00Z",
        durationSeconds: 120
      },
      {
        eventId: "sample-raw-6",
        app: "Code",
        title: "ActivityReview.tsx",
        normalizedTitle: "ActivityReview.tsx",
        kind: "active_window",
        startedAt: "2026-05-24T09:08:00Z",
        endedAt: "2026-05-24T09:09:00Z",
        durationSeconds: 60
      }
    ],
    visualSummaryId: undefined
  }
];
