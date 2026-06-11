import type {
  ActivityCategory,
  DailyBriefResponse,
  HourlyActivityMetric,
  InsightReport,
  VisualWindowSummary,
} from "../types";

export type DataScreenPeriod = "day" | "week" | "month";

export type RankedSignal = {
  label: string;
  count: number;
};

export type DataScreenKpi = {
  label: string;
  value: string;
  detail: string;
};

export type DailyTrendPoint = {
  date: string;
  activeHours: number;
  switchesPerHour: number;
  inputChars: number;
  reportCount: number;
  visualWindowCount: number;
};

export type CategorySharePoint = {
  category: ActivityCategory;
  count: number;
};

export type TrajectoryPoint = {
  id: string;
  date: string;
  startAt: string;
  endAt: string;
  intent: string;
  primaryActivity: ActivityCategory;
  projectHints: string[];
  switchingLevel: string;
  loafingLevel: string;
  confidence: number;
};

export type DataScreenModel = {
  anchorDate: string;
  period: DataScreenPeriod;
  dateRange: string[];
  availableDayCount: number;
  kpis: {
    activeHours: DataScreenKpi;
    switchesPerHour: DataScreenKpi;
    inputChars: DataScreenKpi;
    visualCoverage: DataScreenKpi;
    reportCoverage: DataScreenKpi;
  };
  dailyTrend: DailyTrendPoint[];
  hourlyHeatmap: HourlyActivityMetric[];
  categoryShare: CategorySharePoint[];
  appRank: RankedSignal[];
  projectRank: RankedSignal[];
  trajectory: TrajectoryPoint[];
  textSignals: {
    topTerms: RankedSignal[];
    uncertainLines: string[];
  };
};

type BuildDataScreenModelInput = {
  anchorDate: string;
  period: DataScreenPeriod;
  days: DailyBriefResponse[];
  visualWindowsByDate: Record<string, VisualWindowSummary[]>;
};

const STOPWORDS = new Set([
  "and",
  "the",
  "with",
  "from",
  "this",
  "that",
  "into",
  "active",
  "hour",
  "hours",
  "time",
  "state",
  "recorder",
  "implementation",
  "planning",
  "charts",
  "这是",
  "包含",
  "实现",
]);

const UNCERTAINTY_PATTERN = /(不确定|可能|风险|缺失|失败|error|卡住|blocked|uncertain|risk)/i;

export function buildDataScreenModel({
  anchorDate,
  period,
  days,
  visualWindowsByDate,
}: BuildDataScreenModelInput): DataScreenModel {
  const dateRange = buildDateRange(anchorDate, period);
  const dayByDate = new Map(days.map((day) => [day.date, day]));
  const reports = days.flatMap((day) => [...day.hourlyReports, ...day.fiveHourReports]);
  const visualWindows = dateRange.flatMap((date) => visualWindowsByDate[date] ?? []);
  const activeSeconds = days.reduce(
    (total, day) => total + day.descriptiveStats.activeSeconds,
    0,
  );
  const switchCount = days.reduce(
    (total, day) => total + day.descriptiveStats.switchCount,
    0,
  );
  const inputChars = days.reduce(
    (total, day) => total + day.descriptiveStats.inputChars,
    0,
  );
  const visualWindowCount = days.reduce(
    (total, day) => total + day.descriptiveStats.visualWindowCount,
    0,
  );
  const reportCount = reports.length;
  const activeHours = activeSeconds / 3600;

  return {
    anchorDate,
    period,
    dateRange,
    availableDayCount: days.length,
    kpis: {
      activeHours: {
        label: "Active Hours",
        value: formatHours(activeHours),
        detail: `${days.length}/${dateRange.length} days loaded`,
      },
      switchesPerHour: {
        label: "Switches / Hour",
        value: activeHours > 0 ? (switchCount / activeHours).toFixed(1) : "0.0",
        detail: `${switchCount.toLocaleString()} switches`,
      },
      inputChars: {
        label: "Input Chars",
        value: inputChars.toLocaleString(),
        detail: "clean input summary",
      },
      visualCoverage: {
        label: "5min Windows",
        value: Math.max(visualWindowCount, visualWindows.length).toLocaleString(),
        detail: "trajectory windows",
      },
      reportCoverage: {
        label: "1h / 5h Reports",
        value: reportCount.toLocaleString(),
        detail: `${sumReports(days, "1h")} hourly / ${sumReports(days, "5h")} 5h`,
      },
    },
    dailyTrend: dateRange.map((date) => toDailyTrendPoint(date, dayByDate.get(date))),
    hourlyHeatmap: buildHourlyHeatmap(dateRange, dayByDate),
    categoryShare: rankCategoryShare(days),
    appRank: rankApps(days),
    projectRank: rankProjects(days, visualWindowsByDate, dateRange),
    trajectory: visualWindows
      .slice()
      .sort((left, right) => left.windowStart.localeCompare(right.windowStart))
      .map((summary) => ({
        id: `window-${summary.id}`,
        date: summary.windowStart.slice(0, 10),
        startAt: summary.windowStart,
        endAt: summary.windowEnd,
        intent: summary.taskIntent || summary.summaryText,
        primaryActivity: summary.primaryActivity,
        projectHints: summary.projectHints,
        switchingLevel: summary.switchingLevel,
        loafingLevel: summary.loafingLevel,
        confidence: summary.confidence,
      })),
    textSignals: {
      topTerms: extractReportTerms(reports),
      uncertainLines: extractUncertainLines(reports),
    },
  };
}

