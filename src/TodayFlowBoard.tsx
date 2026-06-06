import {
  Activity,
  Camera,
  Keyboard,
  Shield,
  Sparkles,
  Timer,
} from "lucide-react";
import { useMemo, useState } from "react";
import type { ReactNode } from "react";
import { buildTodayFlowModel } from "./lib/flowModel";
import { formatDuration } from "./lib/uiModel";
import type {
  CollectorHealth,
  FlowConfidence,
  FlowEvidence,
  InputSummary,
  PrivacyMode,
  ScreenshotMeta,
  ScreenshotSummary,
  TimeEvent,
  VisualSummary,
} from "./types";

type TodayFlowBoardProps = {
  events: TimeEvent[];
  screenshotSummary?: ScreenshotSummary;
  screenshots?: ScreenshotMeta[];
  inputSummary?: InputSummary;
  health?: CollectorHealth;
  privacyMode: PrivacyMode;
  screenshotsVisible: boolean;
  visualSummaries: VisualSummary[];
  analyzingScreenshotId?: number | null;
  onAnalyzeScreenshot: (screenshotId: number) => void;
  sourceLabel: string;
};

const SCREENSHOT_CONTEXT_WINDOW_MS = 5 * 60 * 1000;

export function TodayFlowBoard({
  events,
  screenshotSummary,
  screenshots = [],
  inputSummary,
  health,
  privacyMode,
  screenshotsVisible,
  visualSummaries,
  analyzingScreenshotId,
  onAnalyzeScreenshot,
  sourceLabel,
}: TodayFlowBoardProps) {
  const [selectedBucketId, setSelectedBucketId] = useState<string | null>(null);
  const model = useMemo(
    () =>
      buildTodayFlowModel({
        events,
        screenshotSummary,
        inputSummary,
        privacyMode,
      }),
    [events, screenshotSummary, inputSummary, privacyMode],
  );
  const evidenceTotal = model.screenshotCount + model.screenshotSkippedCount;
  const selectedBucket =
    model.buckets.find((bucket) => bucket.id === selectedBucketId) ??
    model.buckets[0];
  const selectedEvidence = selectedBucket?.evidence ?? [];
  const primaryApp = primaryAppLabel(model.evidence);
  const reviewRange = reviewRangeLabel(model.evidence);
  const selectedRange = selectedBucket
    ? formatRange(selectedBucket.startedAt, selectedBucket.endedAt)
    : "No selected window";
  const selectedDuration = selectedBucket
    ? formatDuration(selectedBucket.durationSeconds)
    : "0s";
  const coverageDetail = `${model.screenshotCount} captured / ${model.screenshotSkippedCount} skipped`;
  const todaySummary =
    model.buckets.length === 0
      ? "No time events are available yet. The board is holding a stable review layout for the next collector update."
      : `${formatDuration(model.activeSeconds)} of known work time across ${
          model.buckets.length
        } flow segments. Main focus is ${primaryApp}.`;
  const selectedScreenshots = selectedBucket
    ? selectScreenshotsForBucket(screenshots, selectedBucket)
    : [];
  const summaryByScreenshotId = useMemo(() => {
    const map = new Map<number, VisualSummary>();
    for (const summary of visualSummaries) {
      map.set(summary.screenshotId, summary);
    }
    return map;
  }, [visualSummaries]);

  return (
    <section className="flowBoard" aria-label="Today Flow Board">
      <div className="flowBoardHeader">
        <div>
          <p className="eyebrow">Dayflow review</p>
          <h2>Today Flow Board</h2>
          <p>
            {sourceLabel} - {privacyMode === "raw" ? "Raw evidence" : "Redacted evidence"}
          </p>
        </div>
        <div className="todayStatusCard" aria-label="Privacy and collector health">
          <Shield aria-hidden="true" size={18} />
          <div>
            <strong>{privacyMode === "raw" ? "Raw mode" : "Redacted mode"}</strong>
            <span>{health ? healthLabel(health.status) : "Health unavailable"}</span>
          </div>
        </div>
      </div>

      <section
        className="todayGlance"
        aria-labelledby="today-at-a-glance-heading"
      >
        <div className="panelHeader">
          <Sparkles aria-hidden="true" size={20} />
          <h3 id="today-at-a-glance-heading">Today at a glance</h3>
        </div>
        <p className="todayNarrative">{todaySummary}</p>
        <div className="flowSummary" aria-label="Flow summary metrics">
          <FlowMetric
            icon={<Timer aria-hidden="true" size={18} />}
            label="Known work time"
            value={formatDuration(model.activeSeconds)}
            detail={reviewRange}
          />
          <FlowMetric
            icon={<Activity aria-hidden="true" size={18} />}
            label="Main focus"
            value={primaryApp}
            detail={`${model.buckets.length} readable segments`}
          />
          <FlowMetric
            icon={<Camera aria-hidden="true" size={18} />}
            label="Evidence coverage"
            value={evidenceTotal.toString()}
            detail={coverageDetail}
          />
          <FlowMetric
            icon={<Keyboard aria-hidden="true" size={18} />}
            label="Input captured"
            value={model.inputChars.toLocaleString()}
            detail="characters summarized"
          />
        </div>
      </section>

      <div className="todayReadingGrid">
        <section className="todayFocusPanel" aria-label="Current focus">
          <div className="panelHeader">
            <Activity aria-hidden="true" size={20} />
            <h3>Current focus</h3>
          </div>
          {selectedBucket ? (
            <div className="todayFocusBody">
              <span className="todayFocusTime">{selectedRange}</span>
              <strong>{selectedBucket.app}</strong>
              <p>{selectedBucket.title}</p>
              <div className="todayFocusMeta">
                <span>{selectedDuration}</span>
                <span className={`confidencePill ${selectedBucket.confidence}`}>
                  {confidenceLabel(selectedBucket.confidence)}
                </span>
              </div>
            </div>
          ) : (
            <p className="emptyState">No current focus segment is available.</p>
          )}
        </section>

        <section className="todayCoveragePanel" aria-label="Evidence coverage">
          <div className="panelHeader">
            <Shield aria-hidden="true" size={20} />
            <h3>Evidence coverage</h3>
          </div>
          <div className="todayCoverageGrid">
            <span>
              <strong>{model.screenshotCount}</strong>
              <small>screenshots captured</small>
            </span>
            <span>
              <strong>{model.screenshotSkippedCount}</strong>
              <small>screenshots skipped</small>
            </span>
            <span>
              <strong>{formatDuration(model.uncertainSeconds)}</strong>
              <small>uncertain time</small>
            </span>
          </div>
        </section>
      </div>

      <section className="flowLanePanel readableTimeline" aria-label="Readable timeline">
        <div className="panelHeader">
          <Activity aria-hidden="true" size={20} />
          <h3>Readable timeline</h3>
          <span>Time flow</span>
        </div>
        {model.buckets.length === 0 ? (
          <p className="emptyState">No time events are available yet.</p>
        ) : (
          <div className="flowLane">
            {model.buckets.map((bucket) => (
              <button
                type="button"
                className={`flowBucket ${bucket.confidence} ${
                  selectedBucket?.id === bucket.id ? "selected" : ""
                }`}
                key={bucket.id}
                aria-controls="today-flow-evidence-drawer"
                aria-label={`${bucket.app}, ${formatDuration(bucket.durationSeconds)}, ${bucket.title}`}
                aria-pressed={selectedBucket?.id === bucket.id}
                onClick={() => setSelectedBucketId(bucket.id)}
              >
                <span className="flowBucketTime">
                  {formatRange(bucket.startedAt, bucket.endedAt)}
                </span>
                <strong>{bucket.app}</strong>
                <span>{bucket.title}</span>
                <small>{formatDuration(bucket.durationSeconds)}</small>
              </button>
            ))}
          </div>
        )}
      </section>

      <div className="flowBoardGrid">
        <section className="evidenceDrawer" aria-label="Evidence drawer">
          <div className="panelHeader">
            <Shield aria-hidden="true" size={20} />
            <h3>Evidence drawer</h3>
          </div>
          {model.evidence.length === 0 ? (
            <p className="emptyState">No time events are available yet.</p>
          ) : (
            <div className="evidenceFacts" id="today-flow-evidence-drawer">
              {selectedEvidence.map((item) => (
                <EvidenceRow evidence={item} key={item.id} />
              ))}
              <ScreenshotEvidence
                privacyMode={privacyMode}
                screenshots={selectedScreenshots}
                screenshotsVisible={screenshotsVisible}
                summaryByScreenshotId={summaryByScreenshotId}
                analyzingScreenshotId={analyzingScreenshotId}
                onAnalyzeScreenshot={onAnalyzeScreenshot}
              />
            </div>
          )}
        </section>
      </div>
    </section>
  );
}

