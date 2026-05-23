import { Camera, Clock, ImageIcon } from "lucide-react";
import { useMemo, useState } from "react";
import { feature3SampleScreenshots, feature3SampleSummary } from "./data/feature3Sample";
import { fetchScreenshots, fetchScreenshotSummary } from "./lib/screenshots";
import type { ScreenshotMeta, ScreenshotSummary } from "./types";

type DataSource = "sample" | "live";

interface DailyTrackingProps {
  date: string;
}

function formatTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "Invalid";
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function toLocalDate(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, "0");
  const d = String(date.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

export function DailyTracking({ date }: DailyTrackingProps) {
  const [screenshots, setScreenshots] = useState<ScreenshotMeta[]>(feature3SampleScreenshots);
  const [summary, setSummary] = useState<ScreenshotSummary>(feature3SampleSummary);
  const [source, setSource] = useState<DataSource>("sample");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<number | null>(null);

  const grouped = useMemo(() => {
    const map = new Map<string, ScreenshotMeta[]>();
    for (const shot of screenshots) {
      const hour = new Date(shot.capturedAt).getHours();
      const label = `${String(hour).padStart(2, "0")}:00`;
      const list = map.get(label);
      if (list) {
        list.push(shot);
      } else {
        map.set(label, [shot]);
      }
    }
    return map;
  }, [screenshots]);

  async function loadLive() {
    setLoading(true);
    setError(null);
    try {
      const [shots, sum] = await Promise.all([
        fetchScreenshots(date),
        fetchScreenshotSummary(date),
      ]);
      setScreenshots(shots);
      setSummary(sum);
      setSource("live");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  function loadSample() {
    setScreenshots(feature3SampleScreenshots);
    setSummary(feature3SampleSummary);
    setSource("sample");
    setError(null);
  }

  const topAppList = summary.topApps
    .slice(0, 3)
    .map((a) => `${a.processName} (${a.count})`)
    .join(" · ");

  return (
    <section className="dailyTracking">
      <div className="dailyHeader">
        <div>
          <h2>Daily Tracking</h2>
          <p className="dailyDate">{date}</p>
        </div>
        <div className="actions">
          <button type="button" onClick={loadSample}>
            Sample
          </button>
          <button type="button" onClick={() => void loadLive()} disabled={loading}>
            <Camera aria-hidden="true" size={18} />
            <span>{loading ? "Loading..." : "Live Data"}</span>
          </button>
        </div>
      </div>

      <div className="summaryBar">
        <div className="summaryStat">
          <Camera aria-hidden="true" size={16} />
          <span>
            <strong>{summary.totalScreenshots}</strong> screenshots
          </span>
        </div>
        <div className="summaryStat">
          <Clock aria-hidden="true" size={16} />
          <span>
            <strong>{summary.hoursCovered}</strong> hours active
          </span>
        </div>
        {topAppList && (
          <div className="summaryStat">
            <span>
              Top: <strong>{topAppList}</strong>
            </span>
          </div>
        )}
      </div>

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

      <div className="timeline">
        {[...grouped].map(([hour, shots]) => (
          <div className="timelineGroup" key={hour}>
            <div className="timelineHour">
              <Clock aria-hidden="true" size={14} />
              <span>{hour}</span>
            </div>
            <div className="timelineShots">
              {shots.map((shot) => (
                <div
                  className={`timelineRow ${expanded === shot.id ? "expanded" : ""}`}
                  key={shot.id}
                  onClick={() => setExpanded(expanded === shot.id ? null : shot.id)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      setExpanded(expanded === shot.id ? null : shot.id);
                    }
                  }}
                  role="button"
                  tabIndex={0}
                >
                  <div className="timelineTime">{formatTime(shot.capturedAt)}</div>
                  <div className="timelineThumb">
                    <img
                      src={`/screenshots/${shot.filePath}`}
                      alt={`Screenshot at ${formatTime(shot.capturedAt)}`}
                      width={shot.width}
                      height={shot.height}
                      loading="lazy"
                      onError={(e) => {
                        const target = e.currentTarget;
                        target.style.display = "none";
                        const placeholder = target.nextElementSibling;
                        if (placeholder) {
                          (placeholder as HTMLElement).style.display = "flex";
                        }
                      }}
                    />
                    <div className="thumbPlaceholder" style={{ display: "none" }}>
                      <ImageIcon aria-hidden="true" size={24} />
                    </div>
                  </div>
                  <div className="timelineMeta">
                    <span className="timelineApp">{shot.processName || "Unknown"}</span>
                    <span className="timelineTitle">
                      {shot.windowTitle || ""}
                    </span>
                  </div>
                  {expanded === shot.id && (
                    <div className="timelineExpand">
                      <img
                        src={`/screenshots/${shot.filePath}`}
                        alt={`Full screenshot at ${formatTime(shot.capturedAt)}`}
                        onError={(e) => {
                          (e.currentTarget as HTMLImageElement).style.display = "none";
                        }}
                      />
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}
