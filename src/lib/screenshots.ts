import type {
  ActivityCategory,
  ScreenshotMeta,
  ScreenshotSkippedReasonCount,
  ScreenshotSummary,
  VisualSummary,
} from "../types";

type Fetcher = (input: string) => Promise<Pick<Response, "ok" | "status" | "statusText" | "json">>;
type MutatingFetcher = (
  input: string,
  init?: RequestInit,
) => Promise<Pick<Response, "ok" | "status" | "statusText" | "json">>;

export async function fetchScreenshots(
  date: string,
  fetcher: Fetcher = fetch,
): Promise<ScreenshotMeta[]> {
  const response = await fetcher(dateQuery("/api/screenshots", date));
  if (!response.ok) {
    throw new Error(
      `Collector API failed: ${response.status} ${response.statusText}`.trim(),
    );
  }

  const body: unknown = await response.json();
  if (!isRecord(body) || !Array.isArray(body.screenshots)) {
    throw new Error("Collector API returned an invalid screenshots response");
  }

  return body.screenshots.map(toScreenshotMeta);
}

export async function fetchScreenshotSummary(
  date: string,
  fetcher: Fetcher = fetch,
): Promise<ScreenshotSummary> {
  const response = await fetcher(dateQuery("/api/screenshot-summary", date));
  if (!response.ok) {
    throw new Error(
      `Collector API failed: ${response.status} ${response.statusText}`.trim(),
    );
  }

  const body: unknown = await response.json();
  if (!isRecord(body)) {
    throw new Error("Collector API returned an invalid screenshot-summary response");
  }

  return {
    date: readString(body, "date"),
    totalScreenshots: readNumber(body, "totalScreenshots"),
    hoursCovered: readNumber(body, "hoursCovered"),
    topApps: Array.isArray(body.topApps)
      ? body.topApps.map((item: unknown) => {
          if (!isRecord(item)) {
            throw new Error("topApps row is not a record");
          }
          return {
            processName: readString(item, "processName"),
            count: readNumber(item, "count"),
          };
        })
      : [],
    skippedReasons: readSkippedReasons(body.skippedReasons),
  };
}

export async function fetchVisualSummaries(
  date: string,
  fetcher: Fetcher = fetch,
): Promise<VisualSummary[]> {
  const response = await fetcher(dateQuery("/api/visual-summaries", date));
  if (!response.ok) {
    throw new Error(
      `Collector API failed: ${response.status} ${response.statusText}`.trim(),
    );
  }

  const body: unknown = await response.json();
  if (!isRecord(body) || !Array.isArray(body.summaries)) {
    throw new Error("Collector API returned an invalid visual-summaries response");
  }

  return body.summaries.map(toVisualSummary);
}

export async function analyzeScreenshot(
  screenshotId: number,
  fetcher: MutatingFetcher = fetch,
): Promise<VisualSummary> {
  const response = await fetcher(`/api/screenshots/${screenshotId}/analyze`, {
    method: "POST",
  });
  if (!response.ok) {
    throw new Error(
      `Collector API failed: ${response.status} ${response.statusText}`.trim(),
    );
  }

  const body: unknown = await response.json();
  if (!isRecord(body) || !isRecord(body.summary)) {
    throw new Error("Collector API returned an invalid screenshot analysis response");
  }

  return toVisualSummary(body.summary);
}

function dateQuery(path: string, date: string): string {
  const params = new URLSearchParams({
    date,
    tzOffsetMinutes: String(timezoneOffsetMinutes(date)),
  });
  return `${path}?${params.toString()}`;
}

function timezoneOffsetMinutes(date: string): number {
  const localMidnight = new Date(`${date}T00:00:00`);
  if (!Number.isNaN(localMidnight.getTime())) {
    return localMidnight.getTimezoneOffset();
  }
  return new Date().getTimezoneOffset();
}

function toScreenshotMeta(value: unknown): ScreenshotMeta {
  if (!isRecord(value)) {
    throw new Error("Collector API returned an invalid screenshot row");
  }

  return {
    id: readNumber(value, "id"),
    capturedAt: readString(value, "capturedAt"),
    filePath: readString(value, "filePath"),
    width: readNumber(value, "width"),
    height: readNumber(value, "height"),
    processName: readOptionalString(value, "processName"),
    windowTitle: readOptionalString(value, "windowTitle"),
    captureStatus: readString(value, "captureStatus"),
  };
}

function toVisualSummary(value: unknown): VisualSummary {
  if (!isRecord(value)) {
    throw new Error("Collector API returned an invalid visual summary row");
  }

  try {
    return {
      id: readNumber(value, "id"),
      screenshotId: readNumber(value, "screenshotId"),
      capturedAt: readString(value, "capturedAt"),
      modelProvider: readString(value, "modelProvider"),
      modelName: readString(value, "modelName"),
      promptVersion: readString(value, "promptVersion"),
      summaryText: readString(value, "summaryText"),
      activityCategory: readActivityCategory(value, "activityCategory"),
      projectHints: readStringArray(value, "projectHints"),
      visibleApps: readStringArray(value, "visibleApps"),
      visibleTextHints: readStringArray(value, "visibleTextHints"),
      riskFlags: readStringArray(value, "riskFlags"),
      confidence: readNumber(value, "confidence"),
      createdAt: readString(value, "createdAt"),
      error: readOptionalString(value, "error"),
    };
  } catch (error) {
    throw new Error(`Collector API returned an invalid visual summary row: ${errorMessage(error)}`);
  }
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

function readStringArray(record: Record<string, unknown>, key: string): string[] {
  const value = record[key];
  if (!Array.isArray(value) || value.some((item) => typeof item !== "string")) {
    throw new Error(`API row has invalid ${key}`);
  }
  return value;
}

function readActivityCategory(
  record: Record<string, unknown>,
  key: string,
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
    throw new Error(`API row has invalid ${key}`);
  }
  return value;
}

function readSkippedReasons(value: unknown): ScreenshotSkippedReasonCount[] {
  if (value === null || value === undefined) {
    return [];
  }

  if (!Array.isArray(value)) {
    throw new Error("skippedReasons row is not an array");
  }

  return value.map(toSkippedReasonCount);
}

function toSkippedReasonCount(value: unknown): ScreenshotSkippedReasonCount {
  if (!isRecord(value)) {
    throw new Error("skippedReasons row is not a record");
  }

  const reason = value.reason;
  if (typeof reason !== "string" || reason.length === 0) {
    throw new Error("skippedReasons row has invalid reason");
  }

  const count = value.count;
  if (typeof count !== "number" || !Number.isFinite(count) || count < 0) {
    throw new Error("skippedReasons row has invalid count");
  }

  return { reason, count };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
