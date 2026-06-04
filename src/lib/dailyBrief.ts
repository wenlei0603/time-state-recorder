import type {
  ActivityCategory,
  ActivityCategoryCount,
  DailyActivityStats,
  DailyBrief,
  DailyBriefResponse,
  DailyComparison,
  HourlyActivityMetric,
  InsightReport,
} from "../types";

type Fetcher = (input: string) => Promise<Pick<Response, "ok" | "status" | "statusText" | "json">>;

export async function fetchDailyBrief(
  date: string,
  fetcher: Fetcher = fetch,
): Promise<DailyBriefResponse> {
  const response = await fetcher(dateQuery("/api/daily-brief", date));
  if (!response.ok) {
    throw new Error(
      `Collector API failed: ${response.status} ${response.statusText}`.trim(),
    );
  }

  const body: unknown = await response.json();
  if (!isRecord(body)) {
    throw new Error("Collector API returned an invalid daily-brief response");
  }
  return toDailyBriefResponse(body);
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

function toDailyBriefResponse(value: Record<string, unknown>): DailyBriefResponse {
  return {
    date: readString(value, "date"),
    status: readString(value, "status"),
    nextRunAt: readOptionalString(value, "nextRunAt"),
    brief: readOptionalRecord(value, "brief", toDailyBrief),
    fiveHourReports: readArray(value, "fiveHourReports", toInsightReport),
    descriptiveStats: readDailyActivityStats(value, "descriptiveStats"),
    hourlyMetrics: readArray(value, "hourlyMetrics", toHourlyActivityMetric),
    comparison: readDailyComparison(value, "comparison"),
  };
}

function toDailyBrief(value: unknown): DailyBrief {
  if (!isRecord(value)) {
    throw new Error("Collector API returned an invalid daily brief row");
  }
  return {
    id: readNumber(value, "id"),
    date: readString(value, "date"),
    periodStart: readString(value, "periodStart"),
    periodEnd: readString(value, "periodEnd"),
    generatedAt: readString(value, "generatedAt"),
    scheduledForLocal: readString(value, "scheduledForLocal"),
    modelProvider: readString(value, "modelProvider"),
    modelName: readString(value, "modelName"),
    promptVersion: readString(value, "promptVersion"),
    status: readString(value, "status"),
    descriptiveStats: readDailyActivityStats(value, "descriptiveStats"),
    hourlyMetrics: readArray(value, "hourlyMetrics", toHourlyActivityMetric),
    comparison: readDailyComparison(value, "comparison"),
    fiveHourReportIds: readNumberArray(value, "fiveHourReportIds"),
    dailySummaryText: readString(value, "dailySummaryText"),
    actionTrajectory: readString(value, "actionTrajectory"),
    rawSummaryJson: value.rawSummaryJson ?? null,
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
    categoryMix: readArray(value, "categoryMix", toCategoryCount),
    projectHints: readStringArray(value, "projectHints"),
    evidenceCount: readNumber(value, "evidenceCount"),
    error: readOptionalString(value, "error"),
  };
}

function readDailyActivityStats(record: Record<string, unknown>, key: string): DailyActivityStats {
  const value = readRecord(record, key);
  return {
    date: readString(value, "date"),
    periodStart: readString(value, "periodStart"),
    periodEnd: readString(value, "periodEnd"),
    activeSeconds: readNumber(value, "activeSeconds"),
    activeHours: readNumber(value, "activeHours"),
    windowEventCount: readNumber(value, "windowEventCount"),
    switchCount: readNumber(value, "switchCount"),
    distinctAppCount: readNumber(value, "distinctAppCount"),
    topApps: readArray(value, "topApps", (item) => {
      const row = assertRecord(item, "top app");
      return {
        processName: readString(row, "processName"),
        activeSeconds: readNumber(row, "activeSeconds"),
        share: readNumber(row, "share"),
      };
    }),
    categoryMix: readArray(value, "categoryMix", toCategoryCount),
    inputChars: readNumber(value, "inputChars"),
    inputEvents: readNumber(value, "inputEvents"),
    screenshotCount: readNumber(value, "screenshotCount"),
    highResScreenshotCount: readNumber(value, "highResScreenshotCount"),
    visualWindowCount: readNumber(value, "visualWindowCount"),
    fiveHourReportCount: readNumber(value, "fiveHourReportCount"),
    firstActivityAt: readOptionalString(value, "firstActivityAt"),
    lastActivityAt: readOptionalString(value, "lastActivityAt"),
  };
}

function toHourlyActivityMetric(value: unknown): HourlyActivityMetric {
  const row = assertRecord(value, "hourly metric");
  return {
    hour: readNumber(row, "hour"),
    startAt: readString(row, "startAt"),
    endAt: readString(row, "endAt"),
    activeSeconds: readNumber(row, "activeSeconds"),
    activeRatio: readNumber(row, "activeRatio"),
    windowEventCount: readNumber(row, "windowEventCount"),
    switchCount: readNumber(row, "switchCount"),
    distinctAppCount: readNumber(row, "distinctAppCount"),
    dominantApp: readOptionalString(row, "dominantApp"),
    dominantCategory: readActivityCategory(row, "dominantCategory"),
    inputChars: readNumber(row, "inputChars"),
    screenshotCount: readNumber(row, "screenshotCount"),
    highResScreenshotCount: readNumber(row, "highResScreenshotCount"),
    visualWindowCount: readNumber(row, "visualWindowCount"),
    fiveHourReportIds: readNumberArray(row, "fiveHourReportIds"),
  };
}

function readDailyComparison(record: Record<string, unknown>, key: string): DailyComparison {
  const value = readRecord(record, key);
  return {
    baselineDays: readNumber(value, "baselineDays"),
    comparedDates: readStringArray(value, "comparedDates"),
    activeSecondsDelta: readNumber(value, "activeSecondsDelta"),
    switchesPerHourDelta: readNumber(value, "switchesPerHourDelta"),
    inputCharsDelta: readNumber(value, "inputCharsDelta"),
    screenshotCoverageDelta: readNumber(value, "screenshotCoverageDelta"),
    dominantCategoryShift: readOptionalString(value, "dominantCategoryShift"),
    startTimeShiftMinutes: readOptionalNumber(value, "startTimeShiftMinutes"),
    endTimeShiftMinutes: readOptionalNumber(value, "endTimeShiftMinutes"),
    explanation: readString(value, "explanation"),
  };
}

function toCategoryCount(value: unknown): ActivityCategoryCount {
  const row = assertRecord(value, "category count");
  return {
    activityCategory: readActivityCategory(row, "activityCategory"),
    count: readNumber(row, "count"),
  };
}

function readArray<T>(
  record: Record<string, unknown>,
  key: string,
  mapper: (value: unknown) => T,
): T[] {
  const value = record[key];
  if (!Array.isArray(value)) {
    throw new Error(`API row has invalid ${key}`);
  }
  return value.map(mapper);
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

function readRecord(record: Record<string, unknown>, key: string): Record<string, unknown> {
  return assertRecord(record[key], key);
}

function assertRecord(value: unknown, label: string): Record<string, unknown> {
  if (!isRecord(value)) {
    throw new Error(`API row has invalid ${label}`);
  }
  return value;
}

function readString(record: Record<string, unknown>, key: string): string {
  const value = record[key];
  if (typeof value !== "string") {
    throw new Error(`API row is missing ${key}`);
  }
  return value;
}

function readOptionalString(record: Record<string, unknown>, key: string): string | undefined {
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

function readOptionalNumber(record: Record<string, unknown>, key: string): number | undefined {
  const value = record[key];
  if (value === null || value === undefined) {
    return undefined;
  }
  if (typeof value !== "number") {
    throw new Error(`API row has invalid ${key}`);
  }
  return value;
}

function readNumberArray(record: Record<string, unknown>, key: string): number[] {
  const value = record[key];
  if (!Array.isArray(value) || value.some((item) => typeof item !== "number")) {
    throw new Error(`API row has invalid ${key}`);
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

function readActivityCategory(record: Record<string, unknown>, key: string): ActivityCategory {
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
