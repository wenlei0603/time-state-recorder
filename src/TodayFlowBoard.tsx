import {
  Activity,
  AlertTriangle,
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
        <div className="evidencePreview" aria-label="Privacy and collector health">
          <Shield aria-hidden="true" size={18} />
          <div>
            <strong>{privacyMode === "raw" ? "Raw mode" : "Redacted mode"}</strong>
            <span>{health ? healthLabel(health.status) : "Health unavailable"}</span>
          </div>
        </div>
      </div>

      <section className="flowSummary" aria-label="Flow summary metrics">
        <FlowMetric
          icon={<Timer aria-hidden="true" size={18} />}
          label="Active"
          value={formatDuration(model.activeSeconds)}
          detail={`${model.buckets.length} flow buckets`}
        />
        <FlowMetric
          icon={<AlertTriangle aria-hidden="true" size={18} />}
          label="Uncertain"
          value={formatDuration(model.uncertainSeconds)}
          detail="gaps and partial evidence"
        />
        <FlowMetric
          icon={<Camera aria-hidden="true" size={18} />}
          label="Evidence"
          value={evidenceTotal.toString()}
          detail={`${model.screenshotCount} captured - ${model.screenshotSkippedCount} skipped`}
        />
        <FlowMetric
          icon={<Keyboard aria-hidden="true" size={18} />}
          label="Input"
          value={model.inputChars.toLocaleString()}
          detail="characters summarized"
        />
      </section>

      <div className="flowBoardGrid">
        <section className="flowLanePanel" aria-label="Time flow">
          <div className="panelHeader">
            <Activity aria-hidden="true" size={20} />
            <h3>Time flow</h3>
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
                  <span className="flowBucketTime">{formatTime(bucket.startedAt)}</span>
                  <strong>{bucket.app}</strong>
                  <span>{bucket.title}</span>
                  <small>{formatDuration(bucket.durationSeconds)}</small>
                </button>
              ))}
            </div>
          )}
        </section>

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
  return `${formatTime(start)} - ${end ? formatTime(end) : "now"}`;
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
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return "Invalid";
  }
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}
