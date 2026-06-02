import { useEffect, useMemo, useState } from "react";
import type { ActivityBucket, PrivacyMode } from "./types";
import {
  activityCategoryLabel,
  attentionStateLabel,
  formatBucketMinutes,
  summarizeActivityBuckets
} from "./lib/activity";
import type { UiSourceMode } from "./lib/uiModel";

type ActivityReviewProps = {
  date: string;
  buckets: ActivityBucket[];
  sourceMode: UiSourceMode;
  loading: boolean;
  error: string | null;
  privacyMode: PrivacyMode;
  onLoadSample: () => void;
  onLoadLive: () => void;
};

export function ActivityReview({
  date,
  buckets,
  sourceMode,
  loading,
  error,
  privacyMode,
  onLoadSample,
  onLoadLive
}: ActivityReviewProps) {
  const summary = useMemo(() => summarizeActivityBuckets(buckets), [buckets]);
  const [selectedId, setSelectedId] = useState(() => buckets[0]?.id ?? "");
  const selectedBucket =
    buckets.find((bucket) => bucket.id === selectedId) ?? buckets[0];

  useEffect(() => {
    if (!buckets.some((bucket) => bucket.id === selectedId)) {
      setSelectedId(buckets[0]?.id ?? "");
    }
  }, [buckets, selectedId]);

  const dominantShare =
    summary.totalBucketSeconds > 0
      ? Math.round((summary.dominantSeconds / summary.totalBucketSeconds) * 100)
      : 0;

  return (
    <section className="activityReview" aria-label="Activity Review">
      <div className="sectionHeader">
        <div>
          <p className="eyebrow">3-minute buckets / {sourceMode}</p>
          <h2>Activity Review</h2>
          <p className="reviewSubtitle">
            {date} · {summary.bucketCount} buckets · raw titles guarded by privacy mode
          </p>
        </div>
        <div className="inlineActions">
          <button type="button" onClick={onLoadSample}>
            Sample
          </button>
          <button type="button" onClick={onLoadLive}>
            Live
          </button>
        </div>
      </div>

      {loading && (
        <p className="sampleNotice" role="status">
          Loading activity buckets...
        </p>
      )}
      {error && (
        <p className="sampleNotice" role="status">
          Activity bucket layer unavailable. Keeping {sourceMode} activity data visible:{" "}
          {error}
        </p>
      )}

      <section className="activitySummaryGrid" aria-label="Activity review metrics">
        <article className="metric">
          <span>Coverage</span>
          <strong>{formatBucketMinutes(summary.totalBucketSeconds)}</strong>
        </article>
        <article className="metric">
          <span>Dominant Share</span>
          <strong>{dominantShare}%</strong>
        </article>
        <article className="metric">
          <span>Switches</span>
          <strong>{summary.totalSwitches}</strong>
        </article>
        <article className="metric">
          <span>Bucket Size</span>
          <strong>{buckets[0] ? formatBucketMinutes(buckets[0].bucketSeconds) : "0m"}</strong>
        </article>
      </section>

      <div className="activityReviewGrid">
        <section className="panel activityBreakdown" aria-label="Category mix">
          <h3>Category mix</h3>
          {summary.categoryBreakdown.map((item) => (
            <BreakdownBar
              key={item.category}
              label={item.label}
              seconds={item.seconds}
              totalSeconds={summary.totalBucketSeconds}
            />
          ))}
        </section>

        <section className="panel activityBreakdown" aria-label="Attention rhythm">
          <h3>Attention rhythm</h3>
          {summary.attentionBreakdown.map((item) => (
            <BreakdownBar
              key={item.state}
              label={item.label}
              seconds={item.seconds}
              totalSeconds={summary.totalBucketSeconds}
            />
          ))}
        </section>
      </div>

      <div className="activityReviewGrid wide">
        <section className="panel activityBucketList" aria-label="Activity buckets">
          <h3>Activity buckets</h3>
          {buckets.length === 0 ? (
            <p className="emptyState">No activity buckets for this date.</p>
          ) : (
            buckets.map((bucket) => (
              <button
                type="button"
                key={bucket.id}
                className={`activityBucketButton ${
                  selectedBucket?.id === bucket.id ? "selected" : ""
                }`}
                aria-pressed={selectedBucket?.id === bucket.id}
                onClick={() => setSelectedId(bucket.id)}
              >
                <span className="bucketTime">
                  {timeLabel(bucket.startAt)} - {timeLabel(bucket.endAt)}
                </span>
                <strong>{bucket.dominantApp}</strong>
                <span>{activityCategoryLabel(bucket.activityCategory)}</span>
                <span>{attentionStateLabel(bucket.attentionState)}</span>
                <span>
                  {privacyMode === "raw" ? bucket.normalizedTitle : "Hidden in redacted mode"}
                </span>
              </button>
            ))
          )}
        </section>

        <section className="panel activityEvidence" aria-label="Activity evidence">
          <h3>Activity evidence</h3>
          {selectedBucket ? (
            <>
              <div className="selectedActivity">
                <span>{selectedBucket.dominantApp}</span>
                <strong>
                  {privacyMode === "raw"
                    ? selectedBucket.normalizedTitle
                    : "Hidden in redacted mode"}
                </strong>
                <span>
                  {activityCategoryLabel(selectedBucket.activityCategory)} ·{" "}
                  {attentionStateLabel(selectedBucket.attentionState)} ·{" "}
                  {selectedBucket.switchCount} switches
                </span>
              </div>
              <div className="activityEvidenceRows">
                {selectedBucket.evidence.map((item) => (
                  <article key={`${selectedBucket.id}-${item.eventId}`}>
                    <span>
                      {timeLabel(item.startedAt)} - {timeLabel(item.endedAt)}
                    </span>
                    <strong>{item.app}</strong>
                    <p>
                      {privacyMode === "raw"
                        ? item.normalizedTitle
                        : "Hidden in redacted mode"}
                    </p>
                  </article>
                ))}
              </div>
            </>
          ) : (
            <p className="emptyState">Select a bucket to inspect evidence.</p>
          )}
        </section>
      </div>
    </section>
  );
}

function BreakdownBar({
  label,
  seconds,
  totalSeconds
}: {
  label: string;
  seconds: number;
  totalSeconds: number;
}) {
  const share = totalSeconds > 0 ? Math.round((seconds / totalSeconds) * 100) : 0;
  return (
    <div className="breakdownRow">
      <div className="breakdownMeta">
        <span>{label}</span>
        <strong>{formatBucketMinutes(seconds)}</strong>
      </div>
      <div className="breakdownTrack" aria-hidden="true">
        <span style={{ width: `${share}%` }} />
      </div>
    </div>
  );
}

function timeLabel(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return date.toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit"
  });
}
