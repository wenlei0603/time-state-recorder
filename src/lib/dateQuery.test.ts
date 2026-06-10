import { describe, expect, it } from "vitest";
import {
  collectorDateQuery,
  currentCollectorDate,
  OWNER_TZ_OFFSET_MINUTES,
} from "./dateQuery";

describe("currentCollectorDate", () => {
  it("uses the user's local calendar day for UTC+8 after midnight", () => {
    const localAfterMidnight = new Date("2026-06-04T16:30:00.000Z");

    expect(currentCollectorDate(localAfterMidnight, 480)).toBe("2026-06-05");
  });

  it("defaults to the owner Asia/Shanghai calendar day", () => {
    const shanghaiAfterMidnight = new Date("2026-06-08T16:30:00.000Z");

    expect(currentCollectorDate(shanghaiAfterMidnight)).toBe("2026-06-09");
  });
});

describe("collectorDateQuery", () => {
  it("pins report queries to Asia/Shanghai instead of the browser timezone", () => {
    expect(OWNER_TZ_OFFSET_MINUTES).toBe(-480);
    expect(collectorDateQuery("/api/daily-brief", "2026-06-08")).toBe(
      "/api/daily-brief?date=2026-06-08&tzOffsetMinutes=-480",
    );
  });
});
