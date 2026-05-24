import {
  BarChart3,
  Camera,
  Gauge,
  Keyboard,
  Layers,
  RefreshCw,
  Search
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { CollectorMonitor } from "./CollectorMonitor";
import { DailyTracking } from "./DailyTracking";
import { Dashboard } from "./Dashboard";
import { InputActivity } from "./InputActivity";
import { TimelineView } from "./TimelineView";
import { feature1SampleEvents } from "./data/feature1Sample";
import { feature2SampleSegments, feature2SampleSummary } from "./data/feature2Sample";
import { feature3SampleScreenshots, feature3SampleSummary } from "./data/feature3Sample";
import { fetchTimeEvents } from "./lib/api";
import { currentCollectorDate } from "./lib/dateQuery";
import { fetchInputSummary, fetchTextSegments } from "./lib/input";
import { fetchScreenshots, fetchScreenshotSummary } from "./lib/screenshots";
import {
  defaultLayerVisibility,
  toVisibleDashboardEvents,
  type DensityMode,
  type LayerKey,
  type LayerVisibility,
  type PrivacyMode,
  type TimelineGranularity,
  type UiSourceMode
} from "./lib/uiModel";
import type { ScreenshotMeta, ScreenshotSummary, TextSegment, TimeEvent } from "./types";
import "./styles.css";

type CollectorStatus = "sample" | "loading" | "connected" | "offline";
type ViewMode = "dashboard" | "timeline" | "daily" | "input";
type InputDataStatus = UiSourceMode;

export function App() {
  const [events, setEvents] = useState<TimeEvent[]>(feature1SampleEvents);
  const [segments, setSegments] = useState<TextSegment[]>(feature2SampleSegments);
  const [inputSummary, setInputSummary] = useState(feature2SampleSummary);
  const [screenshots, setScreenshots] = useState<ScreenshotMeta[]>(
    feature3SampleScreenshots
  );
  const [screenshotSummary, setScreenshotSummary] =
    useState<ScreenshotSummary>(feature3SampleSummary);
  const [collectorStatus, setCollectorStatus] = useState<CollectorStatus>("sample");
  const [collectorError, setCollectorError] = useState<string | null>(null);
  const [inputStatus, setInputStatus] = useState<InputDataStatus>("sample");
  const [inputLoading, setInputLoading] = useState(false);
  const [inputError, setInputError] = useState<string | null>(null);
  const [screenshotStatus, setScreenshotStatus] =
    useState<UiSourceMode>("sample");
  const [screenshotLoading, setScreenshotLoading] = useState(false);
  const [screenshotError, setScreenshotError] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>("dashboard");
  const [sourceMode, setSourceMode] = useState<UiSourceMode>("sample");
  const [privacyMode, setPrivacyMode] = useState<PrivacyMode>("redacted");
  const [densityMode, setDensityMode] = useState<DensityMode>("comfortable");
  const [granularity, setGranularity] = useState<TimelineGranularity>("event");
  const [layers, setLayers] = useState<LayerVisibility>(defaultLayerVisibility);
  const [queryDate, setQueryDate] = useState(() => currentCollectorDate());

  const visibleDashboardEvents = useMemo(
    () => toVisibleDashboardEvents(events, layers),
    [events, layers]
  );
  const visibleDashboardSegments = useMemo(
    () => (layers.input ? segments : []),
    [layers.input, segments]
  );

  useEffect(() => {
    void refreshCollector();
  }, []);

  function loadSample() {
    setEvents(feature1SampleEvents);
    setSegments(feature2SampleSegments);
    setInputSummary(feature2SampleSummary);
    setScreenshots(feature3SampleScreenshots);
    setScreenshotSummary(feature3SampleSummary);
    setSourceMode("sample");
    setInputStatus("sample");
    setScreenshotStatus("sample");
    setInputLoading(false);
    setScreenshotLoading(false);
    setCollectorStatus("sample");
    setCollectorError(null);
    setInputError(null);
    setScreenshotError(null);
  }

  async function refreshCollector(options?: {
    privacyMode?: PrivacyMode;
    layers?: LayerVisibility;
    date?: string;
  }) {
    const effectivePrivacyMode = options?.privacyMode ?? privacyMode;
    const effectiveLayers = options?.layers ?? layers;
    const effectiveDate = options?.date ?? queryDate;
    setCollectorStatus("loading");
    setInputLoading(true);
    setScreenshotLoading(true);
    setCollectorError(null);
    setInputError(null);
    setScreenshotError(null);
    try {
      const shouldLoadRawInput = effectivePrivacyMode === "raw";
      const shouldLoadScreenshotRows =
        effectivePrivacyMode === "raw" && effectiveLayers.screenshots;
      const [
        eventsResult,
        summaryResult,
        segmentsResult,
        screenshotSummaryResult,
        screenshotsResult
      ] = await Promise.allSettled([
        fetchTimeEvents(),
        fetchInputSummary(effectiveDate),
        shouldLoadRawInput ? fetchTextSegments(effectiveDate) : Promise.resolve([]),
        fetchScreenshotSummary(effectiveDate),
        shouldLoadScreenshotRows ? fetchScreenshots(effectiveDate) : Promise.resolve([])
      ]);

      if (eventsResult.status === "fulfilled") {
        setEvents(eventsResult.value);
        setSourceMode("live");
        setCollectorStatus("connected");
      } else {
        setCollectorStatus("offline");
        setCollectorError(errorMessage(eventsResult.reason));
      }

      if (
        summaryResult.status === "fulfilled" &&
        segmentsResult.status === "fulfilled"
      ) {
        setInputSummary(summaryResult.value);
        setSegments(segmentsResult.value);
        setInputStatus("live");
      } else {
        const reason =
          summaryResult.status === "rejected"
            ? summaryResult.reason
            : segmentsResult.status === "rejected"
              ? segmentsResult.reason
              : "unknown input error";
        setInputError(errorMessage(reason));
      }

      if (
        screenshotSummaryResult.status === "fulfilled" &&
        screenshotsResult.status === "fulfilled"
      ) {
        setScreenshotSummary(screenshotSummaryResult.value);
        setScreenshots(screenshotsResult.value);
        setScreenshotStatus("live");
      } else {
        const reason =
          screenshotSummaryResult.status === "rejected"
            ? screenshotSummaryResult.reason
            : screenshotsResult.status === "rejected"
              ? screenshotsResult.reason
              : "unknown screenshot error";
        setScreenshotError(errorMessage(reason));
      }
    } catch (error) {
      setCollectorStatus("offline");
      setCollectorError(errorMessage(error));
    } finally {
      setInputLoading(false);
      setScreenshotLoading(false);
    }
  }

  return (
    <main className="shell">
      <header className="topbar">
        <div>
          <p className="eyebrow">v1.1.1 prototype / Toggl-style review</p>
          <h1>Time State Recorder</h1>
          <p className="headerMeta">
            <span className={`statusPill ${collectorStatus}`}>
              {statusLabel(collectorStatus)}
            </span>
            <span>{sourceMode === "live" ? "Live collector" : "Sample workspace"}</span>
          </p>
        </div>
        <div className="controlPanel" aria-label="Dashboard controls">
          <SegmentedControl
            label="Source"
            value={sourceMode}
            options={[
              { value: "sample", label: "Sample" },
              { value: "live", label: "Live" }
            ]}
            onChange={(value) => {
              if (value === "sample") {
                loadSample();
              } else {
                void refreshCollector();
              }
            }}
          />
          <SegmentedControl
            label="Privacy"
            value={privacyMode}
            options={[
              { value: "redacted", label: "Redacted" },
              { value: "raw", label: "Raw" }
            ]}
            onChange={changePrivacyMode}
          />
          <SegmentedControl
            label="Density"
            value={densityMode}
            options={[
              { value: "comfortable", label: "Comfortable" },
              { value: "compact", label: "Compact" }
            ]}
            onChange={setDensityMode}
          />
          <label className="dateQuery">
            <span>Query Date</span>
            <input
              aria-label="Query date"
              type="date"
              value={queryDate}
              onChange={(event) => setQueryDate(event.currentTarget.value)}
            />
          </label>
          <button
            type="button"
            className="iconButton"
            onClick={() => void refreshCollector({ date: queryDate })}
            title="Query collector data"
          >
            <Search aria-hidden="true" size={18} />
            <span>Query</span>
          </button>
          <button
            type="button"
            className="iconButton"
            onClick={() => void refreshCollector({ date: queryDate })}
            title="Refresh collector data"
          >
            <RefreshCw aria-hidden="true" size={18} />
            <span>Refresh</span>
          </button>
        </div>
      </header>

      <nav className="tabBar" aria-label="View mode">
        <button
          type="button"
          className={`tab ${viewMode === "dashboard" ? "active" : ""}`}
          onClick={() => setViewMode("dashboard")}
        >
          <Gauge aria-hidden="true" size={16} />
          <span>Dashboard</span>
        </button>
        <button
          type="button"
          className={`tab ${viewMode === "timeline" ? "active" : ""}`}
          onClick={() => setViewMode("timeline")}
        >
          <BarChart3 aria-hidden="true" size={16} />
          <span>Timeline</span>
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

      <section className="filterBand" aria-label="Timeline filters">
        <SegmentedControl
          label="Timeline"
          value={granularity}
          options={[
            { value: "event", label: "Event" },
            { value: "hour", label: "Hour" }
          ]}
          onChange={setGranularity}
        />
        <div className="layerToggles" aria-label="Layer toggles">
          <Layers aria-hidden="true" size={16} />
          {layerOptions.map((layer) => (
            <button
              type="button"
              key={layer.key}
              className={`togglePill ${layers[layer.key] ? "active" : ""}`}
              onClick={() => toggleLayer(layer.key)}
            >
              {layer.label}
            </button>
          ))}
        </div>
      </section>

      {collectorError && (
        <p className="sampleNotice" role="status">
          Live collector unavailable. Keeping current data visible: {collectorError}
        </p>
      )}
      {inputError && (
        <p className="sampleNotice" role="status">
          Input layer unavailable. Keeping {inputStatus} input data visible: {inputError}
        </p>
      )}
      {screenshotError && (
        <p className="sampleNotice" role="status">
          Screenshot layer unavailable. Keeping {screenshotStatus} screenshot summary visible:{" "}
          {screenshotError}
        </p>
      )}

      {viewMode === "daily" ? (
        <DailyTracking
          date={queryDate}
          screenshots={screenshots}
          summary={screenshotSummary}
          sourceMode={screenshotStatus}
          loading={screenshotLoading}
          error={screenshotError}
          screenshotsVisible={layers.screenshots}
          privacyMode={privacyMode}
          onLoadSample={loadSample}
          onLoadLive={() => void refreshCollector()}
        />
      ) : viewMode === "input" ? (
        <InputActivity
          privacyMode={privacyMode}
          summary={inputSummary}
          segments={segments}
          sourceMode={inputStatus}
          loading={inputLoading}
          error={inputError}
          onLoadSample={loadSample}
          onLoadLive={() => void refreshCollector()}
        />
      ) : viewMode === "timeline" ? (
        <TimelineView
          events={events}
          layers={layers}
          densityMode={densityMode}
          granularity={granularity}
        />
      ) : (
        <>
          <Dashboard
            events={visibleDashboardEvents}
            segments={visibleDashboardSegments}
            layers={layers}
            densityMode={densityMode}
            privacyMode={privacyMode}
            inputSourceMode={inputStatus}
          />
          <section className="workspace dashboardMonitor">
            <CollectorMonitor />
          </section>
        </>
      )}
    </main>
  );

  function toggleLayer(layer: LayerKey) {
    setLayers((current) => {
      const next = {
        ...current,
        [layer]: !current[layer]
      };
      if (layer === "screenshots" && !next.screenshots) {
        setScreenshots([]);
      }
      if (
        layer === "screenshots" &&
        next.screenshots &&
        sourceMode === "live" &&
        privacyMode === "raw"
      ) {
        void refreshCollector({ layers: next, privacyMode, date: queryDate });
      }
      return next;
    });
  }

  function changePrivacyMode(nextMode: PrivacyMode) {
    setPrivacyMode(nextMode);
    if (nextMode === "redacted") {
      if (sourceMode === "live") {
        setSegments([]);
        setScreenshots([]);
      }
      return;
    }
    if (sourceMode === "live") {
      void refreshCollector({ privacyMode: nextMode, date: queryDate });
    }
  }
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
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

const layerOptions: { key: LayerKey; label: string }[] = [
  { key: "windows", label: "Windows" },
  { key: "lifecycle", label: "Lifecycle" },
  { key: "input", label: "Input" },
  { key: "screenshots", label: "Screenshots" }
];

function SegmentedControl<T extends string>({
  label,
  value,
  options,
  onChange
}: {
  label: string;
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
}) {
  return (
    <div className="segmentedGroup" aria-label={label}>
      <span>{label}</span>
      <div className="segmentedControl">
        {options.map((option) => (
          <button
            type="button"
            key={option.value}
            className={option.value === value ? "active" : ""}
            aria-pressed={option.value === value}
            onClick={() => onChange(option.value)}
          >
            {option.label}
          </button>
        ))}
      </div>
    </div>
  );
}
