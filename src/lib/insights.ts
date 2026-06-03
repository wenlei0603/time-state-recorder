import type {
  ActivityCategory,
  ActivityCategoryCount,
  AnalysisStatus,
  AnalysisWorkerStatus,
  InsightReport,
  VisualObservation,
} from "../types";

type Fetcher = (input: string) => Promise<Pick<Response, "ok" | "status" | "statusText" | "json">>;

export async function fetchAnalysisStatus(
  fetcher: Fetcher = fetch,
): Promise<AnalysisStatus> {
  const response = await fetcher("/api/analysis-status");
  if (!response.ok) {
    throw new Error(
      `Collector API failed: ${response.status} ${response.statusText}`.trim(),
    );
  }

  const body: unknown = await response.json();
  if (!isRecord(body) || !isRecord(body.visual) || !isRecord(body.report)) {
    throw new Error("Collector API returned an invalid analysis-status response");
  }

  return {
    visual: toWorkerStatus(body.visual),
    report: toWorkerStatus(body.report),
    latestObservation: readOptionalRecord(body, "latestObservation", toVisualObservation),
    latestReport: readOptionalRecord(body, "latestReport", toInsightReport),
  };
}

export async function fetchInsightReports(
  limit = 5,
  fetcher: Fetcher = fetch,
): Promise<InsightReport[]> {
  const response = await fetcher(`/api/insight-reports?limit=${limit}`);
  if (!response.ok) {
    throw new Error(
      `Collector API failed: ${response.status} ${response.statusText}`.trim(),
    );
  }

  const body: unknown = await response.json();
  if (!isRecord(body) || !Array.isArray(body.reports)) {
    throw new Error("Collector API returned an invalid insight-reports response");
  }

  return body.reports.map(toInsightReport);
}

export async function fetchVisualObservations(
  date: string,
  fetcher: Fetcher = fetch,
): Promise<VisualObservation[]> {
  const response = await fetcher(dateQuery("/api/visual-observations", date));
  if (!response.ok) {
    throw new Error(
      `Collector API failed: ${response.status} ${response.statusText}`.trim(),
    );
  }

  const body: unknown = await response.json();
  if (!isRecord(body) || !Array.isArray(body.observations)) {
    throw new Error("Collector API returned an invalid visual-observations response");
  }

  return body.observations.map(toVisualObservation);
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

function toWorkerStatus(value: Record<string, unknown>): AnalysisWorkerStatus {
  return {
    status: readString(value, "status"),
    lastStartedAt: readOptionalString(value, "lastStartedAt"),
    lastFinishedAt: readOptionalString(value, "lastFinishedAt"),
    nextRunAt: readOptionalString(value, "nextRunAt"),
    lastError: readOptionalString(value, "lastError"),
  };
}

function toVisualObservation(value: unknown): VisualObservation {
  if (!isRecord(value)) {
    throw new Error("Collector API returned an invalid visual observation row");
  }

  return {
    id: readNumber(value, "id"),
    highResScreenshotId: readNumber(value, "highResScreenshotId"),
    capturedAt: readString(value, "capturedAt"),
    filePath: readString(value, "filePath"),
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
}

function toInsightReport(value: unknown): InsightReport {
  if (!isRecord(value)) {
    throw new Error("Collector API returned an invalid insight report row");
  }

  return {
    id: readNumber(value, "id"),
    periodStart: readString(value, "periodStart"),
    periodEnd: readString(value, "periodEnd"),
    generatedAt: readString(value, "generatedAt"),
    reportKind: readString(value, "reportKind"),
    modelProvider: readString(value, "modelProvider"),
    modelName: readString(value, "modelName"),
    summaryText: readString(value, "summaryText"),
    categoryMix: readCategoryMix(value, "categoryMix"),
    projectHints: readStringArray(value, "projectHints"),
    evidenceCount: readNumber(value, "evidenceCount"),
    error: readOptionalString(value, "error"),
  };
}

function readOptionalRecord<T>(
  record: Record<string, unknown>,
  key: string,
  mapper: (value: unknown) => T,
): T | undefined {
  const value = record[key];
  if (value === null || value === undefined) {
    return undefined;
  }
  return mapper(value);
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

function readCategoryMix(
  record: Record<string, unknown>,
  key: string,
): ActivityCategoryCount[] {
  const value = record[key];
  if (!Array.isArray(value)) {
    throw new Error(`API row has invalid ${key}`);
  }
  return value.map((item) => {
    if (!isRecord(item)) {
      throw new Error("categoryMix row is not a record");
    }
    return {
      activityCategory: readActivityCategory(item, "activityCategory"),
      count: readNumber(item, "count"),
    };
  });
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

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
