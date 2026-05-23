import type { InputEvent, InputSummary, TextSegment } from "../types";

type Fetcher = (
  input: string,
) => Promise<Pick<Response, "ok" | "status" | "statusText" | "json">>;

export async function fetchInputEvents(
  fetcher: Fetcher = fetch,
): Promise<InputEvent[]> {
  const response = await fetcher("/api/input-events");
  if (!response.ok) {
    throw new Error(
      `Collector API failed: ${response.status} ${response.statusText}`.trim(),
    );
  }

  const body: unknown = await response.json();
  if (!isRecord(body) || !Array.isArray(body.events)) {
    throw new Error("Collector API returned an invalid input-events response");
  }

  return body.events.map(toInputEvent);
}

export async function fetchInputSummary(
  date: string,
  fetcher: Fetcher = fetch,
): Promise<InputSummary> {
  const response = await fetcher(
    `/api/input-summary?date=${encodeURIComponent(date)}`,
  );
  if (!response.ok) {
    throw new Error(
      `Collector API failed: ${response.status} ${response.statusText}`.trim(),
    );
  }

  const body: unknown = await response.json();
  if (!isRecord(body)) {
    throw new Error(
      "Collector API returned an invalid input-summary response",
    );
  }

  return {
    date: readString(body, "date"),
    totalEvents: readNumber(body, "totalEvents"),
    keydownCount: readNumber(body, "keydownCount"),
    keyupCount: readNumber(body, "keyupCount"),
    segmentCount: readNumber(body, "segmentCount"),
    totalChars: readNumber(body, "totalChars"),
    lastActivity: readOptionalString(body, "lastActivity"),
    topApps: Array.isArray(body.topApps)
      ? body.topApps.map((item: unknown) => {
          if (!isRecord(item)) {
            throw new Error("topApps row is not a record");
          }
          return {
            processName: readString(item, "processName"),
            charCount: readNumber(item, "charCount"),
          };
        })
      : [],
  };
}

export async function fetchTextSegments(
  date: string,
  fetcher: Fetcher = fetch,
): Promise<TextSegment[]> {
  const response = await fetcher(
    `/api/text-segments?date=${encodeURIComponent(date)}`,
  );
  if (!response.ok) {
    throw new Error(
      `Collector API failed: ${response.status} ${response.statusText}`.trim(),
    );
  }

  const body: unknown = await response.json();
  if (!isRecord(body) || !Array.isArray(body.segments)) {
    throw new Error(
      "Collector API returned an invalid text-segments response",
    );
  }

  return body.segments.map(toTextSegment);
}

function toInputEvent(value: unknown): InputEvent {
  if (!isRecord(value)) {
    throw new Error("API returned an invalid input event row");
  }
  return {
    id: readNumber(value, "id"),
    eventTs: readString(value, "eventTs"),
    eventType: readString(value, "eventType") as "keydown" | "keyup",
    vkCode: readNumber(value, "vkCode"),
    scanCode: readNumber(value, "scanCode"),
    character: readOptionalString(value, "character"),
    segmentId: readString(value, "segmentId"),
    foregroundHwnd: readNumber(value, "foregroundHwnd"),
    foregroundPid: readNumber(value, "foregroundPid"),
    processName: readOptionalString(value, "processName"),
    windowTitle: readOptionalString(value, "windowTitle"),
  };
}

function toTextSegment(value: unknown): TextSegment {
  if (!isRecord(value)) {
    throw new Error("API returned an invalid text segment row");
  }
  return {
    id: readString(value, "id"),
    startedAt: readString(value, "startedAt"),
    endedAt: readOptionalString(value, "endedAt"),
    textContent: readString(value, "textContent"),
    keyCount: readNumber(value, "keyCount"),
    backspaceCount: readNumber(value, "backspaceCount"),
    deleteCount: readNumber(value, "deleteCount"),
    foregroundHwnd: readNumber(value, "foregroundHwnd"),
    foregroundPid: readNumber(value, "foregroundPid"),
    processName: readOptionalString(value, "processName"),
    windowTitle: readOptionalString(value, "windowTitle"),
  };
}

function readString(record: Record<string, unknown>, key: string): string {
  const value = record[key];
  if (typeof value !== "string") {
    throw new Error(`API row is missing ${key}`);
  }
  return value;
}

function readOptionalString(
  record: Record<string, unknown>,
  key: string,
): string | undefined {
  const value = record[key];
  if (value === null || value === undefined) {
    return undefined;
  }
  if (typeof value !== "string") {
    throw new Error(`API row has invalid ${key}`);
  }
  return value;
}

function readNumber(record: Record<string, unknown>, key: string): number {
  const value = record[key];
  if (typeof value !== "number") {
    throw new Error(`API row is missing ${key}`);
  }
  return value;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
