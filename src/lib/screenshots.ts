import type { ScreenshotMeta, ScreenshotSummary } from "../types";

type Fetcher = (input: string) => Promise<Pick<Response, "ok" | "status" | "statusText" | "json">>;

export async function fetchScreenshots(
  date: string,
  fetcher: Fetcher = fetch,
): Promise<ScreenshotMeta[]> {
  const response = await fetcher(`/api/screenshots?date=${encodeURIComponent(date)}`);
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
  const response = await fetcher(
    `/api/screenshot-summary?date=${encodeURIComponent(date)}`,
  );
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
  };
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
