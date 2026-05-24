import type { TimeEvent } from "../types";

export const feature1SampleEvents: TimeEvent[] = [
  {
    id: "feature1-1",
    app: "VS Code",
    title: "time-state-recorder/src/lib/statistics.ts",
    startedAt: "2026-05-23T09:00:00.000Z",
    endedAt: "2026-05-23T09:18:00.000Z"
  },
  {
    id: "feature1-2",
    app: "Browser",
    title: "ActivityWatch architecture",
    startedAt: "2026-05-23T09:18:00.000Z",
    endedAt: "2026-05-23T09:34:00.000Z"
  },
  {
    id: "feature1-3",
    app: "Windows Terminal",
    title: "npm test",
    startedAt: "2026-05-23T09:34:00.000Z",
    endedAt: "2026-05-23T09:42:00.000Z"
  },
  {
    id: "feature1-4",
    app: "VS Code",
    title: "time-state-recorder/README.md",
    startedAt: "2026-05-23T09:42:00.000Z",
    endedAt: "2026-05-23T10:04:00.000Z"
  },
  {
    id: "feature1-5",
    app: "Obsidian",
    title: "Project notes",
    startedAt: "2026-05-23T10:04:00.000Z",
    endedAt: "2026-05-23T10:17:00.000Z"
  },
  {
    id: "feature1-6",
    app: "Browser",
    title: "MDN API integration notes",
    startedAt: "2026-05-23T10:17:00.000Z",
    endedAt: "2026-05-23T10:27:00.000Z"
  },
  {
    id: "feature1-lifecycle-1",
    app: "System",
    title: "Windows locked",
    kind: "lifecycle",
    status: "windows_lock",
    startedAt: "2026-05-23T10:27:00.000Z",
    endedAt: "2026-05-23T10:39:00.000Z",
    durationSeconds: 720
  }
];
