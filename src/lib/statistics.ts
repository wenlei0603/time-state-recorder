import type { ApplicationSummary, DurationSummary, TimeEvent } from "../types";

const EMPTY_SUMMARY: DurationSummary = {
  count: 0,
  total: 0,
  mean: 0,
  median: 0,
  min: 0,
  max: 0,
  q1: 0,
  q3: 0,
  standardDeviation: 0
};

export function toDurationSeconds(event: TimeEvent): number {
  if (
    typeof event.durationSeconds === "number" &&
    Number.isFinite(event.durationSeconds) &&
    event.durationSeconds > 0
  ) {
    return Math.round(event.durationSeconds);
  }

  const start = Date.parse(event.startedAt);
  const end = Date.parse(event.endedAt ?? "");

  if (!Number.isFinite(start) || !Number.isFinite(end) || end <= start) {
    return 0;
  }

  return Math.round((end - start) / 1000);
}

export function summarizeDurations(events: TimeEvent[]): DurationSummary {
  const values = events
    .filter(isActiveTimeEvent)
    .map(toDurationSeconds)
    .filter((duration) => duration > 0)
    .sort((a, b) => a - b);

  if (values.length === 0) {
    return EMPTY_SUMMARY;
  }

  const total = values.reduce((sum, value) => sum + value, 0);
  const mean = total / values.length;
  const variance =
    values.reduce((sum, value) => sum + (value - mean) ** 2, 0) / values.length;

  return {
    count: values.length,
    total: round(total),
    mean: round(mean),
    median: round(percentile(values, 0.5)),
    min: values[0],
    max: values[values.length - 1],
    q1: round(percentile(values, 0.25)),
    q3: round(percentile(values, 0.75)),
    standardDeviation: round(Math.sqrt(variance))
  };
}

export function summarizeByApplication(events: TimeEvent[]): ApplicationSummary[] {
  const grouped = new Map<string, { count: number; total: number }>();

  for (const event of events) {
    if (!isActiveTimeEvent(event)) {
      continue;
    }

    const duration = toDurationSeconds(event);
    if (duration <= 0) {
      continue;
    }

    const current = grouped.get(event.app) ?? { count: 0, total: 0 };
    grouped.set(event.app, {
      count: current.count + 1,
      total: current.total + duration
    });
  }

  const grandTotal = Array.from(grouped.values()).reduce(
    (sum, group) => sum + group.total,
    0
  );

  return Array.from(grouped.entries())
    .map(([app, group]) => ({
      app,
      eventCount: group.count,
      totalSeconds: group.total,
      averageSeconds: round(group.total / group.count),
      share: grandTotal === 0 ? 0 : round(group.total / grandTotal, 4)
    }))
    .sort(
      (a, b) =>
        b.totalSeconds - a.totalSeconds ||
        b.eventCount - a.eventCount ||
        a.app.localeCompare(b.app)
    );
}

function isActiveTimeEvent(event: TimeEvent): boolean {
  return event.kind === undefined || event.kind === "active_window";
}

function percentile(sortedValues: number[], ratio: number): number {
  if (sortedValues.length === 1) {
    return sortedValues[0];
  }

  const index = (sortedValues.length - 1) * ratio;
  const lower = Math.floor(index);
  const upper = Math.ceil(index);
  const weight = index - lower;

  return sortedValues[lower] * (1 - weight) + sortedValues[upper] * weight;
}

function round(value: number, digits = 2): number {
  const factor = 10 ** digits;
  return Math.round((value + Number.EPSILON) * factor) / factor;
}
