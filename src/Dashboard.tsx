import { Activity, Clock, Keyboard, MousePointer2, ShieldAlert } from "lucide-react";
import { useMemo } from "react";
import type { ReactNode } from "react";
import type { TextSegment, TimeEvent } from "./types";
import {
  buildDashboardSummary,
  buildTimelineItems,
  filterTimelineEvents,
  formatDuration,
  visibleTimelineTitle,
  type DensityMode,
  type LayerVisibility,
  type PrivacyMode,
  type UiSourceMode
} from "./lib/uiModel";

type DashboardProps = {
  events: TimeEvent[];
  segments: TextSegment[];
  layers: LayerVisibility;
  densityMode: DensityMode;
  privacyMode: PrivacyMode;
  inputSourceMode: UiSourceMode;
};

export function Dashboard({
  events,
  segments,
  layers,
  densityMode,
  privacyMode,
  inputSourceMode
}: DashboardProps) {
  const summary = useMemo(
    () => buildDashboardSummary(events, segments),
    [events, segments]
  );
  const recentItems = useMemo(
    () =>
      buildTimelineItems(filterTimelineEvents(events, layers))
        .slice(-5)
        .reverse(),
    [events, layers]
  );

  return (
    <section className={`dashboard density-${densityMode}`} aria-label="Dashboard">
      <div className="sectionHeader">
        <div>
          <h2>Dashboard</h2>
          <p>Daily review workspace</p>
        </div>
        <span className="statusPill">{privacyMode === "raw" ? "Raw" : "Redacted"}</span>
      </div>

      <section className="statsGrid" aria-label="Dashboard metrics">
        <Metric
          icon={<Activity aria-hidden="true" size={18} />}
          label="Active Time"
          value={formatDuration(summary.activeSeconds)}
        />
        <Metric
          icon={<ShieldAlert aria-hidden="true" size={18} />}
          label="Lifecycle"
          value={formatDuration(summary.lifecycleSeconds)}
        />
        <Metric
          icon={<Clock aria-hidden="true" size={18} />}
          label="Focus Blocks"
          value={summary.focusBlockCount.toString()}
        />
        <Metric
          icon={<MousePointer2 aria-hidden="true" size={18} />}
          label="Context Switches"
          value={summary.contextSwitchCount.toString()}
        />
        <Metric
          icon={<Keyboard aria-hidden="true" size={18} />}
          label="Correction Ratio"
          value={`${Math.round(summary.input.correctionRatio * 100)}%`}
        />
      </section>

      <section className="workspace dashboardWorkspace">
        <div className="panel">
          <div className="panelHeader">
            <Activity aria-hidden="true" size={20} />
            <h3>Work Pattern</h3>
          </div>
          <dl className="statusList">
            <div>
              <dt>Top App</dt>
              <dd>{summary.topApp?.app ?? "N/A"}</dd>
            </div>
            <div>
              <dt>Top App Time</dt>
              <dd>{formatDuration(summary.topApp?.totalSeconds ?? 0)}</dd>
            </div>
            <div>
              <dt>Input Bursts</dt>
              <dd>{summary.input.burstCount}</dd>
            </div>
            <div>
              <dt>Input Apps</dt>
              <dd>{summary.input.activeAppCount}</dd>
            </div>
            <div>
              <dt>Input Source</dt>
              <dd>{layers.input ? inputSourceMode : "Hidden"}</dd>
            </div>
            <div>
              <dt>Screenshots</dt>
              <dd>{layers.screenshots ? "Visible" : "Hidden"}</dd>
            </div>
          </dl>
        </div>

        <div className="panel">
          <div className="panelHeader">
            <Clock aria-hidden="true" size={20} />
            <h3>Recent Timeline</h3>
          </div>
          {recentItems.length === 0 ? (
            <p className="emptyState">No visible timeline items.</p>
          ) : (
            <div className="timelineList compactList">
              {recentItems.map((item) => (
                <article className={`timelineCard ${item.kind}`} key={item.id}>
                  <div>
                    <strong>{item.app}</strong>
                    <span>{visibleTimelineTitle(item, privacyMode)}</span>
                  </div>
                  <time>{formatDuration(item.durationSeconds)}</time>
                </article>
              ))}
            </div>
          )}
        </div>
      </section>
    </section>
  );
}

function Metric({
  icon,
  label,
  value
}: {
  icon: ReactNode;
  label: string;
  value: string;
}) {
  return (
    <article className="metric metricWithIcon">
      <span className="metricIcon">{icon}</span>
      <span>{label}</span>
      <strong>{value}</strong>
    </article>
  );
}
