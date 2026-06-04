import type {
  ActivityCategory,
  InsightReport,
  VisualTrajectoryPoint,
  VisualWindowSummary,
} from "../types";

export type WindowReviewNote = {
  thesis: string;
  intent: string;
  continuity: string;
  primaryActivity: ActivityCategory;
  confidence: number;
  projectHints: string[];
  trajectory: VisualTrajectoryPoint[];
  switchingLevel: string;
  switchingEvidence: string;
  loafingLevel: string;
  loafingEvidence: string;
  riskFlags: string[];
  visibleApps: string[];
  rawAvailable: boolean;
  rawText?: string;
};

export type ReportPhase = {
  label: string;
  detail: string;
};

export type ReportReviewNote = {
  mainThread: string;
  phases: ReportPhase[];
  projects: string[];
  fullText: string;
  rawAvailable: boolean;
  rawText?: string;
};

type ParsedSummary = {
  parsed: boolean;
  looksRaw: boolean;
  value?: Record<string, unknown>;
  rawText?: string;
};

const FALLBACK_WINDOW_THESIS = "Summary generated, details unavailable";
const FALLBACK_REPORT_THREAD = "Report generated, details unavailable";

export function stripJsonSummarySyntax(value: string): string {
  const parsed = parseSummaryPayload(value);
  if (parsed.value) {
    const summaryText = readOptionalString(parsed.value, "summaryText");
    if (summaryText) {
      return summaryText;
    }
  }
  return parsed.looksRaw ? "" : value.trim();
}

export function buildWindowNote(summary: VisualWindowSummary): WindowReviewNote {
  const parsed = parseSummaryPayload(summary.summaryText);
  const payload = parsed.value;
  const parsedSummaryText = payload ? readOptionalString(payload, "summaryText") : "";
  const parsedTaskIntent = payload ? readOptionalString(payload, "taskIntent") : "";
  const parsedContinuity = payload ? readOptionalString(payload, "continuity") : "";
  const parsedProjectHints = payload ? readOptionalStringArray(payload, "projectHints") : [];
  const parsedVisibleApps = payload ? readOptionalStringArray(payload, "visibleApps") : [];
  const parsedRiskFlags = payload ? readOptionalStringArray(payload, "riskFlags") : [];
  const parsedTrajectory = payload ? readOptionalTrajectory(payload, "trajectory") : [];

  const cleanSummary = stripJsonSummarySyntax(summary.summaryText);
  const thesis = firstText(
    parsedSummaryText,
    cleanSummary,
    parsedTaskIntent,
    summary.taskIntent,
    summary.continuity,
    FALLBACK_WINDOW_THESIS,
  );

  return {
    thesis,
    intent: firstText(parsedTaskIntent, summary.taskIntent),
    continuity: firstText(parsedContinuity, summary.continuity),
    primaryActivity: summary.primaryActivity,
    confidence: summary.confidence,
    projectHints:
      parsedProjectHints.length > 0 ? uniqueStrings(parsedProjectHints) : summary.projectHints,
    trajectory: parsedTrajectory.length > 0 ? parsedTrajectory : summary.trajectory,
    switchingLevel: summary.switchingLevel,
    switchingEvidence: summary.switchingEvidence,
    loafingLevel: summary.loafingLevel,
    loafingEvidence: summary.loafingEvidence,
    riskFlags: parsedRiskFlags.length > 0 ? uniqueStrings(parsedRiskFlags) : summary.riskFlags,
    visibleApps: parsedVisibleApps.length > 0 ? uniqueStrings(parsedVisibleApps) : summary.visibleApps,
    rawAvailable: parsed.parsed || parsed.looksRaw || summary.rawSummaryJson !== null,
    rawText: rawWindowText(summary, parsed),
  };
}

export function buildReportNote(report: InsightReport): ReportReviewNote {
  const parsed = parseSummaryPayload(report.summaryText);
  const payload = parsed.value;
  const parsedSummaryText = payload ? readOptionalString(payload, "summaryText") : "";
  const fullText = firstText(
    parsedSummaryText,
    stripJsonSummarySyntax(report.summaryText),
    report.summaryText,
  );
  const sectioned = splitReportText(fullText);

  return {
    mainThread: firstText(sectioned.mainThread, FALLBACK_REPORT_THREAD),
    phases: sectioned.phases,
    projects: report.projectHints,
    fullText,
    rawAvailable: parsed.parsed || parsed.looksRaw,
    rawText: parsed.rawText,
  };
}

function splitReportText(text: string): { mainThread: string; phases: ReportPhase[] } {
  const normalized = normalizeWhitespace(text);
  const markers = [...normalized.matchAll(/[①②③④⑤⑥⑦⑧⑨]/g)];
  if (markers.length === 0) {
    const sentences = normalized.split(/(?<=[。.!?])\s+/).filter(Boolean);
    return {
      mainThread: sentences[0] ?? normalized,
      phases: sentences.slice(1, 4).map((sentence, index) => ({
        label: `Phase ${index + 1}`,
        detail: sentence,
      })),
    };
  }

  const mainThread = normalized.slice(0, markers[0].index).trim();
  const phases = markers.map((marker, index) => {
    const start = (marker.index ?? 0) + marker[0].length;
    const end =
      index + 1 < markers.length ? markers[index + 1].index ?? normalized.length : normalized.length;
    return toPhase(normalized.slice(start, end));
  });

  return { mainThread, phases };
}

