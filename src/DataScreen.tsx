import type { EChartsOption } from "echarts";
import { BarChart3, RefreshCw, Route, Search, TrendingUp } from "lucide-react";
import { useMemo, useState } from "react";
import { EChartPanel } from "./EChartPanel";
import { fetchDailyBrief } from "./lib/dailyBrief";
import { fetchVisualWindowSummaries } from "./lib/insights";
import {
  buildDataScreenModel,
  buildDateRange,
  type DataScreenModel,
  type DataScreenPeriod,
} from "./lib/dataScreenModel";
import { formatOwnerClock } from "./lib/dateQuery";

type Fetcher = (input: string) => Promise<Pick<Response, "ok" | "status" | "statusText" | "json">>;

type DataScreenProps = {
  anchorDate: string;
  fetcher?: Fetcher;
};

export function DataScreen({ anchorDate, fetcher = fetch }: DataScreenProps) {
  const [period, setPeriod] = useState<DataScreenPeriod>("day");
  const [model, setModel] = useState<DataScreenModel | undefined>();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const chartOptions = useMemo(() => (model ? buildChartOptions(model) : undefined), [model]);

  async function updateScreen() {
    setLoading(true);
    setError(null);
    const dateRange = buildDateRange(anchorDate, period);
    const dayResults = await Promise.allSettled(
      dateRange.map((date) => fetchDailyBrief(date, fetcher)),
    );
    const visualResults = await Promise.allSettled(
      dateRange.map(async (date) => ({
        date,
        summaries: await fetchVisualWindowSummaries(date, fetcher),
      })),
    );

    const days = dayResults.flatMap((result) =>
      result.status === "fulfilled" ? [result.value] : [],
    );
    const visualWindowsByDate = Object.fromEntries(
      visualResults.flatMap((result) =>
        result.status === "fulfilled" ? [[result.value.date, result.value.summaries]] : [],
      ),
    );
    const failedCount =
      dayResults.filter((result) => result.status === "rejected").length +
      visualResults.filter((result) => result.status === "rejected").length;

    setModel(
      buildDataScreenModel({
        anchorDate,
        period,
        days,
        visualWindowsByDate,
      }),
    );
    setError(failedCount > 0 ? `${failedCount} dashboard requests failed.` : null);
    setLoading(false);
  }

  return (
    <section className="dataScreen" aria-label="Data Screen Dashboard">
      <div className="dataScreenHeader">
        <div>
          <p className="eyebrow">Data screen</p>
          <h2>Data Screen Dashboard</h2>
          <p>
            {anchorDate} - manual refresh - {model ? `${model.availableDayCount}/${model.dateRange.length} days loaded` : "no snapshot"}
          </p>
        </div>
        <div className="dataScreenControls" aria-label="Data screen controls">
          <SegmentedPeriod value={period} onChange={setPeriod} />
          <button
            type="button"
            className="iconButton"
            onClick={() => void updateScreen()}
            disabled={loading}
          >
            {loading ? <RefreshCw aria-hidden="true" size={18} /> : <Search aria-hidden="true" size={18} />}
            <span>{loading ? "Updating" : "Update screen"}</span>
          </button>
        </div>
      </div>

      {error ? (
        <p className="sampleNotice" role="status">
          {error}
        </p>
      ) : null}

      {!model || !chartOptions ? (
        <div className="emptyDashboardSnapshot">
          <BarChart3 aria-hidden="true" size={32} />
          <p>No dashboard snapshot loaded.</p>
        </div>
      ) : (
        <>
          <section className="dataScreenKpis" aria-label="Data screen metrics">
            {Object.values(model.kpis).map((kpi) => (
              <article className="dataScreenKpi" key={kpi.label}>
                <span>{kpi.label}</span>
                <strong>{kpi.value}</strong>
                <small>{kpi.detail}</small>
              </article>
            ))}
          </section>

          <section className="dataScreenChartGrid" aria-label="Dashboard charts">
            <EChartPanel title="Activity Trend" option={chartOptions.activityTrend} />
            <EChartPanel title="Category Mix" option={chartOptions.categoryMix} />
            <EChartPanel title="Hourly Heatmap" option={chartOptions.hourlyHeatmap} />
          </section>

          <section className="dataScreenSignals" aria-label="Dashboard signals">
            <SignalList title="Top Projects" items={model.projectRank} />
            <SignalList title="Top Apps" items={model.appRank} formatValue={formatSeconds} />
            <SignalList title="Report Terms" items={model.textSignals.topTerms} />
          </section>

          <section className="trajectoryPanel" aria-label="5-minute trajectory">
            <div className="panelHeader">
              <Route aria-hidden="true" size={20} />
              <h3>5min Trajectory</h3>
              <span>{model.trajectory.length} windows</span>
            </div>
            {model.trajectory.length === 0 ? (
              <p className="emptyState">No visual-window trajectory loaded for this period.</p>
            ) : (
              <div className="trajectoryLane">
                {model.trajectory.slice(0, 36).map((point) => (
                  <article className="trajectoryNode" key={point.id}>
                    <time>{formatOwnerClock(point.startAt)}</time>
                    <strong>{point.intent || point.primaryActivity}</strong>
                    <span>{point.projectHints.slice(0, 2).join(" / ") || point.primaryActivity}</span>
                    <small>
                      switching {point.switchingLevel} - loafing {point.loafingLevel}
                    </small>
                  </article>
                ))}
              </div>
            )}
          </section>

          <section className="textSignalPanel" aria-label="Report text signals">
            <div className="panelHeader">
              <TrendingUp aria-hidden="true" size={20} />
              <h3>Report Text Signals</h3>
            </div>
            {model.textSignals.uncertainLines.length === 0 ? (
              <p className="emptyState">No uncertainty or risk lines detected in loaded reports.</p>
            ) : (
              <ul className="signalBullets">
                {model.textSignals.uncertainLines.map((line) => (
                  <li key={line}>{line}</li>
                ))}
              </ul>
            )}
          </section>
        </>
      )}
    </section>
  );
}

