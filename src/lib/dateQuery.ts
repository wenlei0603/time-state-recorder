export function currentCollectorDate(
  date: Date = new Date(),
  timeZoneOffsetMinutes = -date.getTimezoneOffset()
): string {
  const localTimestamp = date.getTime() + timeZoneOffsetMinutes * 60 * 1000;
  return new Date(localTimestamp).toISOString().slice(0, 10);
}