function toPhase(segment: string): ReportPhase {
  const clean = segment.trim().replace(/^[。.;；\s]+/, "");
  const separator = clean.search(/[：:]/);
  if (separator > 0) {
    return {
      label: clean.slice(0, separator).trim(),
      detail: clean.slice(separator + 1).trim(),
    };
  }
  const firstSentence = clean.split(/(?<=[。.!?])\s+/)[0] ?? clean;
  return {
    label: firstSentence.slice(0, 36),
    detail: clean,
  };
}

function parseSummaryPayload(value: string): ParsedSummary {
  const rawText = value.trim();
  const unfenced = removeCodeFence(rawText);
  const direct = tryParseRecord(unfenced);
  if (direct) {
    return { parsed: true, looksRaw: true, value: direct, rawText };
  }

  const objectCandidate = jsonObjectCandidate(unfenced);
  if (objectCandidate) {
    const objectRecord = tryParseRecord(objectCandidate);
    if (objectRecord) {
      return { parsed: true, looksRaw: true, value: objectRecord, rawText };
    }
  }

  return { parsed: false, looksRaw: looksJsonLike(rawText), rawText };
}

function removeCodeFence(value: string): string {
  const match = value.match(/^```(?:json)?\s*([\s\S]*?)\s*```$/i);
  if (match) {
    return match[1].trim();
  }
  return value.replace(/^```(?:json)?/i, "").replace(/```$/i, "").trim();
}

function jsonObjectCandidate(value: string): string | undefined {
  const start = value.indexOf("{");
  const end = value.lastIndexOf("}");
  if (start >= 0 && end > start) {
    return value.slice(start, end + 1);
  }
  return undefined;
}

function tryParseRecord(value: string): Record<string, unknown> | undefined {
  try {
    const parsed: unknown = JSON.parse(value);
    if (typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)) {
      return parsed as Record<string, unknown>;
    }
  } catch {
    return undefined;
  }
  return undefined;
}

function looksJsonLike(value: string): boolean {
  const trimmed = value.trim();
  return (
    trimmed.startsWith("```") ||
    trimmed.startsWith("{") ||
    /\bsummaryText\b/.test(trimmed) ||
    /\\"[a-zA-Z]+/.test(trimmed)
  );
}

function readOptionalString(
  record: Record<string, unknown>,
  key: string,
): string {
  const value = record[key];
  return typeof value === "string" ? value.trim() : "";
}

function readOptionalStringArray(
  record: Record<string, unknown>,
  key: string,
): string[] {
  const value = record[key];
  if (!Array.isArray(value)) {
    return [];
  }
  return value.filter((item): item is string => typeof item === "string");
}

function readOptionalTrajectory(
  record: Record<string, unknown>,
  key: string,
): VisualTrajectoryPoint[] {
  const value = record[key];
  if (!Array.isArray(value)) {
    return [];
  }
  return value.flatMap((item) => {
    if (typeof item !== "object" || item === null || Array.isArray(item)) {
      return [];
    }
    const row = item as Record<string, unknown>;
    const minuteMark = row.minuteMark;
    const screenshotId = row.screenshotId;
    const observation = row.observation;
    const activityCategory = row.activityCategory;
    if (
      typeof minuteMark !== "number" ||
      typeof screenshotId !== "number" ||
      typeof observation !== "string" ||
      !isActivityCategory(activityCategory)
    ) {
      return [];
    }
    return [{ minuteMark, screenshotId, observation, activityCategory }];
  });
}

function isActivityCategory(value: unknown): value is ActivityCategory {
  return (
    value === "project_work" ||
    value === "research" ||
    value === "writing" ||
    value === "coding" ||
    value === "communication" ||
    value === "meeting" ||
    value === "admin" ||
    value === "learning" ||
    value === "planning" ||
    value === "loafing" ||
    value === "personal" ||
    value === "idle" ||
    value === "unknown"
  );
}

function rawWindowText(summary: VisualWindowSummary, parsed: ParsedSummary): string | undefined {
  if (parsed.rawText && (parsed.parsed || parsed.looksRaw)) {
    return parsed.rawText;
  }
  if (summary.rawSummaryJson !== null && summary.rawSummaryJson !== undefined) {
    return JSON.stringify(summary.rawSummaryJson, null, 2);
  }
  return undefined;
}

function firstText(...values: string[]): string {
  return values.find((value) => value.trim().length > 0)?.trim() ?? "";
}

function normalizeWhitespace(value: string): string {
  return value.replace(/\s+/g, " ").trim();
}

function uniqueStrings(values: string[]): string[] {
  return [...new Set(values.map((value) => value.trim()).filter(Boolean))];
}