function SegmentedPeriod({
  value,
  onChange,
}: {
  value: DataScreenPeriod;
  onChange: (value: DataScreenPeriod) => void;
}) {
  return (
    <div className="segmentedControl" aria-label="Data screen period">
      {(["day", "week", "month"] as const).map((period) => (
        <button
          type="button"
          key={period}
          className={value === period ? "active" : ""}
          onClick={() => onChange(period)}
        >
          {period[0].toUpperCase() + period.slice(1)}
        </button>
      ))}
    </div>
  );
}

function SignalList({
  title,
  items,
  formatValue = (value: number) => value.toLocaleString(),
}: {
  title: string;
  items: { label: string; count: number }[];
  formatValue?: (value: number) => string;
}) {
  const max = Math.max(...items.map((item) => item.count), 1);
  return (
    <section className="signalList" aria-label={title}>
      <div className="panelHeader">
        <h3>{title}</h3>
      </div>
      {items.length === 0 ? (
        <p className="emptyState">No signal loaded.</p>
      ) : (
        <div className="rankRows">
          {items.slice(0, 8).map((item) => (
            <div className="rankRow" key={item.label}>
              <span>{item.label}</span>
              <div className="rankTrack">
                <span style={{ width: `${Math.max(6, (item.count / max) * 100)}%` }} />
              </div>
              <strong>{formatValue(item.count)}</strong>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

function buildChartOptions(model: DataScreenModel): Record<string, EChartsOption> {
  return {
    activityTrend: {
      tooltip: { trigger: "axis" },
      grid: { left: 44, right: 18, top: 24, bottom: 36 },
      xAxis: {
        type: "category",
        data: model.dailyTrend.map((point) => point.date.slice(5)),
      },
      yAxis: { type: "value" },
      series: [
        {
          type: "bar",
          name: "Active hours",
          data: model.dailyTrend.map((point) => round(point.activeHours)),
          itemStyle: { color: "#2f6f73" },
        },
        {
          type: "line",
          name: "Reports",
          data: model.dailyTrend.map((point) => point.reportCount),
          smooth: true,
          symbolSize: 6,
          lineStyle: { color: "#c47f3f", width: 2 },
          itemStyle: { color: "#c47f3f" },
        },
      ],
    },
    categoryMix: {
      tooltip: { trigger: "axis" },
      grid: { left: 110, right: 18, top: 24, bottom: 28 },
      xAxis: { type: "value" },
      yAxis: {
        type: "category",
        data: model.categoryShare.map((item) => item.category),
      },
      series: [
        {
          type: "bar",
          data: model.categoryShare.map((item) => item.count),
          itemStyle: { color: "#4f6f52" },
        },
      ],
    },
    hourlyHeatmap: {
      tooltip: { trigger: "axis" },
      grid: { left: 44, right: 18, top: 24, bottom: 36 },
      xAxis: {
        type: "category",
        data: model.hourlyHeatmap.map((metric) => `${String(metric.hour).padStart(2, "0")}:00`),
      },
      yAxis: { type: "value" },
      series: [
        {
          type: "bar",
          data: model.hourlyHeatmap.map((metric) => Math.round(metric.activeSeconds / 60)),
          itemStyle: { color: "#385f8f" },
        },
      ],
    },
  };
}

function formatSeconds(seconds: number): string {
  if (seconds >= 3600) {
    return `${(seconds / 3600).toFixed(1)}h`;
  }
  return `${Math.round(seconds / 60)}m`;
}

function round(value: number): number {
  return Math.round(value * 10) / 10;
}
