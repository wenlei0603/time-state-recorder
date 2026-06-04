import type {
  ActivityCategory,
  ActivityCategoryCount,
  AnalysisStatus,
  AnalysisWorkerStatus,
  DailyActivityStats,
  DailyBrief,
  DailyComparison,
  HourlyActivityMetric,
  InsightReport,
  VisualObservation,
  VisualTrajectoryPoint,
  VisualWindowSummary,
} from "../types";

type Fetcher = (input: string) => Promise<Pick<Response, "ok" | "status" | "statusText" | "json">>;
type InsightReportsQuery = number | {
  date?: string;
  kind?: string;
  limit?: number;
};

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
    latestWindowSummary: readOptionalRecord(
      body,
      "latestWindowSummary",
      toVisualWindowSummary,
    ),
    latestReport: readOptionalRecord(body, "latestReport", toInsightReport),
    daily: readOptionalRecord(body, "daily", toWorkerStatus),
    latestDailyBrief: readOptionalRecord(body, "latestDailyBrief", toDailyBrief),
  };
}

export async function fetchInsightReports(
  query: InsightReportsQuery = 5,
  fetcher: Fetcher = fetch,
): Promise<InsightReport[]> {
  const response = await fetcher(insightReportsPath(query));
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

function insightReportsPath(query: InsightReportsQuery): string {
  if (typeof query === "number") {
    return `/api/insight-reports?limit=${query}`;
  }
  const params = new URLSearchParams();
  if (query.date) {
    params.set("date", query.date);
    params.set("tzOffsetMinutes", String(timezoneOffsetMinutes(query.date)));
  }
  if (query.kind) {
    params.set("kind", query.kind);
  }
  if (query.limit !== undefined) {
    params.set("limit", String(query.limit));
  }
  const text = params.toString();
  return text ? `/api/insight-reports?${text}` : "/api/insight-reports";
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

export async function fetchVisualWindowSummaries(
  date: string,
  fetcher: Fetcher = fetch,
): Promise<VisualWindowSummary[]> {
  const response = await fetcher(dateQuery("/api/visual-window-summaries", date));
  if (!response.ok) {
    throw new Error(
      `Collector API failed: ${response.status} ${response.statusText}`.trim(),
    );
  }

  const body: unknown = await response.json();
  if (!isRecord(body) || !Array.isArray(body.summaries)) {
    throw new Error("Collector API returned an invalid visual-window-summaries response");
  }

  return body.summaries.map(toVisualWindowSummary);
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

function toWorkerStatus(value: unknown): AnalysisWorkerStatus {
  if (!isRecord(value)) {
    throw new Error("Collector API returned an invalid worker status row");
  }
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

function toVisualWindowSummary(value: unknown): VisualWindowSummary {
  if (!isRecord(value)) {
    throw new Error("Collector API returned an invalid visual window summary row");
  }

  return {
    id: readNumber(value, "id"),
    windowStart: readString(value, "windowStart"),
    windowEnd: readString(value, "windowEnd"),
    sampledScreenshotIds: readNumberArray(value, "sampledScreenshotIds"),
    previousSummaryId: readOptionalNumber(value, "previousSummaryId"),
    modelProvider: readString(value, "modelProvider"),
    modelName: readString(value, "modelName"),
    promptVersion: readString(value, "promptVersion"),
    summaryText: readString(value, "summaryText"),
    continuity: readString(value, "continuity"),
    primaryActivity: readActivityCategory(value, "primaryActivity"),
    projectHints: readStringArray(value, "projectHints"),
    taskIntent: readString(value, "taskIntent"),
    trajectory: readTrajectory(value, "trajectory"),
    switchingLevel: readString(value, "switchingLevel"),
    switchingEvidence: readString(value, "switchingEvidence"),
    loafingLevel: readString(value, "loafingLevel"),
    loafingEvidence: readString(value, "loafingEvidence"),
    visibleApps: readStringArray(value, "visibleApps"),
    visibleTextHints: readStringArray(value, "visibleTextHints"),
    riskFlags: readStringArray(value, "riskFlags"),
    confidence: readNumber(value, "confidence"),
    rawSummaryJson: value.rawSummaryJson ?? null,
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
    hourlyMetrics: readHourlyMetrics(value, "hourlyMetrics"),
    comparison: readDailyComparison(value, "comparison"),
    fiveHourReportIds: readNumberArray(value, "fiveHourReportIds"),
    dailySummaryText: readString(value, "dailySummaryText"),
    actionTrajectory: readString(value, "actionTrajectory"),
    rawSummaryJson: value.rawSummaryJson ?? null,
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

function readOptionalNumber(
  record: Record<string, unknown>,
  key: string,
): number | undefined {
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

function readDailyActivityStats(
  record: Record<string, unknown>,
  key: string,
): DailyActivityStats {
  const value = record[key];
  if (!isRecord(value)) {
    throw new Error(`API row has invalid ${key}`);
  }
  return {
    date: readString(value, "date"),
    periodStart: readString(value, "periodStart"),
    periodEnd: readString(value, "periodEnd"),
    activeSeconds: readNumber(value, "activeSeconds"),
    activeHours: readNumber(value, "activeHours"),
    windowEventCount: readNumber(value, "windowEventCount"),
    switchCount: readNumber(value, "switchCount"),
    distinctAppCount: readNumber(value, "distinctAppCount"),
    topApps: readTopApps(value, "topApps"),
    categoryMix: readCategoryMix(value, "categoryMix"),
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

function readTopApps(record: Record<string, unknown>, key: string) {
  const value = record[key];
  if (!Array.isArray(value)) {
    throw new Error(`API row has invalid ${key}`);
  }
  return value.map((item) => {
    if (!isRecord(item)) {
      throw new Error("topApps row is not a record");
    }
    return {
      processName: readString(item, "processName"),
      activeSeconds: readNumber(item, "activeSeconds"),
      share: readNumber(item, "share"),
    };
  });
}

function readHourlyMetrics(
  record: Record<string, unknown>,
  key: string,
): HourlyActivityMetric[] {
  const value = record[key];
  if (!Array.isArray(value)) {
    throw new Error(`API row has invalid ${key}`);
  }
  return value.map((item) => {
    if (!isRecord(item)) {
      throw new Error("hourlyMetrics row is not a record");
    }
    return {
      hour: readNumber(item, "hour"),
      startAt: readString(item, "startAt"),
      endAt: readString(item, "endAt"),
      activeSeconds: readNumber(item, "activeSeconds"),
      activeRatio: readNumber(item, "activeRatio"),
      windowEventCount: readNumber(item, "windowEventCount"),
      switchCount: readNumber(item, "switchCount"),
      distinctAppCount: readNumber(item, "distinctAppCount"),
      dominantApp: readOptionalString(item, "dominantApp"),
      dominantCategory: readActivityCategory(item, "dominantCategory"),
      inputChars: readNumber(item, "inputChars"),
      screenshotCount: readNumber(item, "screenshotCount"),
      highResScreenshotCount: readNumber(item, "highResScreenshotCount"),
      visualWindowCount: readNumber(item, "visualWindowCount"),
      fiveHourReportIds: readNumberArray(item, "fiveHourReportIds"),
    };
  });
}

function readDailyComparison(
  record: Record<string, unknown>,
  key: string,
): DailyComparison {
  const value = record[key];
  if (!isRecord(value)) {
    throw new Error(`API row has invalid ${key}`);
  }
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

function readTrajectory(
  record: Record<string, unknown>,
  key: string,
): VisualTrajectoryPoint[] {
  const value = record[key];
  if (!Array.isArray(value)) {
    throw new Error(`API row has invalid ${key}`);
  }
  return value.map((item) => {
    if (!isRecord(item)) {
      throw new Error("trajectory row is not a record");
    }
    return {
      minuteMark: readNumber(item, "minuteMark"),
      screenshotId: readNumber(item, "screenshotId"),
      observation: readString(item, "observation"),
      activityCategory: readActivityCategory(item, "activityCategory"),
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
