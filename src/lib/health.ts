import type { CollectorHealth, DbStats, SubsystemHealth } from "../types";

type Fetcher = (input: string) => Promise<Pick<Response, "ok" | "status" | "statusText" | "json">>;

export async function fetchCollectorHealth(
  fetcher: Fetcher = fetch,
): Promise<CollectorHealth> {
  const response = await fetcher("/api/health");
  if (!response.ok) {
    throw new Error(
      `Collector API failed: ${response.status} ${response.statusText}`.trim(),
    );
  }

  const body: unknown = await response.json();
  if (!isRecord(body)) {
    throw new Error("Collector API returned an invalid health response");
  }

  const subsystems: SubsystemHealth = { status: "not_started", errorCount: 0 };

  return {
    status: readStatus(body, "status", "error"),
    startedAt: readString(body, "startedAt", new Date().toISOString()),
    uptimeSeconds: readNumber(body, "uptimeSeconds", 0),
    version: readString(body, "version", "unknown"),
    windowCollector: readSubsystem(body, "windowCollector", subsystems),
    inputCollector: readSubsystem(body, "inputCollector", subsystems),
    screenshotCollector: readSubsystem(body, "screenshotCollector", subsystems),
    dbStats: readDbStats(body, "dbStats"),
  };
}

function readSubsystem(
  record: Record<string, unknown>,
  key: string,
  fallback: SubsystemHealth,
): SubsystemHealth {
  const value = record[key];
  if (!isRecord(value)) return fallback;
  return {
    status: readSubsystemStatus(value, "status"),
    lastEventAt: readOptionalString(value, "lastEventAt"),
    errorCount: readNumber(value, "errorCount", 0),
    lastError: readOptionalString(value, "lastError"),
    mode: readOptionalString(value, "mode"),
    lastCaptureStatus: readOptionalString(value, "lastCaptureStatus"),
    lastSkipReason: readOptionalString(value, "lastSkipReason"),
  };
}

function readDbStats(
  record: Record<string, unknown>,
  key: string,
): DbStats {
  const value = record[key];
  if (!isRecord(value)) {
    return {
      windowEvents: 0,
      lifecycleEvents: 0,
      inputEvents: 0,
      textSegments: 0,
      screenshots: 0,
      highResScreenshots: 0,
      blockerHits: 0,
      imageRetention: defaultImageRetention(),
    };
  }
  return {
    windowEvents: readNumber(value, "windowEvents", 0),
    lifecycleEvents: readNumber(value, "lifecycleEvents", 0),
    inputEvents: readNumber(value, "inputEvents", 0),
    textSegments: readNumber(value, "textSegments", 0),
    screenshots: readNumber(value, "screenshots", 0),
    highResScreenshots: readNumber(value, "highResScreenshots", 0),
    blockerHits: readNumber(value, "blockerHits", 0),
    imageRetention: readImageRetention(value, "imageRetention"),
  };
}

function readImageRetention(
  record: Record<string, unknown>,
  key: string,
): DbStats["imageRetention"] {
  const value = record[key];
  if (!isRecord(value)) return defaultImageRetention();
  return {
    retentionDays: readNumber(value, "retentionDays", 30),
    activeFiles: readNumber(value, "activeFiles", 0),
    expiredFiles: readNumber(value, "expiredFiles", 0),
    activeBytes: readNumber(value, "activeBytes", 0),
    expiredBytes: readNumber(value, "expiredBytes", 0),
    pendingGoogleDriveUpload: readBoolean(value, "pendingGoogleDriveUpload", false),
    googleDriveMessage: readOptionalString(value, "googleDriveMessage"),
  };
}

function defaultImageRetention(): DbStats["imageRetention"] {
  return {
    retentionDays: 30,
    activeFiles: 0,
    expiredFiles: 0,
    activeBytes: 0,
    expiredBytes: 0,
    pendingGoogleDriveUpload: false,
  };
}

function readStatus(
  record: Record<string, unknown>,
  key: string,
  fallback: "ok" | "degraded" | "error",
): "ok" | "degraded" | "error" {
  const value = record[key];
  if (typeof value !== "string") return fallback;
  if (value !== "ok" && value !== "degraded" && value !== "error") return fallback;
  return value;
}

function readSubsystemStatus(
  record: Record<string, unknown>,
  key: string,
): "running" | "error" | "not_started" {
  const value = record[key];
  if (typeof value !== "string") return "not_started";
  if (value !== "running" && value !== "error" && value !== "not_started") return "not_started";
  return value;
}

function readString(
  record: Record<string, unknown>,
  key: string,
  fallback: string,
): string {
  const value = record[key];
  if (typeof value !== "string") return fallback;
  return value;
}

function readOptionalString(
  record: Record<string, unknown>,
  key: string,
): string | undefined {
  const value = record[key];
  if (value === null || value === undefined) return undefined;
  if (typeof value !== "string") return undefined;
  return value;
}

function readNumber(
  record: Record<string, unknown>,
  key: string,
  fallback: number,
): number {
  const value = record[key];
  if (typeof value !== "number") return fallback;
  return value;
}

function readBoolean(
  record: Record<string, unknown>,
  key: string,
  fallback: boolean,
): boolean {
  const value = record[key];
  if (typeof value !== "boolean") return fallback;
  return value;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