function ScreenshotEvidence({
  privacyMode,
  screenshots,
  screenshotsVisible,
  summaryByScreenshotId,
  analyzingScreenshotId,
  onAnalyzeScreenshot,
}: {
  privacyMode: PrivacyMode;
  screenshots: ScreenshotMeta[];
  screenshotsVisible: boolean;
  summaryByScreenshotId: Map<number, VisualSummary>;
  analyzingScreenshotId?: number | null;
  onAnalyzeScreenshot: (screenshotId: number) => void;
}) {
  if (!screenshotsVisible) {
    return (
      <p className="flowHint">
        Screenshots layer is disabled for this evidence drawer.
      </p>
    );
  }

  if (privacyMode !== "raw") {
    return (
      <p className="flowHint">
        Screenshot preview hidden in redacted mode.
      </p>
    );
  }

  if (screenshots.length === 0) {
    return <p className="flowHint">No screenshot rows overlap this bucket.</p>;
  }

  return (
    <div className="todayScreenshotStrip" aria-label="Screenshot evidence">
      {screenshots.slice(0, 4).map((shot) => {
        const summary = summaryByScreenshotId.get(shot.id);
        return (
          <figure className="todayScreenshotCard" key={shot.id}>
            <img
              src={`/screenshots/${shot.filePath}`}
              alt={`Evidence screenshot at ${formatTime(shot.capturedAt)}`}
              width={shot.width}
              height={shot.height}
              loading="lazy"
            />
            <figcaption>
              <span>{formatTime(shot.capturedAt)}</span>
              <strong>{shot.processName ?? "Unknown"}</strong>
            </figcaption>
            <button
              type="button"
              className="analysisButton"
              disabled={analyzingScreenshotId === shot.id}
              onClick={() => onAnalyzeScreenshot(shot.id)}
            >
              <Sparkles aria-hidden="true" size={16} />
              <span>
                {analyzingScreenshotId === shot.id
                  ? "Analyzing..."
                  : "Analyze screenshot"}
              </span>
            </button>
            {summary ? (
              <div className="visualSummaryCard">
                <strong>{summary.modelProvider}</strong>
                <p>{summary.summaryText}</p>
              </div>
            ) : null}
          </figure>
        );
      })}
    </div>
  );
}

