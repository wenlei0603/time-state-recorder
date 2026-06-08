export const OWNER_TIME_ZONE = "Asia/Shanghai";
export const OWNER_TZ_OFFSET_MINUTES = -480;
export const OWNER_LOCAL_OFFSET_MINUTES = 480;

export function currentCollectorDate(
  date: Date = new Date(),
  timeZoneOffsetMinutes = OWNER_LOCAL_OFFSET_MINUTES,
): string {
  const localTimestamp = date.getTime() + timeZoneOffsetMinutes * 60 * 1000;
  return new Date(localTimestamp).toISOString().slice(0, 10);
}

export function collectorDateQuery(path: string, date: string): string {
  const params = new URLSearchParams({
    date,
    tzOffsetMinutes: String(OWNER_TZ_OFFSET_MINUTES),
  });
  return `${path}?${params.toString()}`;
}

export function formatOwnerClock(value: string | Date, fallback = "Invalid"): string {
  const date = ownerShiftedDate(value);
  if (!date) {
    return fallback;
  }
  return `${pad2(date.getUTCHours())}:${pad2(date.getUTCMinutes())}`;
}

export function formatOwnerShortDateTime(value: string | Date, fallback = "Invalid"): string {
  const date = ownerShiftedDate(value);
  if (!date) {
    return fallback;
  }
  return `${pad2(date.getUTCMonth() + 1)}/${pad2(date.getUTCDate())} ${formatOwnerClock(value)}`;
}

export function ownerDateKey(value: string | Date): string | undefined {
  const date = ownerShiftedDate(value);
  if (!date) {
    return undefined;
  }
  return [
    date.getUTCFullYear(),
    pad2(date.getUTCMonth() + 1),
    pad2(date.getUTCDate()),
  ].join("-");
}

export function ownerHour(value: string | Date): number | undefined {
  const date = ownerShiftedDate(value);
  return date ? date.getUTCHours() : undefined;
}

export function isSameOwnerDate(left: string | Date, right: string | Date): boolean {
  const leftKey = ownerDateKey(left);
  return leftKey !== undefined && leftKey === ownerDateKey(right);
}

function ownerShiftedDate(value: string | Date): Date | undefined {
  const date = value instanceof Date ? value : new Date(value);
  if (Number.isNaN(date.getTime())) {
    return undefined;
  }
  return new Date(date.getTime() + OWNER_LOCAL_OFFSET_MINUTES * 60 * 1000);
}

function pad2(value: number): string {
  return value.toString().padStart(2, "0");
}
