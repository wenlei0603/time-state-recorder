import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SettingsPanel } from "./SettingsPanel";

function configResponse(overrides = {}) {
  return {
    ok: true,
    status: 200,
    statusText: "OK",
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
      ...overrides,
    }),
  };
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("SettingsPanel", () => {
  it("shows API and storage settings from the backend", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(configResponse() as Response);

    render(<SettingsPanel />);

    expect(await screen.findByDisplayValue("data/dev-sample.sqlite3")).toBeInTheDocument();
    expect(screen.getByDisplayValue("https://api.example.test/v1")).toBeInTheDocument();
    expect(screen.getByText("********oken")).toBeInTheDocument();
  });

  it("saves changed API and storage settings", async () => {
    const fetcher = vi
      .spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(configResponse() as Response)
      .mockResolvedValueOnce(
        configResponse({
          restartRequired: true,
          restartReasons: ["database_path"],
        }) as Response,
      );

    render(<SettingsPanel />);

    const databaseInput = await screen.findByLabelText("Database path");
    fireEvent.change(databaseInput, { target: { value: "D:/TSR/dev.sqlite3" } });
    fireEvent.change(screen.getByLabelText("API key"), {
      target: { value: "updated-token" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save settings" }));

    await waitFor(() => {
      expect(fetcher).toHaveBeenCalledWith(
        "/api/config",
        expect.objectContaining({ method: "PATCH" }),
      );
    });
    expect(await screen.findByText(/restart required/i)).toBeInTheDocument();
  });
});
