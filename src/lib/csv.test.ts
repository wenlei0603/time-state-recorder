import { describe, expect, it } from "vitest";
import { parseTimeEventsCsv } from "./csv";

describe("parseTimeEventsCsv", () => {
  it("parses a headered CSV into time events", () => {
    const csv = [
      "app,title,startedAt,endedAt",
      "VS Code,feature1.ts,2026-05-23T09:00:00.000Z,2026-05-23T09:10:00.000Z",
      "Browser,\"Docs, Search\",2026-05-23T09:10:00.000Z,2026-05-23T09:25:00.000Z"
    ].join("\n");

    expect(parseTimeEventsCsv(csv)).toEqual({
      events: [
        {
          id: "csv-1",
          app: "VS Code",
          title: "feature1.ts",
          startedAt: "2026-05-23T09:00:00.000Z",
          endedAt: "2026-05-23T09:10:00.000Z"
        },
        {
          id: "csv-2",
          app: "Browser",
          title: "Docs, Search",
          startedAt: "2026-05-23T09:10:00.000Z",
          endedAt: "2026-05-23T09:25:00.000Z"
        }
      ],
      errors: []
    });
  });

  it("reports invalid rows without dropping valid rows", () => {
    const csv = [
      "app,title,startedAt,endedAt",
      "VS Code,feature1.ts,2026-05-23T09:00:00.000Z,2026-05-23T09:10:00.000Z",
      "Browser,Missing end,2026-05-23T09:10:00.000Z,"
    ].join("\n");

    expect(parseTimeEventsCsv(csv)).toEqual({
      events: [
        {
          id: "csv-1",
          app: "VS Code",
          title: "feature1.ts",
          startedAt: "2026-05-23T09:00:00.000Z",
          endedAt: "2026-05-23T09:10:00.000Z"
        }
      ],
      errors: ["Row 3: missing endedAt or durationSeconds"]
    });
  });

  it("reports reversed intervals as invalid duration data", () => {
    const csv = [
      "app,title,startedAt,endedAt",
      "VS Code,feature1.ts,2026-05-23T09:10:00.000Z,2026-05-23T09:00:00.000Z"
    ].join("\n");

    expect(parseTimeEventsCsv(csv)).toEqual({
      events: [],
      errors: ["Row 2: endedAt must be after startedAt"]
    });
  });

  it("rejects calendar-invalid ISO timestamps", () => {
    const csv = [
      "app,title,startedAt,endedAt",
      "VS Code,feature1.ts,2026-02-30T09:00:00.000Z,2026-05-23T09:10:00.000Z"
    ].join("\n");

    expect(parseTimeEventsCsv(csv)).toEqual({
      events: [],
      errors: ["Row 2: invalid startedAt"]
    });
  });

  it("accepts recorder-style snake_case fields with explicit duration", () => {
    const csv = [
      "process_name,window_title,started_at,duration_seconds",
      "VS Code,README.md,2026-05-23T09:00:00.000Z,120"
    ].join("\n");

    expect(parseTimeEventsCsv(csv)).toEqual({
      events: [
        {
          id: "csv-1",
          app: "VS Code",
          title: "README.md",
          startedAt: "2026-05-23T09:00:00.000Z",
          durationSeconds: 120
        }
      ],
      errors: []
    });
  });
});
