import { BarChart3, FileUp, RotateCcw, TableProperties } from "lucide-react";
import { useMemo, useRef, useState } from "react";
import { feature1SampleEvents } from "./data/feature1Sample";
import { parseTimeEventsCsv } from "./lib/csv";
import {
  summarizeByApplication,
  summarizeDurations,
  toDurationSeconds
} from "./lib/statistics";
import type { TimeEvent } from "./types";
import "./styles.css";

const sampleCsv = [
  "app,title,startedAt,endedAt",
  ...feature1SampleEvents.map((event) =>
    [event.app, event.title, event.startedAt, event.endedAt ?? ""]
      .map((cell) => `"${cell.replaceAll('"', '""')}"`)
      .join(",")
  )
].join("\n");

export function App() {
  const [events, setEvents] = useState<TimeEvent[]>(feature1SampleEvents);
  const [csvText, setCsvText] = useState(sampleCsv);
  const [errors, setErrors] = useState<string[]>([]);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const durationSummary = useMemo(() => summarizeDurations(events), [events]);
  const appSummary = useMemo(() => summarizeByApplication(events), [events]);

  function loadSample() {
    setCsvText(sampleCsv);
    setEvents(feature1SampleEvents);
    setErrors([]);
  }

  function importCsv(nextText = csvText) {
    const parsed = parseTimeEventsCsv(nextText);
    setEvents(parsed.events);
    setErrors(parsed.errors);
  }

  async function handleFile(file: File | undefined) {
    if (!file) {
      return;
    }
    const text = await file.text();
    setCsvText(text);
    importCsv(text);
  }

  const largest = Math.max(...appSummary.map((item) => item.totalSeconds), 1);

  return (
    <main className="shell">
      <header className="topbar">
        <div>
          <p className="eyebrow">MVP / Feature 1</p>
          <h1>Time State Recorder</h1>
        </div>
        <div className="actions" aria-label="Dataset actions">
          <button type="button" onClick={loadSample} title="Load feature1 sample">
            <RotateCcw aria-hidden="true" size={18} />
            <span>Feature1 Sample</span>
          </button>
          <button
            type="button"
            onClick={() => fileInputRef.current?.click()}
            title="Import CSV"
          >
            <FileUp aria-hidden="true" size={18} />
            <span>Import CSV</span>
          </button>
          <input
            ref={fileInputRef}
            className="visuallyHidden"
            type="file"
            accept=".csv,text/csv"
            tabIndex={-1}
            aria-label="CSV file"
            onChange={(event) => void handleFile(event.target.files?.[0])}
          />
        </div>
      </header>

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
            <FileUp aria-hidden="true" size={20} />
            <h2>CSV Input</h2>
          </div>
          <textarea
            value={csvText}
            onChange={(event) => setCsvText(event.target.value)}
            spellCheck={false}
            aria-label="CSV input"
          />
          <div className="inlineActions">
            <button type="button" onClick={() => importCsv()}>
              <TableProperties aria-hidden="true" size={18} />
              <span>Calculate</span>
            </button>
          </div>
          {errors.length > 0 && (
            <ul className="errors" aria-label="CSV errors">
              {errors.map((error) => (
                <li key={error}>{error}</li>
              ))}
            </ul>
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
                  <td>{event.endedAt ? formatTime(event.endedAt) : "Duration only"}</td>
                  <td>{formatSeconds(toDurationSeconds(event))}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>
    </main>
  );
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
