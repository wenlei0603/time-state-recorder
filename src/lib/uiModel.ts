import type { TextSegment, TimeEvent } from "../types";
import { summarizeByApplication, toDurationSeconds } from "./statistics";

export type UiSourceMode = "sample" | "live";
export type PrivacyMode = "redacted" | "raw";
export type DensityMode = "comfortable" | "compact";
export type TimelineGranularity = "event" | "hour";
export type LayerKey = "windows" | "lifecycle" | "input" | "screenshots";
export type LayerVisibility = Record<LayerKey, boolean>;

export type DashboardSummary = {
  activeSeconds: number;
  lifecycleSeconds: number;
  focusBlockCount: number;
  contextSwitchCount: number;
  topApp?: {
    app: string;
    totalSeconds: number;
  };
  input: InputInsightSummary;
};

export type InputInsightSummary = {
  totalKeys: number;
  totalChars: number;
  correctionCount: number;
  correctionRatio: number;
  burstCount: number;
  activeAppCount: number;
};

export type TimelineItem = {
  id: string;
  app: string;
  title: string;
  startedAt: string;
  endedAt?: string;
  kind: "active_window" | "lifecycle" | "hour";
  status?: string;
  durationSeconds: number;
  activeSeconds: number;
  lifecycleSeconds: number;
  eventCount: number;
};

export const defaultLayerVisibility: LayerVisibility = {
  windows: true,
  lifecycle: true,
  input: true,
  screenshots: true
};

const FOCUS_BLOCK_SECONDS = 25 * 60;
const INPUT_BURST_KEYS = 20;
const INPUT_BURST_SECONDS = 120;

export function buildDashboardSummary(
  events: TimeEvent[],
  segments: TextSegment[]
): DashboardSummary {
  const activeEvents = events.filter(isActiveWindowEvent);
  const lifecycleEvents = events.filter(isLifecycleEvent);
  const appSummary = summarizeByApplication(events);

  return {
    activeSeconds: activeEvents.reduce((total, event) => total + eventDuration(event), 0),
    lifecycleSeconds: lifecycleEvents.reduce(
      (total, event) => total + eventDuration(event),
      0
    ),
    focusBlockCount: activeEvents.filter(
      (event) => eventDuration(event) >= FOCUS_BLOCK_SECONDS
    ).length,
    contextSwitchCount: countContextSwitches(activeEvents),
    topApp: appSummary[0]
      ? {
          app: appSummary[0].app,
          totalSeconds: appSummary[0].totalSeconds
        }
      : undefined,
    input: summarizeInputInsights(segments)
  };
}

export function filterTimelineEvents(
  events: TimeEvent[],
  layers: LayerVisibility
): TimeEvent[] {
  return events.filter((event) => {
    if (isLifecycleEvent(event)) {
      return layers.lifecycle;
    }
    return layers.windows;
  });
}

export function buildTimelineItems(events: TimeEvent[]): TimelineItem[] {
  return [...events]
    .sort((a, b) => a.startedAt.localeCompare(b.startedAt))
    .map((event) => {
      const lifecycle = isLifecycleEvent(event);
      const durationSeconds = eventDuration(event);
      return {
        id: event.id,
        app: event.app,
        title: event.title,
        startedAt: event.startedAt,
        endedAt: event.endedAt,
        kind: lifecycle ? "lifecycle" : "active_window",
        status: event.status,
        durationSeconds,
        activeSeconds: lifecycle ? 0 : durationSeconds,
        lifecycleSeconds: lifecycle ? durationSeconds : 0,
        eventCount: 1
      };
    });
}

