import {
  Activity,
  AlertTriangle,
  Camera,
  Keyboard,
  Shield,
  Timer,
} from "lucide-react";
import { useMemo } from "react";
import type { ReactNode } from "react";
import { buildTodayFlowModel } from "./lib/flowModel";
import { formatDuration } from "./lib/uiModel";
import type {
  CollectorHealth,
  FlowConfidence,
  FlowEvidence,
  InputSummary,
  PrivacyMode,
  ScreenshotSummary,
  TimeEvent,
} from "./types";

type TodayFlowBoardProps = {
  events: TimeEvent[];
  screenshotSummary?: ScreenshotSummary;
  inputSummary?: InputSummary;
  health?: CollectorHealth;
  privacyMode: PrivacyMode;
  sourceLabel: string;
};

export function TodayFlowBoard({
  events,
  screenshotSummary,
  inputSummary,
  health,
  privacyMode,
  sourceLabel,
}: TodayFlowBoardProps) {
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

  return (
    <section className="flowBoard" aria-label="Today Flow Board">
      <div className="flowBoardHeader">
        <div>
          <p className="eyebrow">Dayflow review</p>
          <h2>Today Flow Board</h2>
          <p>
            {sourceLabel} · {privacyMode === "raw" ? "Raw evidence" : "Redacted evidence"}
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
          detail={`${model.screenshotCount} captured · ${model.screenshotSkippedCount} skipped`}
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
                  className={`flowBucket ${bucket.confidence}`}
                  key={bucket.id}
                  aria-label={`${bucket.app}, ${formatDuration(bucket.durationSeconds)}`}
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
            <div className="evidenceFacts">
              {model.evidence.map((item) => (
                <EvidenceRow evidence={item} key={item.id} />
              ))}
            </div>
          )}
        </section>
      </div>
    </section>
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

function formatTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return "Invalid";
  }
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}
