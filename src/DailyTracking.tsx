import { Camera, Clock, ImageIcon } from "lucide-react";
import { useMemo, useState } from "react";
import type { PrivacyMode, UiSourceMode } from "./lib/uiModel";
import type { ScreenshotMeta, ScreenshotSummary } from "./types";

interface DailyTrackingProps {
  date: string;
  screenshots: ScreenshotMeta[];
  summary: ScreenshotSummary;
  sourceMode: UiSourceMode;
  loading: boolean;
  error: string | null;
  screenshotsVisible: boolean;
  privacyMode: PrivacyMode;
  onLoadSample: () => void;
  onLoadLive: () => void;
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

export function DailyTracking({
  date,
  screenshots,
  summary,
  sourceMode,
  loading,
  error,
  screenshotsVisible,
  privacyMode,
  onLoadSample,
  onLoadLive
}: DailyTrackingProps) {
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
          <button type="button" onClick={onLoadSample}>
            Sample
          </button>
          <button type="button" onClick={onLoadLive} disabled={loading}>
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

      {sourceMode === "sample" && (
        <p className="sampleNotice">
          Showing sample data. Click "Live Data" when the collector is running.
        </p>
      )}

      {!screenshotsVisible && (
        <p className="sampleNotice">
          Screenshots layer is hidden. Summary counts stay visible; screenshot evidence is not rendered.
        </p>
      )}

      {screenshotsVisible && privacyMode === "redacted" && (
        <p className="sampleNotice">
          Screenshot images are hidden in redacted mode. Switch to Raw to load visual evidence.
        </p>
      )}

      {screenshotsVisible ? (
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
                    {privacyMode === "raw" ? (
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
                    ) : null}
                    <div
                      className="thumbPlaceholder"
                      style={{ display: privacyMode === "raw" ? "none" : "flex" }}
                    >
                      <ImageIcon aria-hidden="true" size={24} />
                    </div>
                  </div>
                  <div className="timelineMeta">
                    <span className="timelineApp">{shot.processName || "Unknown"}</span>
                    <span className="timelineTitle">
                      {shot.windowTitle || ""}
                    </span>
                  </div>
                  {expanded === shot.id && privacyMode === "raw" && (
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
      ) : null}
    </section>
  );
}
