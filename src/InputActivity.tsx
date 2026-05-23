import { BarChart3, Clock, Keyboard, Maximize2, TableProperties } from "lucide-react";
import { Fragment, useMemo, useState } from "react";
import { feature2SampleSegments, feature2SampleSummary } from "./data/feature2Sample";
import { fetchInputSummary, fetchTextSegments } from "./lib/input";
import type { InputSummary, TextSegment } from "./types";

type DataSource = "sample" | "live";

function formatTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "Invalid";
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

export function InputActivity() {
  const [summary, setSummary] = useState<InputSummary>(feature2SampleSummary);
  const [segments, setSegments] = useState<TextSegment[]>(feature2SampleSegments);
  const [source, setSource] = useState<DataSource>("sample");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<string | null>(null);

  const largest = useMemo(() => {
    const max = Math.max(...summary.topApps.map((a) => a.charCount), 1);
    return max;
  }, [summary.topApps]);

  async function loadLive() {
    setLoading(true);
    setError(null);
    try {
      const today = new Date().toISOString().slice(0, 10);
      const [sum, segs] = await Promise.all([
        fetchInputSummary(today),
        fetchTextSegments(today),
      ]);
      setSummary(sum);
      setSegments(segs);
      setSource("live");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  function loadSample() {
    setSummary(feature2SampleSummary);
    setSegments(feature2SampleSegments);
    setSource("sample");
    setError(null);
  }

  return (
    <section className="inputActivity">
      <div className="dailyHeader">
        <div>
          <h2>Input Activity</h2>
          <p className="dailyDate">Keyboard capture via Raw Input</p>
        </div>
        <div className="actions">
          <button type="button" onClick={loadSample}>
            Sample
          </button>
          <button type="button" onClick={() => void loadLive()} disabled={loading}>
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

      {error && (
        <p className="errors" role="status">
          {error}
        </p>
      )}

      {source === "sample" && (
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
              {segments.map((seg) => (
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
                        <pre className="segmentText">{seg.textContent}</pre>
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
