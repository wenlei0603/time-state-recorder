import { BarChart3, Camera, Keyboard, RefreshCw, RotateCcw, Server, TableProperties } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { DailyTracking } from "./DailyTracking";
import { InputActivity } from "./InputActivity";
import { feature1SampleEvents } from "./data/feature1Sample";
import { fetchTimeEvents } from "./lib/api";
import {
  summarizeByApplication,
  summarizeDurations,
  toDurationSeconds
} from "./lib/statistics";
import type { TimeEvent } from "./types";
import "./styles.css";

type CollectorStatus = "sample" | "loading" | "connected" | "offline";
type ViewMode = "stats" | "daily" | "input";

export function App() {
  const [events, setEvents] = useState<TimeEvent[]>(feature1SampleEvents);
  const [collectorStatus, setCollectorStatus] = useState<CollectorStatus>("sample");
  const [collectorError, setCollectorError] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>("stats");

  const durationSummary = useMemo(() => summarizeDurations(events), [events]);
  const appSummary = useMemo(() => summarizeByApplication(events), [events]);

  useEffect(() => {
    void refreshCollector();
  }, []);

  function loadSample() {
    setEvents(feature1SampleEvents);
    setCollectorStatus("sample");
    setCollectorError(null);
  }

  async function refreshCollector() {
    setCollectorStatus("loading");
    setCollectorError(null);
    try {
      const nextEvents = await fetchTimeEvents();
      setEvents(nextEvents);
      setCollectorStatus("connected");
    } catch (error) {
      setCollectorStatus("offline");
      setCollectorError(error instanceof Error ? error.message : String(error));
    }
  }

  const largest = Math.max(...appSummary.map((item) => item.totalSeconds), 1);
  const today = new Date().toISOString().slice(0, 10);

  return (
    <main className="shell">
      <header className="topbar">
        <div>
          <p className="eyebrow">MVP / Features 1 &amp; 3</p>
          <h1>Time State Recorder</h1>
        </div>
        <div className="actions" aria-label="Dataset actions">
          <button type="button" onClick={loadSample} title="Load feature1 sample">
            <RotateCcw aria-hidden="true" size={18} />
            <span>Feature1 Sample</span>
          </button>
          <button type="button" onClick={() => void refreshCollector()}>
            <RefreshCw aria-hidden="true" size={18} />
            <span>Collector Data</span>
          </button>
        </div>
      </header>

      <nav className="tabBar" aria-label="View mode">
        <button
          type="button"
          className={`tab ${viewMode === "stats" ? "active" : ""}`}
          onClick={() => setViewMode("stats")}
        >
          <BarChart3 aria-hidden="true" size={16} />
          <span>Statistics</span>
        </button>
        <button
          type="button"
          className={`tab ${viewMode === "daily" ? "active" : ""}`}
          onClick={() => setViewMode("daily")}
        >
          <Camera aria-hidden="true" size={16} />
          <span>Daily Tracking</span>
        </button>
        <button
          type="button"
          className={`tab ${viewMode === "input" ? "active" : ""}`}
          onClick={() => setViewMode("input")}
        >
          <Keyboard aria-hidden="true" size={16} />
          <span>Input Activity</span>
        </button>
      </nav>

      {viewMode === "daily" ? (
        <DailyTracking date={today} />
      ) : viewMode === "input" ? (
        <InputActivity />
      ) : (
        <>
          <section className="statsGrid" aria-label="Descriptive statistics">
            <Metric label="Count" value={durationSummary.count.toString()} />
            <Metric label="Total" value={formatMetricSeconds(durationSummary.total)} />
            <Metric label="Mean" value={formatMetricSeconds(durationSummary.mean)} />
            <Metric label="Median" value={formatMetricSeconds(durationSummary.median)} />
            <Metric label="Std dev" value={formatMetricSeconds(durationSummary.standardDeviation)} />
            <Metric label="Min / Max" value={`${formatMetricSeconds(durationSummary.min)} / ${formatMetricSeconds(durationSummary.max)}`} />
            <Metric label="Q1 / Q3" value={`${formatMetricSeconds(durationSummary.q1)} / ${formatMetricSeconds(durationSummary.q3)}`} />
          </section>

          <section className="workspace">
            <div className="panel">
              <div className="panelHeader">
                <BarChart3 aria-hidden="true" size={20} />
                <h2>Application Time</h2>
              </div>
              <div className="bars">
                {appSummary.map((item) => (
                  <div className="barRow" key={item.app}>
                    <div className="barLabel">
                      <span>{item.app}</span>
                      <strong>{formatSeconds(item.totalSeconds)}</strong>
                    </div>
                    <div className="barTrack" aria-hidden="true">
                      <div
                        className="barFill"
                        style={{ width: `${(item.totalSeconds / largest) * 100}%` }}
                      />
                    </div>
                    <div className="barMeta">
                      <span>{item.eventCount} events</span>
                      <span>{Math.round(item.share * 100)}%</span>
                    </div>
                  </div>
                ))}
              </div>
            </div>

            <div className="panel">
              <div className="panelHeader">
                <Server aria-hidden="true" size={20} />
                <h2>Collector Connection</h2>
              </div>
              <dl className="statusList">
                <div>
                  <dt>Status</dt>
                  <dd>
                    <span className={`statusPill ${collectorStatus}`}>
                      {statusLabel(collectorStatus)}
                    </span>
                  </dd>
                </div>
                <div>
                  <dt>Endpoint</dt>
                  <dd className="codeLine">/api/time-events</dd>
                </div>
                <div>
                  <dt>Rows</dt>
                  <dd>{events.length}</dd>
                </div>
              </dl>
              <div className="inlineActions">
                <button type="button" onClick={() => void refreshCollector()}>
                  <RefreshCw aria-hidden="true" size={18} />
                  <span>Refresh Collector</span>
                </button>
              </div>
              {collectorError && (
                <p className="errors" role="status">
                  {collectorError}
                </p>
              )}
            </div>
          </section>

          <section className="panel tablePanel">
            <div className="panelHeader">
              <TableProperties aria-hidden="true" size={20} />
              <h2>Event Rows</h2>
            </div>
            <div className="tableWrap">
              <table>
                <thead>
                  <tr>
                    <th>App</th>
                    <th>Title</th>
                    <th>Started</th>
                    <th>Ended</th>
                    <th>Duration</th>
                  </tr>
                </thead>
                <tbody>
                  {events.map((event) => (
                    <tr key={event.id}>
                      <td>{event.app}</td>
                      <td>{event.title}</td>
                      <td>{formatTime(event.startedAt)}</td>
                      <td>{event.endedAt ? formatTime(event.endedAt) : "Open"}</td>
                      <td>{formatSeconds(toDurationSeconds(event))}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>
        </>
      )}
    </main>
  );
}

function statusLabel(status: CollectorStatus): string {
  switch (status) {
    case "connected":
      return "Connected";
    case "loading":
      return "Loading";
    case "offline":
      return "Offline";
    case "sample":
      return "Sample";
  }
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <article className="metric">
      <span>{label}</span>
      <strong>{value}</strong>
    </article>
  );
}

function formatSeconds(value: number): string {
  if (!Number.isFinite(value)) {
    return "0s";
  }

  const rounded = Math.round(value);
  const minutes = Math.floor(rounded / 60);
  const seconds = rounded % 60;

  if (minutes === 0) {
    return `${seconds}s`;
  }

  return `${minutes}m ${seconds.toString().padStart(2, "0")}s`;
}

function formatMetricSeconds(value: number): string {
  const rounded = Math.round(value);
  const minutes = Math.floor(rounded / 60);
  const seconds = rounded % 60;

  if (minutes === 0) {
    return `${seconds}s`;
  }

  if (seconds === 0) {
    return `${minutes}m`;
  }

  return `${minutes}m ${seconds}s`;
}

function formatTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return "Invalid";
  }
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}