function FlowMetric({
  icon,
  label,
  value,
  detail,
}: {
  icon: ReactNode;
  label: string;
  value: string;
  detail: string;
}) {
  return (
    <article className="flowMetric">
      <span className="metricIcon">{icon}</span>
      <span>{label}</span>
      <strong>{value}</strong>
      <small>{detail}</small>
    </article>
  );
}

function EvidenceRow({ evidence }: { evidence: FlowEvidence }) {
  return (
    <article className="evidencePreview">
      <Activity aria-hidden="true" size={18} />
      <div>
        <strong>{evidence.app}</strong>
        <span>{evidence.title}</span>
        <small>{formatRange(evidence.startedAt, evidence.endedAt)}</small>
      </div>
      <span className={`confidencePill ${evidence.confidence}`}>
        {confidenceLabel(evidence.confidence)}
      </span>
    </article>
  );
}

function primaryAppLabel(evidence: FlowEvidence[]): string {
  const totals = new Map<string, number>();
  for (const item of evidence) {
    if (item.kind === "lifecycle") {
      continue;
    }
    totals.set(item.app, (totals.get(item.app) ?? 0) + item.durationSeconds);
  }

  const [top] = [...totals.entries()].sort(
    (left, right) => right[1] - left[1] || left[0].localeCompare(right[0]),
  );
  return top?.[0] ?? "No active app";
}

function reviewRangeLabel(evidence: FlowEvidence[]): string {
  let rangeStart: Date | null = null;
  let rangeEnd: Date | null = null;

  for (const item of evidence) {
    const startedAt = parseDate(item.startedAt);
    const endedAt = parseDate(item.endedAt ?? item.startedAt);
    if (!startedAt || !endedAt) {
      continue;
    }

    if (!rangeStart || startedAt < rangeStart) {
      rangeStart = startedAt;
    }
    if (!rangeEnd || endedAt > rangeEnd) {
      rangeEnd = endedAt;
    }
  }

  if (!rangeStart || !rangeEnd) {
    return "No review window";
  }

  return formatRangeFromDates(rangeStart, rangeEnd);
}

