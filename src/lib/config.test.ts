import { describe, expect, it, vi } from "vitest";
import { fetchAppConfig, saveAppConfig } from "./config";

describe("config API client", () => {
  it("loads redacted app config from /api/config", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        config: {
          storage: {
            databasePath: "data/dev-sample.sqlite3",
            screenshotDir: "data/screenshots",
            highResScreenshotDir: "data/high-res-screenshots",
          },
          runtime: { apiAddr: "127.0.0.1:4317", pollMs: 1000 },
          capture: {
            screenshotIntervalSecs: 60,
            highResScreenshotIntervalSecs: 60,
            idleThresholdSecs: 120,
          },
          visual: {
            provider: "minimax",
            apiKey: null,
            apiKeyMasked: "********oken",
            baseUrl: "https://api.example.test/v1",
            model: "MiniMax-M3",
            imageDetail: "high",
            maxCompletionTokens: 120000,
          },
        },
        restartRequired: false,
        restartReasons: [],
      }),
    });

    const response = await fetchAppConfig(fetcher);

    expect(response.config.storage.databasePath).toBe("data/dev-sample.sqlite3");
    expect(response.config.visual.apiKeyMasked).toBe("********oken");
    expect(fetcher).toHaveBeenCalledWith("/api/config");
  });

  it("saves config patches as JSON PATCH-style partial updates", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        config: {
          storage: {
            databasePath: "D:/TSR/dev.sqlite3",
            screenshotDir: "data/screenshots",
            highResScreenshotDir: "data/high-res-screenshots",
          },
          runtime: { apiAddr: "127.0.0.1:4317", pollMs: 1000 },
          capture: {
            screenshotIntervalSecs: 60,
            highResScreenshotIntervalSecs: 60,
            idleThresholdSecs: 120,
          },
          visual: {
            provider: "minimax",
            apiKey: null,
            apiKeyMasked: "********oken",
            baseUrl: "https://api.updated.test/v1",
            model: "MiniMax-M3",
            imageDetail: "high",
            maxCompletionTokens: 120000,
          },
        },
        restartRequired: true,
        restartReasons: ["database_path"],
      }),
    });

    const response = await saveAppConfig(
      {
        storage: { databasePath: "D:/TSR/dev.sqlite3" },
        visual: {
          provider: "minimax",
          apiKey: "updated-token",
          baseUrl: "https://api.updated.test/v1",
          model: "MiniMax-M3",
          imageDetail: "high",
          maxCompletionTokens: 120000,
        },
      },
      fetcher,
    );

    expect(response.restartRequired).toBe(true);
    expect(fetcher).toHaveBeenCalledWith("/api/config", {
      method: "PATCH",
      headers: { "content-type": "application/json" },
      body: expect.stringContaining("updated-token"),
    });
  });
});