export function buildHourlyTimelineItems(events: TimeEvent[]): TimelineItem[] {
  const buckets = new Map<string, TimelineItem>();

  for (const item of buildTimelineItems(events)) {
    const hourKey = item.startedAt.slice(0, 13);
    const existing = buckets.get(hourKey);
    if (existing) {
      existing.durationSeconds += item.durationSeconds;
      existing.activeSeconds += item.activeSeconds;
      existing.lifecycleSeconds += item.lifecycleSeconds;
      existing.eventCount += 1;
      existing.endedAt = item.endedAt ?? existing.endedAt;
      continue;
    }

    buckets.set(hourKey, {
      id: `hour-${hourKey}`,
      app: "1 hour bucket",
      title: hourKey.replace("T", " "),
      startedAt: `${hourKey}:00:00.000Z`,
      endedAt: item.endedAt,
      kind: "hour",
      durationSeconds: item.durationSeconds,
      activeSeconds: item.activeSeconds,
      lifecycleSeconds: item.lifecycleSeconds,
      eventCount: 1
    });
  }

  return [...buckets.values()].sort((a, b) => a.startedAt.localeCompare(b.startedAt));
}

export function summarizeInputInsights(
  segments: TextSegment[]
): InputInsightSummary {
  const totalKeys = segments.reduce((total, segment) => total + segment.keyCount, 0);
  const totalChars = segments.reduce(
    (total, segment) => total + segment.textContent.length,
    0
  );
  const correctionCount = segments.reduce(
    (total, segment) => total + segment.backspaceCount + segment.deleteCount,
    0
  );
  const activeApps = new Set(
    segments
      .map((segment) => segment.processName)
      .filter((name): name is string => Boolean(name))
  );

  return {
    totalKeys,
    totalChars,
    correctionCount,
    correctionRatio: totalKeys === 0 ? 0 : roundTo(correctionCount / totalKeys, 2),
    burstCount: segments.filter(isInputBurst).length,
    activeAppCount: activeApps.size
  };
}

export function listSegmentApps(segments: TextSegment[]): string[] {
  return [
    ...new Set(
      segments
        .map((segment) => segment.processName)
        .filter((name): name is string => Boolean(name))
    )
  ].sort((a, b) => a.localeCompare(b));
}

export function formatDuration(value: number): string {
  if (!Number.isFinite(value) || value <= 0) {
    return "0s";
  }

  const rounded = Math.round(value);
  const hours = Math.floor(rounded / 3600);
  const minutes = Math.floor((rounded % 3600) / 60);
  const seconds = rounded % 60;

  if (hours > 0) {
    return `${hours}h ${minutes.toString().padStart(2, "0")}m`;
  }
  if (minutes > 0) {
    if (seconds === 0) {
      return `${minutes}m`;
    }
    return `${minutes}m ${seconds.toString().padStart(2, "0")}s`;
  }
  return `${seconds}s`;
}

function isLifecycleEvent(event: TimeEvent): boolean {
  return event.kind === "lifecycle";
}

function isActiveWindowEvent(event: TimeEvent): boolean {
  return !isLifecycleEvent(event);
}

function eventDuration(event: TimeEvent): number {
  return toDurationSeconds(event);
}

function countContextSwitches(events: TimeEvent[]): number {
  let count = 0;
  let previousApp: string | undefined;

  for (const event of [...events].sort((a, b) => a.startedAt.localeCompare(b.startedAt))) {
    if (previousApp !== undefined && previousApp !== event.app) {
      count += 1;
    }
    previousApp = event.app;
  }

  return count;
}

function isInputBurst(segment: TextSegment): boolean {
  return (
    segment.keyCount >= INPUT_BURST_KEYS ||
    segmentDurationSeconds(segment) >= INPUT_BURST_SECONDS
  );
}

function segmentDurationSeconds(segment: TextSegment): number {
  if (!segment.endedAt) {
    return 0;
  }

  const start = Date.parse(segment.startedAt);
  const end = Date.parse(segment.endedAt);
  if (!Number.isFinite(start) || !Number.isFinite(end) || end <= start) {
    return 0;
  }
  return (end - start) / 1000;
}

function roundTo(value: number, digits: number): number {
  const factor = 10 ** digits;
  return Math.round(value * factor) / factor;
}
