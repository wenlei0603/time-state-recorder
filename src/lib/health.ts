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

  return {
    status: readStatus(body, "status"),
    startedAt: readString(body, "startedAt"),
    uptimeSeconds: readNumber(body, "uptimeSeconds"),
    version: readString(body, "version"),
    windowCollector: readSubsystem(body, "windowCollector"),
    inputCollector: readSubsystem(body, "inputCollector"),
    screenshotCollector: readSubsystem(body, "screenshotCollector"),
    dbStats: readDbStats(body, "dbStats"),
  };
}

function readSubsystem(
  record: Record<string, unknown>,
  key: string,
): SubsystemHealth {
  const value = record[key];
  if (!isRecord(value)) {
    throw new Error(`Collector API row is missing ${key}`);
  }
  return {
    status: readSubsystemStatus(value, "status"),
    lastEventAt: readOptionalString(value, "lastEventAt"),
    errorCount: readNumber(value, "errorCount"),
    lastError: readOptionalString(value, "lastError"),
  };
}

function readDbStats(
  record: Record<string, unknown>,
  key: string,
): DbStats {
  const value = record[key];
  if (!isRecord(value)) {
    throw new Error(`Collector API row is missing ${key}`);
  }
  return {
    windowEvents: readNumber(value, "windowEvents"),
    inputEvents: readNumber(value, "inputEvents"),
    textSegments: readNumber(value, "textSegments"),
    screenshots: readNumber(value, "screenshots"),
    blockerHits: readNumber(value, "blockerHits"),
  };
}

function readStatus(
  record: Record<string, unknown>,
  key: string,
): "ok" | "degraded" | "error" {
  const value = record[key];
  if (typeof value !== "string") {
    throw new Error(`Collector API row is missing ${key}`);
  }
  if (value !== "ok" && value !== "degraded" && value !== "error") {
    throw new Error(`Collector API row has invalid ${key}: ${value}`);
  }
  return value;
}

function readSubsystemStatus(
  record: Record<string, unknown>,
  key: string,
): "running" | "error" | "not_started" {
  const value = record[key];
  if (typeof value !== "string") {
    throw new Error(`Collector API row is missing ${key}`);
  }
  if (value !== "running" && value !== "error" && value !== "not_started") {
    throw new Error(`Collector API row has invalid ${key}: ${value}`);
  }
  return value;
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
  key: string,
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

function readNumber(record: Record<string, unknown>, key: string): number {
  const value = record[key];
  if (typeof value !== "number") {
    throw new Error(`Collector API row is missing ${key}`);
  }
  return value;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
