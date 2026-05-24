import type { TimeEvent } from "../types";

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

function readString(record: Record<string, unknown>, key: string): string {
  const value = record[key];
  if (typeof value !== "string") {
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

function readOptionalTimeEventKind(
  record: Record<string, unknown>,
  key: string
): TimeEvent["kind"] {
  const value = record[key];
  if (value === null || value === undefined) {
    return undefined;
  }
  if (value !== "active_window" && value !== "lifecycle") {
    throw new Error(`Collector API row has invalid ${key}`);
  }
  return value;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
