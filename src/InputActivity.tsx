import { BarChart3, Clock, Keyboard, Maximize2, TableProperties } from "lucide-react";
import { Fragment, useEffect, useMemo, useState } from "react";
import {
  listSegmentApps,
  summarizeInputInsights,
  type PrivacyMode,
  type UiSourceMode
} from "./lib/uiModel";
import type { InputSummary, TextSegment } from "./types";

type AppFilter = "all" | string;

type InputActivityProps = {
  privacyMode?: PrivacyMode;
  summary: InputSummary;
  segments: TextSegment[];
  sourceMode: UiSourceMode;
  loading: boolean;
  error: string | null;
  onLoadSample: () => void;
  onLoadLive: () => void;
};

function formatTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "Invalid";
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

export function InputActivity({
  privacyMode = "redacted",
  summary,
  segments,
  sourceMode,
  loading,
  error,
  onLoadSample,
  onLoadLive
}: InputActivityProps) {
  const [expanded, setExpanded] = useState<string | null>(null);
  const [appFilter, setAppFilter] = useState<AppFilter>("all");

  const largest = useMemo(() => {
    const max = Math.max(...summary.topApps.map((a) => a.charCount), 1);
    return max;
  }, [summary.topApps]);
  const appOptions = useMemo(() => listSegmentApps(segments), [segments]);
  const visibleSegments = useMemo(
    () =>
      appFilter === "all"
        ? segments
        : segments.filter((segment) => segment.processName === appFilter),
    [appFilter, segments]
  );
  const inputInsights = useMemo(
    () => summarizeInputInsights(visibleSegments),
    [visibleSegments]
  );

  useEffect(() => {
    if (appFilter !== "all" && !appOptions.includes(appFilter)) {
      setAppFilter("all");
      setExpanded(null);
    }
  }, [appFilter, appOptions]);

  return (
    <section className="inputActivity">
      <div className="dailyHeader">
        <div>
          <h2>Input Activity</h2>
          <p className="dailyDate">
            Keyboard capture via Raw Input · {sourceMode === "live" ? "Live" : "Sample"} input data
          </p>
        </div>
        <div className="actions">
          <button type="button" onClick={() => {
            setAppFilter("all");
            setExpanded(null);
            onLoadSample();
          }}>
            Sample
          </button>
          <button type="button" onClick={onLoadLive} disabled={loading}>
            <Keyboard aria-hidden="true" size={18} />
            <span>{loading ? "Loading..." : "Live Data"}</span>
          </button>
        </div>
      </div>

      <section className="statsGrid" aria-label="Input statistics">
        <Metric label="Total Events" value={summary.totalEvents.toString()} />
        <Metric label="Keydown" value={summary.keydownCount.toString()} />
        <Metric label="Keyup" value={summary.keyupCount.toString()} />
        <Metric label="Segments" value={summary.segmentCount.toString()} />
        <Metric label="Characters" value={summary.totalChars.toString()} />
        <Metric
          label="Last Activity"
          value={summary.lastActivity ? formatTime(summary.lastActivity) : "N/A"}
        />
      </section>

      <section className="insightStrip" aria-label="Input insights">
        <Metric label="Visible Keys" value={inputInsights.totalKeys.toLocaleString()} />
        <Metric label="Corrections" value={inputInsights.correctionCount.toString()} />
        <Metric
          label="Correction Ratio"
          value={`${Math.round(inputInsights.correctionRatio * 100)}%`}
        />
        <Metric label="Input Bursts" value={inputInsights.burstCount.toString()} />
        <Metric label="Input Apps" value={inputInsights.activeAppCount.toString()} />
      </section>

      {error && (
        <p className="errors" role="status">
          {error}
        </p>
      )}

      {sourceMode === "sample" && (
        <p className="sampleNotice">
          Showing sample data. Click "Live Data" when the collector is running.
        </p>
      )}

      <section className="workspace">
        <div className="panel">
          <div className="panelHeader">
            <BarChart3 aria-hidden="true" size={20} />
            <h2>Characters by Application</h2>
          </div>
          <div className="bars">
            {summary.topApps.map((item) => (
              <div className="barRow" key={item.processName}>
                <div className="barLabel">
                  <span>{item.processName}</span>
                  <strong>{item.charCount.toLocaleString()}</strong>
                </div>
                <div className="barTrack" aria-hidden="true">
                  <div
                    className="barFill"
                    style={{ width: `${(item.charCount / largest) * 100}%` }}
                  />
                </div>
                <div className="barMeta">
                  <span>{Math.round((item.charCount / Math.max(summary.totalChars, 1)) * 100)}%</span>
                </div>
              </div>
            ))}
          </div>
        </div>

        <div className="panel">
          <div className="panelHeader">
            <Clock aria-hidden="true" size={20} />
            <h2>Session Info</h2>
          </div>
          <dl className="statusList">
            <div>
              <dt>Date</dt>
              <dd>{summary.date}</dd>
            </div>
            <div>
              <dt>Total Events</dt>
              <dd>{summary.totalEvents.toLocaleString()}</dd>
            </div>
            <div>
              <dt>Keydown / Keyup</dt>
              <dd>{summary.keydownCount} / {summary.keyupCount}</dd>
            </div>
            <div>
              <dt>Segments</dt>
              <dd>{summary.segmentCount}</dd>
            </div>
            <div>
              <dt>Last Activity</dt>
              <dd>{summary.lastActivity ? formatTime(summary.lastActivity) : "N/A"}</dd>
            </div>
          </dl>
        </div>
      </section>

      <section className="panel tablePanel">
        <div className="panelHeader">
          <TableProperties aria-hidden="true" size={20} />
          <h2>Text Segments</h2>
        </div>
        <div className="tableTools">
          <label>
            App
            <select
              value={appFilter}
              onChange={(event) => {
                setAppFilter(event.target.value);
                setExpanded(null);
              }}
            >
              <option value="all">All apps</option>
              {appOptions.map((app) => (
                <option key={app} value={app}>
                  {app}
                </option>
              ))}
            </select>
          </label>
          <span className="statusPill">
            {privacyMode === "raw" ? "Raw text visible" : "Raw text hidden"}
          </span>
        </div>
        <div className="tableWrap">
          <table>
            <thead>
              <tr>
                <th>Time</th>
                <th>App</th>
                <th>Title</th>
                <th>Keys</th>
                <th>BS</th>
                <th>Del</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {visibleSegments.map((seg) => (
                <Fragment key={seg.id}>
                  <tr
                    className={`segmentRow ${expanded === seg.id ? "expanded" : ""}`}
                    onClick={() => setExpanded(expanded === seg.id ? null : seg.id)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" || e.key === " ") {
                        e.preventDefault();
                        setExpanded(expanded === seg.id ? null : seg.id);
                      }
                    }}
                    tabIndex={0}
                    role="button"
                    aria-expanded={expanded === seg.id}
                  >
                    <td className="cellMono">{formatTime(seg.startedAt)}</td>
                    <td>{seg.processName || "Unknown"}</td>
                    <td className="cellTitle">{seg.windowTitle || ""}</td>
                    <td className="cellNum">{seg.keyCount}</td>
                    <td className="cellNum">{seg.backspaceCount}</td>
                    <td className="cellNum">{seg.deleteCount}</td>
                    <td className="cellAction">
                      <Maximize2
                        aria-hidden="true"
                        size={14}
                        style={{
                          transform: expanded === seg.id ? "rotate(180deg)" : undefined,
                          transition: "transform 0.2s",
                        }}
                      />
                    </td>
                  </tr>
                  {expanded === seg.id && (
                    <tr className="segmentExpand">
                      <td colSpan={7}>
                        {privacyMode === "raw" ? (
                          <pre className="segmentText">{seg.textContent}</pre>
                        ) : (
                          <p className="segmentText redactedText">
                            Raw text hidden in redacted mode. Keys: {seg.keyCount}, corrections:{" "}
                            {seg.backspaceCount + seg.deleteCount}.
                          </p>
                        )}
                      </td>
                    </tr>
                  )}
                </Fragment>
              ))}
            </tbody>
          </table>
        </div>
      </section>
    </section>
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
