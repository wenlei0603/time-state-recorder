import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CollectorMonitor } from "./CollectorMonitor";

function healthResponse() {
  return {
    ok: true,
    status: 200,
    statusText: "OK",
    json: async () => ({
      status: "ok",
      startedAt: "2026-06-04T00:00:00Z",
      uptimeSeconds: 120,
      version: "1.2.0",
      windowCollector: { status: "running", errorCount: 0 },
      inputCollector: { status: "running", errorCount: 0 },
      screenshotCollector: { status: "running", errorCount: 0 },
      dbStats: {
        windowEvents: 10,
        lifecycleEvents: 0,
        inputEvents: 0,
        textSegments: 0,
        screenshots: 5,
        highResScreenshots: 2,
        blockerHits: 0,
        imageRetention: {
          retentionDays: 30,
          activeFiles: 7,
          expiredFiles: 3,
          activeBytes: 123456,
          expiredBytes: 45678,
          pendingGoogleDriveUpload: true,
          googleDriveMessage:
            "Local screenshots are temporary for 30 days. Upload older evidence to Google Drive before cleanup.",
        },
      },
    }),
  };
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("CollectorMonitor image retention", () => {
  it("shows the 30 day local retention and Google Drive upload reminder", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(healthResponse() as Response);

    render(<CollectorMonitor />);

    expect(await screen.findByText("Image Retention")).toBeInTheDocument();
    expect(screen.getByText("30 days local")).toBeInTheDocument();
    expect(screen.getByText(/Upload older evidence to Google Drive/i)).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();
  });
});
