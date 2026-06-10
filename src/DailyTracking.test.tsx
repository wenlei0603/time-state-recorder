import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { feature3SampleScreenshots, feature3SampleSummary } from "./data/feature3Sample";
import { DailyTracking } from "./DailyTracking";

describe("DailyTracking", () => {
  it("uses placeholders for sample screenshots instead of requesting absent files", () => {
    render(
      <DailyTracking
        date="2026-06-06"
        screenshots={feature3SampleScreenshots}
        summary={feature3SampleSummary}
        sourceMode="sample"
        loading={false}
        error={null}
        screenshotsVisible
        privacyMode="raw"
        visualSummaries={[]}
        onAnalyzeScreenshot={vi.fn()}
        onLoadSample={vi.fn()}
        onLoadLive={vi.fn()}
      />
    );

    expect(screen.getAllByText("Sample screenshot").length).toBeGreaterThan(0);
    expect(
      screen.queryByRole("img", { name: /screenshot at/i })
    ).not.toBeInTheDocument();
  });
});