export function buildDateRange(anchorDate: string, period: DataScreenPeriod): string[] {
  const anchor = parseDate(anchorDate);
  if (!anchor) {
    return [anchorDate];
  }

  if (period === "day") {
    return [formatDate(anchor)];
  }

  if (period === "week") {
    const day = anchor.getUTCDay();
    const mondayOffset = day === 0 ? -6 : 1 - day;
    const monday = addDays(anchor, mondayOffset);
    return Array.from({ length: 7 }, (_, index) => formatDate(addDays(monday, index)));
  }

  const first = new Date(Date.UTC(anchor.getUTCFullYear(), anchor.getUTCMonth(), 1));
  const nextMonth = new Date(Date.UTC(anchor.getUTCFullYear(), anchor.getUTCMonth() + 1, 1));
  const dayCount = Math.round((nextMonth.getTime() - first.getTime()) / 86_400_000);
  return Array.from({ length: dayCount }, (_, index) => formatDate(addDays(first, index)));
}

export function extractReportTerms(reports: InsightReport[], limit = 12): RankedSignal[] {
  const counts = new Map<string, number>();
  for (const report of reports) {
    for (const term of reportTerms(report.summaryText)) {
      counts.set(term, (counts.get(term) ?? 0) + 1);
    }
  }
  return rankCounts(counts).slice(0, limit);
}

function buildHourlyHeatmap(
  dateRange: string[],
  dayByDate: Map<string, DailyBriefResponse>,
): HourlyActivityMetric[] {
  const totals = new Map<number, HourlyActivityMetric>();

  for (const date of dateRange) {
    const day = dayByDate.get(date);
    for (const metric of day?.hourlyMetrics ?? []) {
      const current = totals.get(metric.hour);
      if (!current) {
        totals.set(metric.hour, { ...metric });
        continue;
      }
      current.activeSeconds += metric.activeSeconds;
      current.activeRatio = Math.max(current.activeRatio, metric.activeRatio);
      current.windowEventCount += metric.windowEventCount;
      current.switchCount += metric.switchCount;
      current.distinctAppCount = Math.max(current.distinctAppCount, metric.distinctAppCount);
      current.inputChars += metric.inputChars;
      current.screenshotCount += metric.screenshotCount;
      current.highResScreenshotCount += metric.highResScreenshotCount;
      current.visualWindowCount += metric.visualWindowCount;
      current.fiveHourReportIds = [
        ...new Set([...current.fiveHourReportIds, ...metric.fiveHourReportIds]),
      ];
    }
  }

  return Array.from({ length: 24 }, (_, hour) => {
    const metric = totals.get(hour);
    return (
      metric ?? {
        hour,
        startAt: "",
        endAt: "",
        activeSeconds: 0,
        activeRatio: 0,
        windowEventCount: 0,
        switchCount: 0,
        distinctAppCount: 0,
        dominantCategory: "unknown",
        inputChars: 0,
        screenshotCount: 0,
        highResScreenshotCount: 0,
        visualWindowCount: 0,
        fiveHourReportIds: [],
      }
    );
  });
}

function toDailyTrendPoint(date: string, day?: DailyBriefResponse): DailyTrendPoint {
  const stats = day?.descriptiveStats;
  const activeSeconds = stats?.activeSeconds ?? 0;
  const activeHours = activeSeconds / 3600;
  return {
    date,
    activeHours,
    switchesPerHour: activeHours > 0 ? (stats?.switchCount ?? 0) / activeHours : 0,
    inputChars: stats?.inputChars ?? 0,
    reportCount: day ? day.hourlyReports.length + day.fiveHourReports.length : 0,
    visualWindowCount: stats?.visualWindowCount ?? 0,
  };
}

