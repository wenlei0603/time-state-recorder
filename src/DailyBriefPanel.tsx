import { BarChart3, Clock3, FileText, Flame, Layers } from "lucide-react";
import type { ReactNode } from "react";
import type { DailyBriefResponse, HourlyActivityMetric, InsightReport } from "./types";
import type { PrivacyMode, UiSourceMode } from "./lib/uiModel";

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
  const reports = response?.fiveHourReports ?? [];

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
            <Metric icon={<Layers size={17} />} label="reports" value={`${reports.length} reports`} />
            <Metric icon={<BarChart3 size={17} />} label="switches" value={`${stats.switchCount} switches`} />
            <Metric icon={<FileText size={17} />} label="input" value={`${stats.inputChars} chars`} />
          </div>

          <HourlyHeatmap metrics={response?.hourlyMetrics ?? []} />

          <div className="dailyBriefSection">
            <h3>Past Comparison</h3>
            <p>{response?.comparison.explanation ?? "No baseline comparison yet."}</p>
          </div>

          <div className="dailyBriefSection">
            <h3>5h Reports</h3>
            {reports.length > 0 ? (
              <div className="dailyReportList">
                {reports.map((report) => (
                  <ReportRow key={report.id} report={report} canShowText={canShowText} />
                ))}
              </div>
            ) : (
              <p className="emptyState">No 5h reports for this date yet.</p>
            )}
          </div>

          <div className="dailyBriefSection">
            <h3>Daily Action Trajectory</h3>
            {canShowText && brief ? (
              <>
                <p className="dailyBriefLead">{brief.dailySummaryText}</p>
                <p>{brief.actionTrajectory}</p>
              </>
            ) : (
              <p className="redactedText insightRedacted">
                Daily narrative generated. Text is hidden in redacted mode.
              </p>
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

function ReportRow({
  report,
  canShowText,
}: {
  report: InsightReport;
  canShowText: boolean;
}) {
  return (
    <article className="dailyReportRow">
      <div>
        <strong>{formatRange(report.periodStart, report.periodEnd)}</strong>
        <span>{report.evidenceCount} windows · {report.modelProvider}</span>
      </div>
      {canShowText ? (
        <p>{report.summaryText}</p>
      ) : (
        <p className="redactedText">Report text hidden in redacted mode.</p>
      )}
    </article>
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

function formatRange(start: string, end: string): string {
  return `${formatTime(start)} - ${formatTime(end)}`;
}

function formatTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "Invalid";
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
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
