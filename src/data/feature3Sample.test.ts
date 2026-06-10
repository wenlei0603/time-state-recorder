import { describe, expect, it } from "vitest";
import { feature3SampleScreenshots } from "./feature3Sample";

describe("feature3SampleScreenshots", () => {
  it("uses safe date-scoped screenshot paths", () => {
    for (const screenshot of feature3SampleScreenshots) {
      expect(screenshot.filePath, screenshot.filePath).toMatch(
        /^\d{4}-\d{2}-\d{2}\/[^/\\]+\.jpg$/,
      );
      expect(screenshot.filePath, screenshot.filePath).not.toContain("..");
    }
  });
});
