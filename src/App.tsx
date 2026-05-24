import {
  BarChart3,
  Camera,
  Gauge,
  Keyboard,
  Layers,
  RefreshCw
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { CollectorMonitor } from "./CollectorMonitor";
import { DailyTracking } from "./DailyTracking";
import { Dashboard } from "./Dashboard";
import { InputActivity } from "./InputActivity";
import { TimelineView } from "./TimelineView";
import { feature1SampleEvents } from "./data/feature1Sample";
import { feature2SampleSegments } from "./data/feature2Sample";
import { fetchTimeEvents } from "./lib/api";
import {
  defaultLayerVisibility,
  type DensityMode,
  type LayerKey,
  type LayerVisibility,
  type PrivacyMode,
  type TimelineGranularity,
  type UiSourceMode
} from "./lib/uiModel";
import type { TextSegment, TimeEvent } from "./types";
import "./styles.css";

type CollectorStatus = "sample" | "loading" | "connected" | "offline";
type ViewMode = "dashboard" | "timeline" | "daily" | "input";

export function App() {
  const [events, setEvents] = useState<TimeEvent[]>(feature1SampleEvents);
  const [segments, setSegments] = useState<TextSegment[]>(feature2SampleSegments);
  const [collectorStatus, setCollectorStatus] = useState<CollectorStatus>("sample");
  const [collectorError, setCollectorError] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>("dashboard");
  const [sourceMode, setSourceMode] = useState<UiSourceMode>("sample");
  const [privacyMode, setPrivacyMode] = useState<PrivacyMode>("redacted");
  const [densityMode, setDensityMode] = useState<DensityMode>("comfortable");
  const [granularity, setGranularity] = useState<TimelineGranularity>("event");
  const [layers, setLayers] = useState<LayerVisibility>(defaultLayerVisibility);

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
    setSourceMode("sample");
    setCollectorStatus("sample");
    setCollectorError(null);
  }

  async function refreshCollector() {
    setCollectorStatus("loading");
    setCollectorError(null);
    try {
      const nextEvents = await fetchTimeEvents();
      setEvents(nextEvents);
      setSourceMode("live");
      setCollectorStatus("connected");
    } catch (error) {
      setCollectorStatus("offline");
      setCollectorError(error instanceof Error ? error.message : String(error));
    }
  }

  const today = new Date().toISOString().slice(0, 10);

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
            onChange={setPrivacyMode}
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
          <button
            type="button"
            className="iconButton"
            onClick={() => void refreshCollector()}
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

      {viewMode === "daily" ? (
        <DailyTracking date={today} />
      ) : viewMode === "input" ? (
        <InputActivity />
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
            events={events}
            segments={visibleDashboardSegments}
            layers={layers}
            densityMode={densityMode}
            privacyMode={privacyMode}
          />
          <section className="workspace dashboardMonitor">
            <CollectorMonitor />
          </section>
        </>
      )}
    </main>
  );

  function toggleLayer(layer: LayerKey) {
    setLayers((current) => ({
      ...current,
      [layer]: !current[layer]
    }));
  }
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