function healthLabel(status: CollectorHealth["status"]): string {
  switch (status) {
    case "ok":
      return "Collector healthy";
    case "degraded":
      return "Collector degraded";
    case "error":
      return "Collector error";
  }
}

function confidenceLabel(confidence: FlowConfidence): string {
  switch (confidence) {
    case "high":
      return "High";
    case "partial":
      return "Partial";
    case "uncertain":
      return "Uncertain";
  }
}

function formatRange(start: string, end?: string): string {
  const startedAt = parseDate(start);
  if (!startedAt) {
    return "Invalid";
  }

  if (!end) {
    return `${formatClock(startedAt)} - now`;
  }

  const endedAt = parseDate(end);
  if (!endedAt) {
    return `${formatClock(startedAt)} - Invalid`;
  }

  return formatRangeFromDates(startedAt, endedAt);
}

function overlapsBucket(
  shot: ScreenshotMeta,
  bucket: { startedAt: string; endedAt?: string },
): boolean {
  const capturedAt = Date.parse(shot.capturedAt);
  const startedAt = Date.parse(bucket.startedAt);

  if (!Number.isFinite(capturedAt) || !Number.isFinite(startedAt)) {
    return false;
  }

  if (!bucket.endedAt) {
    return capturedAt >= startedAt;
  }

  const endedAt = Date.parse(bucket.endedAt);
  if (!Number.isFinite(endedAt)) {
    return false;
  }

  return capturedAt >= startedAt && capturedAt <= endedAt;
}

function selectScreenshotsForBucket(
  screenshots: ScreenshotMeta[],
  bucket: { startedAt: string; endedAt?: string },
): ScreenshotMeta[] {
  const overlapping = screenshots.filter((shot) => overlapsBucket(shot, bucket));
  if (overlapping.length > 0) {
    return overlapping;
  }

  const anchor = bucketAnchorTime(bucket);
  if (!Number.isFinite(anchor)) {
    return [];
  }

  return screenshots
    .map((shot) => ({
      shot,
      distance: Math.abs(Date.parse(shot.capturedAt) - anchor),
    }))
    .filter(({ distance }) => Number.isFinite(distance))
    .filter(({ distance }) => distance <= SCREENSHOT_CONTEXT_WINDOW_MS)
    .sort((left, right) => left.distance - right.distance)
    .slice(0, 4)
    .map(({ shot }) => shot);
}

function bucketAnchorTime(bucket: { startedAt: string; endedAt?: string }): number {
  const startedAt = Date.parse(bucket.startedAt);
  if (!Number.isFinite(startedAt)) {
    return Number.NaN;
  }

  const endedAt = bucket.endedAt ? Date.parse(bucket.endedAt) : Number.NaN;
  if (Number.isFinite(endedAt)) {
    return startedAt + (endedAt - startedAt) / 2;
  }

  return startedAt;
}

function formatTime(value: string): string {
  const date = parseDate(value);
  if (!date) {
    return "Invalid";
  }
  return formatClock(date);
}

function parseDate(value: string): Date | null {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? null : date;
}

function formatRangeFromDates(startedAt: Date, endedAt: Date): string {
  const showDate =
    endedAt < startedAt || !isSameLocalDate(startedAt, endedAt);
  if (showDate) {
    return `${formatShortDateTime(startedAt)} - ${formatShortDateTime(endedAt)}`;
  }

  return `${formatClock(startedAt)} - ${formatClock(endedAt)}`;
}

function formatShortDateTime(date: Date): string {
  return `${pad2(date.getMonth() + 1)}/${pad2(date.getDate())} ${formatClock(
    date,
  )}`;
}

function formatClock(date: Date): string {
  return `${pad2(date.getHours())}:${pad2(date.getMinutes())}`;
}

function isSameLocalDate(left: Date, right: Date): boolean {
  return (
    left.getFullYear() === right.getFullYear() &&
    left.getMonth() === right.getMonth() &&
    left.getDate() === right.getDate()
  );
}

function pad2(value: number): string {
  return value.toString().padStart(2, "0");
}
