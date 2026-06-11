import { BarChart3, Clock3, FileText, Flame, Layers } from "lucide-react";
import type { ReactNode } from "react";
import { StructuredDiaryDashboard } from "./StructuredDiaryDashboard";
import { StructuredDailyNarrative } from "./StructuredDailyNarrative";
import { StructuredReportCard } from "./StructuredReportCard";
import { presentInsightReport } from "./lib/reportPresentation";
import type { PrivacyMode, UiSourceMode } from "./lib/uiModel";
import type { DailyBriefResponse, HourlyActivityMetric } from "./types";

type DailyBriefPanelProps = {
  response?: DailyBriefResponse;
  sourceMode: UiSourceMode;
  privacyMode: PrivacyMode;
  loading: boolean;
  error?: string | null;
};

export function DailyBriefPanel({
  response,
  sourceMode,
  privacyMode,
  loading,
  error,
}: DailyBriefPanelProps) {
  const canShowText = privacyMode === "raw";
  const stats = response?.descriptiveStats;
  const brief = response?.brief;
  const hourlyReports = response?.hourlyReports ?? [];
  const fiveHourReports = response?.fiveHourReports ?? [];

  return (
    <section className="dailyBriefPanel" aria-label="Daily Brief">
      <div className="dailyBriefHeader">
        <div>
          <p className="eyebrow">Daily brief</p>
          <h2>Daily Brief</h2>
          <p>
            {response?.date ?? "No date"} ·{" "}
            {sourceMode === "live" ? "backend summary" : "sample workspace"}
          </p>
        </div>
        <span className={`statusPill ${statusClass(response?.status)}`}>
          {loading ? "Refreshing" : response?.status ?? "Missing"}
        </span>
      </div>

      {error ? (
        <p className="errors" role="status">
          {error}
        </p>
      ) : null}

      {stats ? (
        <>
          <div className="dailyBriefStats" aria-label="Daily activity statistics">
            <Metric icon={<Clock3 size={17} />} label="active" value={`${stats.activeHours.toFixed(1)}h active`} />
            <Metric
              icon={<Layers size={17} />}
              label="reports"
              value={`${hourlyReports.length} hourly / ${fiveHourReports.length} 5h`}
            />
            <Metric icon={<BarChart3 size={17} />} label="switches" value={`${stats.switchCount} switches`} />
            <Metric icon={<FileText size={17} />} label="input" value={`${stats.inputChars} chars`} />
          </div>

          <HourlyHeatmap metrics={response?.hourlyMetrics ?? []} />

          {response?.diaryDashboard ? (
            <div className="dailyBriefSection">
              <h3>Diary Dashboard</h3>
              <StructuredDiaryDashboard
                dashboard={response.diaryDashboard}
                canShowText={canShowText}
              />
            </div>
          ) : null}

          <div className="dailyBriefSection">
            <h3>Past Comparison</h3>
            <p>{response?.comparison.explanation ?? "No baseline comparison yet."}</p>
          </div>

          <div className="dailyBriefSection">
            <h3>Daily Action Trajectory</h3>
            {brief ? (
              <StructuredDailyNarrative brief={brief} canShowText={canShowText} />
            ) : (
              <p className="emptyState">No daily narrative for this date yet.</p>
            )}
          </div>

          <div className="dailyBriefSection">
            <h3>Hourly Reports</h3>
            {hourlyReports.length > 0 ? (
              <div className="dailyReportList">
                {hourlyReports.map((report) => (
                  <StructuredReportCard
                    key={report.id}
                    presentation={presentInsightReport(report)}
                    canShowText={canShowText}
                  />
                ))}
              </div>
            ) : (
              <p className="emptyState">No hourly reports for this date yet.</p>
            )}
          </div>

          <div className="dailyBriefSection">
            <h3>Scheduled 5h Reports</h3>
            {fiveHourReports.length > 0 ? (
              <div className="dailyReportList">
                {fiveHourReports.map((report) => (
                  <StructuredReportCard
                    key={report.id}
                    presentation={presentInsightReport(report)}
                    canShowText={canShowText}
                  />
                ))}
              </div>
            ) : (
              <p className="emptyState">No scheduled 5h reports for this date yet.</p>
            )}
          </div>
        </>
      ) : (
        <p className="emptyState">Daily brief has not connected to backend data yet.</p>
      )}
    </section>
  );
}

function Metric({
  icon,
  label,
  value,
}: {
  icon: ReactNode;
  label: string;
  value: string;
}) {
  return (
    <div className="dailyBriefMetric">
      <span className="metricIcon">{icon}</span>
      <div>
        <strong>{value}</strong>
        <span>{label}</span>
      </div>
    </div>
  );
}

function HourlyHeatmap({ metrics }: { metrics: HourlyActivityMetric[] }) {
  const visibleMetrics = metrics.length > 0 ? metrics : emptyHours();
  return (
    <div className="dailyBriefSection">
      <h3>
        <Flame aria-hidden="true" size={16} />
        Hourly Heatmap
      </h3>
      <div className="hourlyHeatmap" aria-label="Hourly activity heatmap">
        {visibleMetrics.map((metric) => (
          <div
            key={metric.hour}
            className="heatCell"
            style={{ ["--heat" as string]: String(Math.max(0.06, metric.activeRatio)) }}
            title={`${formatHour(metric.hour)} ${formatDuration(metric.activeSeconds)} · ${metric.dominantApp ?? "Unknown"}`}
          >
            <span>{formatHour(metric.hour)}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function emptyHours(): HourlyActivityMetric[] {
  return Array.from({ length: 24 }, (_, hour) => ({
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
  }));
}

function formatHour(hour: number): string {
  return `${String(hour).padStart(2, "0")}:00`;
}

function formatDuration(seconds: number): string {
  if (seconds >= 3600) {
    return `${(seconds / 3600).toFixed(1)}h`;
  }
  return `${Math.round(seconds / 60)}m`;
}

function statusClass(status?: string): string {
  if (status === "complete") return "connected";
  if (status === "error") return "offline";
  if (status === "running" || status === "pending") return "loading";
  return "idle";
}
