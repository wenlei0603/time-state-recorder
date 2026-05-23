import type { CsvParseResult, TimeEvent } from "../types";

const FIELD_ALIASES = {
  app: ["app", "process_name", "processName"],
  title: ["title", "window_title", "windowTitle"],
  startedAt: ["startedAt", "started_at"],
  endedAt: ["endedAt", "ended_at"],
  durationSeconds: ["durationSeconds", "duration_seconds", "duration"]
} as const;

export function parseTimeEventsCsv(input: string): CsvParseResult {
  const rows = parseRows(input).filter((row) =>
    row.some((cell) => cell.trim().length > 0)
  );

  if (rows.length === 0) {
    return { events: [], errors: ["CSV is empty"] };
  }

  const headers = rows[0].map((header) => header.trim());
  const missingHeaders = [
    hasAnyHeader(headers, FIELD_ALIASES.app) ? "" : "app",
    hasAnyHeader(headers, FIELD_ALIASES.startedAt) ? "" : "startedAt",
    hasAnyHeader(headers, FIELD_ALIASES.endedAt) ||
    hasAnyHeader(headers, FIELD_ALIASES.durationSeconds)
      ? ""
      : "endedAt"
  ].filter(Boolean);

  if (missingHeaders.length > 0) {
    return {
      events: [],
      errors: [`Missing headers: ${missingHeaders.join(", ")}`]
    };
  }

  const events: TimeEvent[] = [];
  const errors: string[] = [];

  rows.slice(1).forEach((row, rowIndex) => {
    const rowNumber = rowIndex + 2;
    const record = toRecord(headers, row);
    const app = readField(record, FIELD_ALIASES.app).trim();
    const title = readField(record, FIELD_ALIASES.title).trim();
    const startedAt = readField(record, FIELD_ALIASES.startedAt).trim();
    const endedAt = readField(record, FIELD_ALIASES.endedAt).trim();
    const durationRaw = readField(record, FIELD_ALIASES.durationSeconds).trim();
    const missingField = [
      app ? "" : "app",
      startedAt ? "" : "startedAt",
      endedAt || durationRaw ? "" : "endedAt or durationSeconds"
    ].find(Boolean);

    if (missingField) {
      errors.push(`Row ${rowNumber}: missing ${missingField}`);
      return;
    }

    if (!isValidDate(startedAt)) {
      errors.push(`Row ${rowNumber}: invalid startedAt`);
      return;
    }

    if (endedAt && !isValidDate(endedAt)) {
      errors.push(`Row ${rowNumber}: invalid endedAt`);
      return;
    }

    if (endedAt && Date.parse(endedAt) <= Date.parse(startedAt)) {
      errors.push(`Row ${rowNumber}: endedAt must be after startedAt`);
      return;
    }

    const durationSeconds = durationRaw ? Number(durationRaw) : undefined;
    if (
      durationRaw &&
      (!Number.isFinite(durationSeconds) || Number(durationSeconds) <= 0)
    ) {
      errors.push(`Row ${rowNumber}: invalid durationSeconds`);
      return;
    }

    const event: TimeEvent = {
      id: `csv-${events.length + 1}`,
      app,
      title,
      startedAt
    };

    if (endedAt) {
      event.endedAt = endedAt;
    }

    if (durationSeconds) {
      event.durationSeconds = durationSeconds;
    }

    events.push(event);
  });

  return { events, errors };
}

function hasAnyHeader(headers: string[], aliases: readonly string[]): boolean {
  return aliases.some((alias) => headers.includes(alias));
}

function readField(
  record: Record<string, string>,
  aliases: readonly string[]
): string {
  for (const alias of aliases) {
    if (record[alias] !== undefined) {
      return record[alias];
    }
  }

  return "";
}

function parseRows(input: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [];
  let cell = "";
  let quoted = false;

  for (let index = 0; index < input.length; index += 1) {
    const char = input[index];
    const next = input[index + 1];

    if (char === '"' && quoted && next === '"') {
      cell += '"';
      index += 1;
      continue;
    }

    if (char === '"') {
      quoted = !quoted;
      continue;
    }

    if (char === "," && !quoted) {
      row.push(cell);
      cell = "";
      continue;
    }

    if ((char === "\n" || char === "\r") && !quoted) {
      if (char === "\r" && next === "\n") {
        index += 1;
      }
      row.push(cell);
      rows.push(row);
      row = [];
      cell = "";
      continue;
    }

    cell += char;
  }

  row.push(cell);
  rows.push(row);
  return rows;
}

function toRecord(headers: string[], row: string[]): Record<string, string> {
  return headers.reduce<Record<string, string>>((record, header, index) => {
    record[header] = row[index] ?? "";
    return record;
  }, {});
}

function isValidDate(value: string): boolean {
  if (!Number.isFinite(Date.parse(value))) {
    return false;
  }

  const match =
    /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.(\d{1,3}))?(?:Z|[+-]\d{2}:\d{2})?$/.exec(
      value
    );

  if (!match) {
    return true;
  }

  const [, yearRaw, monthRaw, dayRaw, hourRaw, minuteRaw, secondRaw, msRaw] =
    match;
  const year = Number(yearRaw);
  const month = Number(monthRaw);
  const day = Number(dayRaw);
  const hour = Number(hourRaw);
  const minute = Number(minuteRaw);
  const second = Number(secondRaw);
  const ms = Number(msRaw ?? "0");
  const lastDay = new Date(Date.UTC(year, month, 0)).getUTCDate();

  return (
    month >= 1 &&
    month <= 12 &&
    day >= 1 &&
    day <= lastDay &&
    hour >= 0 &&
    hour <= 23 &&
    minute >= 0 &&
    minute <= 59 &&
    second >= 0 &&
    second <= 59 &&
    ms >= 0 &&
    ms <= 999
  );
}
