import { describe, expect, it } from "vitest";
import { currentCollectorDate } from "./dateQuery";

describe("currentCollectorDate", () => {
  it("uses the user's local calendar day for UTC+8 after midnight", () => {
    const localAfterMidnight = new Date("2026-06-04T16:30:00.000Z");

    expect(currentCollectorDate(localAfterMidnight, 480)).toBe("2026-06-05");
  });
});
