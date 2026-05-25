import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { feature3SampleScreenshots } from "./feature3Sample";

describe("feature3SampleScreenshots", () => {
  it("points to screenshot files that exist in the local prototype data directory", () => {
    for (const screenshot of feature3SampleScreenshots) {
      expect(
        existsSync(resolve("data", "screenshots", screenshot.filePath)),
        screenshot.filePath,
      ).toBe(true);
    }
  });
});
