import { Clock, Layers, ShieldAlert } from "lucide-react";
import { useMemo } from "react";
import type { TimeEvent } from "./types";
import {
  buildHourlyTimelineItems,
  buildTimelineItems,
  filterTimelineEvents,
  formatDuration,
  visibleTimelineTitle,
  type DensityMode,
  type LayerVisibility,
  type PrivacyMode,
  type TimelineGranularity
} from "./lib/uiModel";

type TimelineViewProps = {
  events: TimeEvent[];
  layers: LayerVisibility;
  densityMode: DensityMode;
  privacyMode: PrivacyMode;
  granularity: TimelineGranularity;
};

export function TimelineView({
  events,
  layers,
  densityMode,
  privacyMode,
  granularity
}: TimelineViewProps) {
  const visibleEvents = useMemo(
    () => filterTimelineEvents(events, layers),
    [events, layers]
  );
  const items = useMemo(
    () =>
      granularity === "hour"
        ? buildHourlyTimelineItems(visibleEvents)
        : buildTimelineItems(visibleEvents),
    [granularity, visibleEvents]
  );

  return (
    <section className={`timelineView density-${densityMode}`} aria-label="Timeline">
      <div className="sectionHeader">
        <div>
          <h2>Timeline</h2>
          <p>{granularity === "hour" ? "Hourly buckets" : "Event-level review"}</p>
        </div>
        <span className="statusPill">{items.length} rows</span>
      </div>

      {items.length === 0 ? (
        <p className="emptyState">No visible timeline items. Turn on Windows or Lifecycle layers.</p>
      ) : (
        <div className="timelineList">
          {items.map((item) => (
            <article className={`timelineCard ${item.kind}`} key={item.id}>
              <div className="timelineCardLead">
                {item.kind === "lifecycle" ? (
                  <ShieldAlert aria-hidden="true" size={18} />
                ) : item.kind === "hour" ? (
                  <Layers aria-hidden="true" size={18} />
                ) : (
                  <Clock aria-hidden="true" size={18} />
                )}
                <div>
                  <strong>{item.app}</strong>
                  <span>{visibleTimelineTitle(item, privacyMode)}</span>
                </div>
              </div>
              <div className="timelineCardMeta">
                <span>{formatTime(item.startedAt)}</span>
                <strong>{formatDuration(item.durationSeconds)}</strong>
              </div>
              {item.kind === "hour" && (
                <div className="timelineSplit" aria-label="Hour split">
                  <span>Active {formatDuration(item.activeSeconds)}</span>
                  <span>Lifecycle {formatDuration(item.lifecycleSeconds)}</span>
                </div>
              )}
              {item.status && <span className="timelineStatus">{item.status}</span>}
            </article>
          ))}
        </div>
      )}
    </section>
  );
}

function formatTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return "Invalid";
  }
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}
