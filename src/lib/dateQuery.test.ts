import { describe, expect, it } from "vitest";
import { currentCollectorDate } from "./dateQuery";

describe("currentCollectorDate", () => {
  it("uses the collector UTC day instead of the user's local calendar day", () => {
    const localAfterMidnight = new Date("2026-05-24T16:38:00.000Z");

    expect(currentCollectorDate(localAfterMidnight)).toBe("2026-05-24");
  });
});
