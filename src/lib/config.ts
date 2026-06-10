import type { AppConfig, AppConfigPatch, AppConfigResponse } from "../types";

type Fetcher = (
  input: string,
  init?: RequestInit,
) => Promise<Pick<Response, "ok" | "status" | "statusText" | "json">>;

export async function fetchAppConfig(
  fetcher: Fetcher = fetch,
): Promise<AppConfigResponse> {
  const response = await fetcher("/api/config");
  if (!response.ok) {
    throw new Error(
      `Collector API failed: ${response.status} ${response.statusText}`.trim(),
    );
  }
  return normalizeConfigResponse(await response.json());
}

export async function saveAppConfig(
  patch: AppConfigPatch,
  fetcher: Fetcher = fetch,
): Promise<AppConfigResponse> {
  const response = await fetcher("/api/config", {
    method: "PATCH",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(patch),
  });
  if (!response.ok) {
    throw new Error(
      `Collector API failed: ${response.status} ${response.statusText}`.trim(),
    );
  }
  return normalizeConfigResponse(await response.json());
}

function normalizeConfigResponse(value: unknown): AppConfigResponse {
  if (!isRecord(value) || !isRecord(value.config)) {
    throw new Error("Collector API returned an invalid config response");
  }
  const config = value.config;
  return {
    config: {
      storage: {
        databasePath: readString(config.storage, "databasePath", "data/local.sqlite3"),
        screenshotDir: readString(config.storage, "screenshotDir", "data/screenshots"),
        highResScreenshotDir: readString(
          config.storage,
          "highResScreenshotDir",
          "data/high-res-screenshots",
        ),
      },
      runtime: {
        apiAddr: readString(config.runtime, "apiAddr", "127.0.0.1:4317"),
        pollMs: readNumber(config.runtime, "pollMs", 1000),
      },
      capture: {
        screenshotIntervalSecs: readNumber(
          config.capture,
          "screenshotIntervalSecs",
          60,
        ),
        highResScreenshotIntervalSecs: readNumber(
          config.capture,
          "highResScreenshotIntervalSecs",
          60,
        ),
        idleThresholdSecs: readNumber(config.capture, "idleThresholdSecs", 120),
      },
      visual: {
        provider: readString(config.visual, "provider", "local"),
        apiKey: readOptionalString(config.visual, "apiKey") ?? null,
        apiKeyMasked: readOptionalString(config.visual, "apiKeyMasked"),
        baseUrl: readOptionalString(config.visual, "baseUrl"),
        model: readString(config.visual, "model", "MiniMax-M3"),
        imageDetail: readString(config.visual, "imageDetail", "high"),
        maxCompletionTokens: readNumber(config.visual, "maxCompletionTokens", 200000),
      },
    },
    restartRequired: readBoolean(value, "restartRequired", false),
    restartReasons: Array.isArray(value.restartReasons)
      ? value.restartReasons.filter((item): item is string => typeof item === "string")
      : [],
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function readString(value: unknown, key: string, fallback: string): string {
  if (!isRecord(value)) return fallback;
  const field = value[key];
  return typeof field === "string" ? field : fallback;
}

function readOptionalString(value: unknown, key: string): string | undefined {
  if (!isRecord(value)) return undefined;
  const field = value[key];
  return typeof field === "string" ? field : undefined;
}

function readNumber(value: unknown, key: string, fallback: number): number {
  if (!isRecord(value)) return fallback;
  const field = value[key];
  return typeof field === "number" && Number.isFinite(field) ? field : fallback;
}

function readBoolean(value: unknown, key: string, fallback: boolean): boolean {
  if (!isRecord(value)) return fallback;
  const field = value[key];
  return typeof field === "boolean" ? field : fallback;
}