function rankCategoryShare(days: DailyBriefResponse[]): CategorySharePoint[] {
  const counts = new Map<ActivityCategory, number>();
  for (const day of days) {
    for (const item of day.descriptiveStats.categoryMix) {
      counts.set(item.activityCategory, (counts.get(item.activityCategory) ?? 0) + item.count);
    }
  }
  return [...counts.entries()]
    .map(([category, count]) => ({ category, count }))
    .sort((left, right) => right.count - left.count || left.category.localeCompare(right.category));
}

function rankApps(days: DailyBriefResponse[], limit = 10): RankedSignal[] {
  const counts = new Map<string, number>();
  for (const day of days) {
    for (const app of day.descriptiveStats.topApps) {
      counts.set(app.processName, (counts.get(app.processName) ?? 0) + app.activeSeconds);
    }
  }
  return rankCounts(counts).slice(0, limit);
}

function rankProjects(
  days: DailyBriefResponse[],
  visualWindowsByDate: Record<string, VisualWindowSummary[]>,
  dateRange: string[],
  limit = 10,
): RankedSignal[] {
  const counts = new Map<string, number>();

  for (const day of days) {
    for (const report of [...day.hourlyReports, ...day.fiveHourReports]) {
      for (const hint of report.projectHints) {
        increment(counts, normalizeProjectLabel(hint));
      }
      if (/time\s+state\s+recorder/i.test(report.summaryText)) {
        increment(counts, "time-state-recorder");
      }
    }
  }

  for (const date of dateRange) {
    const uniqueDailyHints = new Set(
      (visualWindowsByDate[date] ?? []).flatMap((summary) =>
        summary.projectHints.map(normalizeProjectLabel),
      ),
    );
    for (const hint of uniqueDailyHints) {
      increment(counts, hint);
    }
  }

  return rankCounts(counts).slice(0, limit);
}

function extractUncertainLines(reports: InsightReport[], limit = 6): string[] {
  return reports
    .flatMap((report) => splitSentences(report.summaryText))
    .filter((line) => UNCERTAINTY_PATTERN.test(line))
    .slice(0, limit);
}

function reportTerms(text: string): string[] {
  const normalized = text.toLowerCase();
  return Array.from(normalized.matchAll(/[a-z][a-z0-9-]{2,}|[\u4e00-\u9fa5]{2,}/g))
    .map((match) => match[0])
    .filter((term) => !STOPWORDS.has(term) && !isNoisyChineseTerm(term));
}

function splitSentences(text: string): string[] {
  return text
    .split(/(?<=[。.!?；;])\s*/)
    .map((line) => line.trim())
    .filter(Boolean);
}

function isNoisyChineseTerm(term: string): boolean {
  return /[\u4e00-\u9fa5]/.test(term) && /[的是和与及在了]/.test(term);
}

function sumReports(days: DailyBriefResponse[], kind: string): number {
  return days.reduce(
    (total, day) =>
      total +
      [...day.hourlyReports, ...day.fiveHourReports].filter(
        (report) => report.reportKind === kind,
      ).length,
    0,
  );
}

function rankCounts(counts: Map<string, number>): RankedSignal[] {
  return [...counts.entries()]
    .filter(([label]) => label.length > 0)
    .map(([label, count]) => ({ label, count }))
    .sort(
      (left, right) =>
        right.count - left.count ||
        Number(isAsciiSignal(right.label)) - Number(isAsciiSignal(left.label)) ||
        left.label.localeCompare(right.label),
    );
}

function increment(counts: Map<string, number>, label: string): void {
  if (!label) {
    return;
  }
  counts.set(label, (counts.get(label) ?? 0) + 1);
}

function normalizeProjectLabel(label: string): string {
  return label.trim().toLowerCase().replace(/\s+/g, "-");
}

function isAsciiSignal(label: string): boolean {
  return /^[a-z0-9-]+$/i.test(label);
}

function formatHours(hours: number): string {
  return `${hours.toFixed(1)}h`;
}

function parseDate(value: string): Date | undefined {
  const date = new Date(`${value}T00:00:00Z`);
  return Number.isNaN(date.getTime()) ? undefined : date;
}

function addDays(date: Date, days: number): Date {
  const next = new Date(date);
  next.setUTCDate(next.getUTCDate() + days);
  return next;
}

function formatDate(date: Date): string {
  return date.toISOString().slice(0, 10);
}
