import type { DailyBrief, InsightReport } from "../types";

export type ReportPhase = {
  label: string;
  body: string;
  meta?: string;
};

export type StructuredReportPresentation = {
  title: string;
  timeRange: string;
  eyebrow: string;
  overview: string;
  phases: ReportPhase[];
  chips: string[];
  evidence: string[];
  uncertainty: string[];
  rawText: string;
};

const MAX_OVERVIEW_LENGTH = 180;
const MAX_PHASE_BODY_LENGTH = 220;
const MAX_CHIPS = 8;
const OWNER_TIME_ZONE = "Asia/Shanghai";
const RISK_PATTERN = /(不确定|可能|待|缺失|卡住|ERROR|风险|无法)/i;
const NUMBERED_MARKER_PATTERN = /[①②③④⑤⑥⑦⑧⑨]|\d+[).、]/g;
const TIME_SPAN_PATTERN = /\d{1,2}:\d{2}\s*-\s*\d{1,2}:\d{2}/g;
const UTC_TIMESTAMP_PATTERN = /\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z/g;

export function presentInsightReport(report: InsightReport): StructuredReportPresentation {
  const visibleText = normalizeVisibleTimeText(report.summaryText);
  const sections = splitReportText(visibleText);
  const phases = sections.slice(0, 6).map((section, index) => ({
    label: phaseLabel(section, index),
    body: truncateText(stripLeadingMarker(section), MAX_PHASE_BODY_LENGTH),
    meta: index === 0 ? `${report.evidenceCount} source windows` : undefined,
  }));

  return {
    title: report.reportKind === "5h" ? "5h Report" : "1h Report",
    timeRange: formatReportRange(report.periodStart, report.periodEnd),
    eyebrow: `${report.reportKind} - ${report.modelProvider}`,
    overview: truncateText(
      stripLeadingMarker(sections[0] ?? visibleText),
      MAX_OVERVIEW_LENGTH,
    ),
    phases: phases.length > 0 ? phases : fallbackPhase(visibleText),
    chips: reportChips(report),
    evidence: [
      `${report.evidenceCount} windows`,
      ...report.categoryMix.map((item) => `${item.activityCategory} ${item.count}`),
    ],
    uncertainty: extractUncertainty(sections),
    rawText: report.summaryText,
  };
}

export function presentDailyNarrative(brief: DailyBrief): StructuredReportPresentation {
  const rawText = [brief.dailySummaryText, brief.actionTrajectory].filter(Boolean).join("\n");
  const visibleText = normalizeVisibleTimeText(rawText);
  const sections = splitReportText(visibleText);
  const phases = sections.slice(0, 8).map((section, index) => ({
    label: phaseLabel(section, index),
    body: truncateText(stripLeadingMarker(section), MAX_PHASE_BODY_LENGTH),
  }));

  return {
    title: "Daily Action Trajectory",
    timeRange: brief.date,
    eyebrow: `daily - ${brief.modelProvider}`,
    overview: truncateText(
      stripLeadingMarker(sections[0] ?? visibleText),
      MAX_OVERVIEW_LENGTH,
    ),
    phases: phases.length > 0 ? phases : fallbackPhase(visibleText),
    chips: [`${brief.fiveHourReportIds.length} 5h reports`],
    evidence: [],
    uncertainty: extractUncertainty(sections),
    rawText,
  };
}

export function splitReportText(text: string): string[] {
  return [text]
    .flatMap((part) => part.split(/[；;。]\s*/))
    .flatMap((part) => splitAtMatches(part, NUMBERED_MARKER_PATTERN))
    .flatMap((part) => splitAtMatches(part, TIME_SPAN_PATTERN))
    .map((part) => part.trim())
    .filter(Boolean);
}

export function truncateText(text: string, maxLength: number): string {
  if (text.length <= maxLength) {
    return text;
  }
  if (maxLength <= 3) {
    return text.slice(0, maxLength);
  }
  return `${text.slice(0, maxLength - 3).trimEnd()}...`;
}

export function normalizeVisibleTimeText(text: string): string {
  return text.replace(UTC_TIMESTAMP_PATTERN, (token) => formatClock(new Date(token)));
}

export function formatReportRange(startIso: string, endIso: string): string {
  return `${formatClock(new Date(startIso))} - ${formatClock(new Date(endIso))}`;
}

function formatClock(value: Date): string {
  return new Intl.DateTimeFormat("zh-CN", {
    timeZone: OWNER_TIME_ZONE,
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(value);
}

function reportChips(report: InsightReport): string[] {
  return [
    ...report.projectHints,
    `${report.evidenceCount} windows`,
    ...report.categoryMix.map((item) => item.activityCategory),
  ]
    .filter(Boolean)
    .slice(0, MAX_CHIPS);
}

function extractUncertainty(sections: string[]): string[] {
  return sections
    .filter((section) => RISK_PATTERN.test(section))
    .slice(0, 3)
    .map((section) => truncateText(stripLeadingMarker(section), 120));
}

function phaseLabel(section: string, index: number): string {
  const timeMatch = section.match(/\d{1,2}:\d{2}\s*-\s*\d{1,2}:\d{2}/);
  if (timeMatch) {
    return timeMatch[0].replace(/\s+/g, "");
  }
  return `Phase ${index + 1}`;
}

function stripLeadingMarker(text: string): string {
  return text.replace(/^(?:[①②③④⑤⑥⑦⑧⑨]|\d+[).、])\s*/, "").trim();
}

function fallbackPhase(text: string): ReportPhase[] {
  return [{ label: "Summary", body: truncateText(text, MAX_PHASE_BODY_LENGTH) }];
}

function splitAtMatches(text: string, pattern: RegExp): string[] {
  pattern.lastIndex = 0;
  const matches = Array.from(text.matchAll(pattern));
  if (matches.length <= 1) {
    return [text];
  }

  const parts: string[] = [];
  let startIndex = 0;
  for (const match of matches) {
    const matchIndex = match.index ?? 0;
    if (matchIndex > startIndex) {
      parts.push(text.slice(startIndex, matchIndex));
      startIndex = matchIndex;
    }
  }
  parts.push(text.slice(startIndex));
  return parts;
}
