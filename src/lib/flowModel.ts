import type {
  FlowBucket,
  FlowConfidence,
  FlowEvidence,
  InputSummary,
  PrivacyMode,
  ScreenshotSummary,
  TimeEvent,
  TodayFlowModel,
} from "../types";
import { toDurationSeconds } from "./statistics";

type BuildTodayFlowModelInput = {
  events: TimeEvent[];
  screenshotSummary?: ScreenshotSummary;
  inputSummary?: InputSummary;
  privacyMode?: PrivacyMode;
};

const REDACTED_TITLE = "Hidden in redacted mode";

const UNCERTAIN_LIFECYCLE_STATUSES = new Set([
  "collector_gap",
  "capture_unavailable",
  "session_stop",
  "power_suspend",
]);

export function buildTodayFlowModel({
  events,
  screenshotSummary,
  inputSummary,
  privacyMode = "redacted",
}: BuildTodayFlowModelInput): TodayFlowModel {
  const mode: PrivacyMode = privacyMode === "raw" ? "raw" : "redacted";
  const screenshotCount = screenshotSummary?.totalScreenshots ?? 0;
  const skippedReasons = screenshotSummary?.skippedReasons ?? [];
  const screenshotSkippedCount = skippedReasons.reduce(
    (sum, reason) => sum + reason.count,
    0,
  );
  const orderedEvents = [...events].sort(compareEvents);
  const evidence = orderedEvents.map((event) =>
    toFlowEvidence(event, mode),
  );
  const buckets = evidence.map(toFlowBucket);

  return {
    privacyMode: mode,
    activeSeconds: sumDurations(orderedEvents, isActiveEvent),
    uncertainSeconds: evidence
      .filter((item) => item.confidence === "uncertain")
      .reduce((sum, item) => sum + item.durationSeconds, 0),
    screenshotCount,
    screenshotSkippedCount,
    inputChars: inputSummary?.totalChars ?? 0,
    skippedReasons,
    buckets,
    evidence,
  };
}

function toFlowEvidence(
  event: TimeEvent,
  privacyMode: PrivacyMode,
): FlowEvidence {
  const durationSeconds = toDurationSeconds(event);

  return {
    id: event.id,
    app: event.app,
    title: privacyMode === "redacted" ? REDACTED_TITLE : event.title,
    kind: event.kind,
    status: event.status,
    startedAt: event.startedAt,
    endedAt: event.endedAt,
    durationSeconds,
    confidence: toConfidence(event, durationSeconds),
    screenshotVisible: privacyMode === "raw",
  };
}

function toFlowBucket(evidence: FlowEvidence): FlowBucket {
  return {
    id: evidence.id,
    app: evidence.app,
    title: evidence.title,
    kind: evidence.kind,
    status: evidence.status,
    startedAt: evidence.startedAt,
    endedAt: evidence.endedAt,
    durationSeconds: evidence.durationSeconds,
    confidence: evidence.confidence,
    evidence: [evidence],
  };
}

function toConfidence(
  event: TimeEvent,
  durationSeconds: number,
): FlowConfidence {
  if (event.kind === "lifecycle") {
    return UNCERTAIN_LIFECYCLE_STATUSES.has(event.status ?? "")
      ? "uncertain"
      : "partial";
  }

  return durationSeconds > 0 ? "high" : "partial";
}

function sumDurations(
  events: TimeEvent[],
  predicate: (event: TimeEvent) => boolean,
): number {
  return events.reduce((sum, event) => {
    if (!predicate(event)) return sum;
    return sum + toDurationSeconds(event);
  }, 0);
}

function isActiveEvent(event: TimeEvent): boolean {
  return event.kind === undefined || event.kind === "active_window";
}

function compareEvents(left: TimeEvent, right: TimeEvent): number {
  const leftTime = Date.parse(left.startedAt);
  const rightTime = Date.parse(right.startedAt);

  if (
    Number.isFinite(leftTime) &&
    Number.isFinite(rightTime) &&
    leftTime !== rightTime
  ) {
    return leftTime - rightTime;
  }

  if (left.startedAt !== right.startedAt) {
    return left.startedAt.localeCompare(right.startedAt);
  }

  return left.id.localeCompare(right.id);
}
