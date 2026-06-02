import type {
  ActivityBucket,
  ActivityBucketsResponse,
  ActivityCategory,
  AttentionState,
  BucketEvidence,
  TimeEvent
} from "../types";

type Fetcher = (input: string) => Promise<Pick<Response, "ok" | "status" | "statusText" | "json">>;

export async function fetchTimeEvents(fetcher: Fetcher = fetch): Promise<TimeEvent[]> {
  const response = await fetcher("/api/time-events");
  if (!response.ok) {
    throw new Error(
      `Collector API failed: ${response.status} ${response.statusText}`.trim()
    );
  }

  const body: unknown = await response.json();
  if (!isRecord(body) || !Array.isArray(body.events)) {
    throw new Error("Collector API returned an invalid time-events response");
  }

  return body.events.map(toTimeEvent);
}

export async function fetchActivityBuckets(
  date: string,
  bucketSeconds = 180,
  fetcher: Fetcher = fetch
): Promise<ActivityBucketsResponse> {
  const params = new URLSearchParams({
    date,
    bucketSeconds: String(bucketSeconds)
  });
  const response = await fetcher(`/api/activity-buckets?${params.toString()}`);
  if (!response.ok) {
    throw new Error(
      `Collector API failed: ${response.status} ${response.statusText}`.trim()
    );
  }

  const body: unknown = await response.json();
  if (!isRecord(body) || !Array.isArray(body.buckets)) {
    throw new Error("Collector API returned an invalid activity-buckets response");
  }

  return {
    date: readString(body, "date"),
    bucketSeconds: readNumber(body, "bucketSeconds"),
    buckets: body.buckets.map(toActivityBucket)
  };
}

function toTimeEvent(value: unknown): TimeEvent {
  if (!isRecord(value)) {
    throw new Error("Collector API returned an invalid event row");
  }

  const event: TimeEvent = {
    id: readString(value, "id"),
    app: readString(value, "app"),
    title: readString(value, "title"),
    startedAt: readString(value, "startedAt"),
    endedAt: readOptionalString(value, "endedAt"),
    durationSeconds: readOptionalNumber(value, "durationSeconds")
  };
  const kind = readOptionalTimeEventKind(value, "kind");
  const status = readOptionalString(value, "status");
  const sessionId = readOptionalString(value, "sessionId");

  if (kind !== undefined) event.kind = kind;
  if (status !== undefined) event.status = status;
  if (sessionId !== undefined) event.sessionId = sessionId;

  return event;
}

function toActivityBucket(value: unknown): ActivityBucket {
  if (!isRecord(value)) {
    throw new Error("Collector API returned an invalid activity bucket row");
  }

  try {
    return {
      id: readString(value, "id"),
      startAt: readString(value, "startAt"),
      endAt: readString(value, "endAt"),
      bucketSeconds: readNumber(value, "bucketSeconds"),
      dominantApp: readString(value, "dominantApp"),
      dominantTitle: readString(value, "dominantTitle"),
      normalizedTitle: readString(value, "normalizedTitle"),
      dominantDurationSeconds: readNumber(value, "dominantDurationSeconds"),
      switchCount: readNumber(value, "switchCount"),
      projectId: readNullableString(value, "projectId"),
      projectName: readNullableString(value, "projectName"),
      activityCategory: readActivityCategory(value, "activityCategory"),
      attentionState: readAttentionState(value, "attentionState"),
      confidence: readNumber(value, "confidence"),
      evidence: readArray(value, "evidence").map(toBucketEvidence),
      visualSummaryId: readNullableNumber(value, "visualSummaryId")
    };
  } catch (error) {
    throw new Error(`Collector API returned an invalid activity bucket row: ${errorMessage(error)}`);
  }
}

function toBucketEvidence(value: unknown): BucketEvidence {
  if (!isRecord(value)) {
    throw new Error("Collector API returned an invalid activity bucket evidence row");
  }

  return {
    eventId: readString(value, "eventId"),
    app: readString(value, "app"),
    title: readString(value, "title"),
    normalizedTitle: readString(value, "normalizedTitle"),
    kind: readTimeEventKind(value, "kind"),
    startedAt: readString(value, "startedAt"),
    endedAt: readString(value, "endedAt"),
    durationSeconds: readNumber(value, "durationSeconds")
  };
}

function readString(record: Record<string, unknown>, key: string): string {
  const value = record[key];
  if (typeof value !== "string") {
    throw new Error(`Collector API row is missing ${key}`);
  }
  return value;
}

function readNumber(record: Record<string, unknown>, key: string): number {
  const value = record[key];
  if (typeof value !== "number") {
    throw new Error(`Collector API row is missing ${key}`);
  }
  return value;
}

function readArray(record: Record<string, unknown>, key: string): unknown[] {
  const value = record[key];
  if (!Array.isArray(value)) {
    throw new Error(`Collector API row is missing ${key}`);
  }
  return value;
}

function readOptionalString(
  record: Record<string, unknown>,
  key: string
): string | undefined {
  const value = record[key];
  if (value === null || value === undefined) {
    return undefined;
  }
  if (typeof value !== "string") {
    throw new Error(`Collector API row has invalid ${key}`);
  }
  return value;
}

function readOptionalNumber(
  record: Record<string, unknown>,
  key: string
): number | undefined {
  const value = record[key];
  if (value === null || value === undefined) {
    return undefined;
  }
  if (typeof value !== "number") {
    throw new Error(`Collector API row has invalid ${key}`);
  }
  return value;
}

function readNullableString(
  record: Record<string, unknown>,
  key: string
): string | undefined {
  const value = record[key];
  if (value === null || value === undefined) {
    return undefined;
  }
  if (typeof value !== "string") {
    throw new Error(`Collector API row has invalid ${key}`);
  }
  return value;
}

function readNullableNumber(
  record: Record<string, unknown>,
  key: string
): number | undefined {
  const value = record[key];
  if (value === null || value === undefined) {
    return undefined;
  }
  if (typeof value !== "number") {
    throw new Error(`Collector API row has invalid ${key}`);
  }
  return value;
}

function readOptionalTimeEventKind(
  record: Record<string, unknown>,
  key: string
): TimeEvent["kind"] {
  const value = record[key];
  if (value === null || value === undefined) {
    return undefined;
  }
  return readTimeEventKind(record, key);
}

function readTimeEventKind(
  record: Record<string, unknown>,
  key: string
): "active_window" | "lifecycle" {
  const value = record[key];
  if (value !== "active_window" && value !== "lifecycle") {
    throw new Error(`Collector API row has invalid ${key}`);
  }
  return value;
}

function readActivityCategory(
  record: Record<string, unknown>,
  key: string
): ActivityCategory {
  const value = record[key];
  if (
    value !== "project_work" &&
    value !== "research" &&
    value !== "writing" &&
    value !== "coding" &&
    value !== "communication" &&
    value !== "meeting" &&
    value !== "admin" &&
    value !== "learning" &&
    value !== "planning" &&
    value !== "loafing" &&
    value !== "personal" &&
    value !== "idle" &&
    value !== "unknown"
  ) {
    throw new Error(`Collector API row has invalid ${key}`);
  }
  return value;
}

function readAttentionState(
  record: Record<string, unknown>,
  key: string
): AttentionState {
  const value = record[key];
  if (
    value !== "deep_focus" &&
    value !== "steady" &&
    value !== "light_switching" &&
    value !== "fragmented" &&
    value !== "away" &&
    value !== "unknown"
  ) {
    throw new Error(`Collector API row has invalid ${key}`);
  }
  return value;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
