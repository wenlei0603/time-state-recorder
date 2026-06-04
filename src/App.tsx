import {
  Activity,
  BarChart3,
  Camera,
  Gauge,
  Keyboard,
  Layers,
  RefreshCw,
  Search
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { ActivityReview } from "./ActivityReview";
import { CollectorMonitor } from "./CollectorMonitor";
import { DailyBriefPanel } from "./DailyBriefPanel";
import { DailyTracking } from "./DailyTracking";
import { Dashboard } from "./Dashboard";
import { InputActivity } from "./InputActivity";
import { InsightFeedback } from "./InsightFeedback";
import { TimelineView } from "./TimelineView";
import { TodayFlowBoard } from "./TodayFlowBoard";
import { activitySampleBuckets } from "./data/activitySample";
import { feature1SampleEvents } from "./data/feature1Sample";
import { feature2SampleSegments, feature2SampleSummary } from "./data/feature2Sample";
import { feature3SampleScreenshots, feature3SampleSummary } from "./data/feature3Sample";
import { visualSummarySample } from "./data/visualSummarySample";
import { fetchActivityBuckets, fetchTimeEvents } from "./lib/api";
import { fetchDailyBrief } from "./lib/dailyBrief";
import { currentCollectorDate } from "./lib/dateQuery";
import { fetchCollectorHealth } from "./lib/health";
import { fetchInputSummary, fetchTextSegments } from "./lib/input";
import { fetchAnalysisStatus, fetchInsightReports } from "./lib/insights";
import {
  analyzeScreenshot,
  fetchScreenshots,
  fetchScreenshotSummary,
  fetchVisualSummaries
} from "./lib/screenshots";
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
import type {
  ActivityBucket,
  AnalysisStatus,
  CollectorHealth,
  DailyBriefResponse,
  InsightReport,
  ScreenshotMeta,
  ScreenshotSummary,
  TextSegment,
  TimeEvent,
  VisualSummary
} from "./types";
import "./styles.css";

type CollectorStatus = "sample" | "loading" | "connected" | "offline";
type ViewMode = "today" | "activity" | "dashboard" | "timeline" | "daily" | "input";
type InputDataStatus = UiSourceMode;
type RefreshContext = {
  privacyMode: PrivacyMode;
  layers: LayerVisibility;
  viewMode: ViewMode;
  date: string;
};

export function App() {
  const [events, setEvents] = useState<TimeEvent[]>(feature1SampleEvents);
  const [activityBuckets, setActivityBuckets] =
    useState<ActivityBucket[]>(activitySampleBuckets);
  const [visualSummaries, setVisualSummaries] =
    useState<VisualSummary[]>(visualSummarySample);
  const [segments, setSegments] = useState<TextSegment[]>(feature2SampleSegments);
  const [inputSummary, setInputSummary] = useState(feature2SampleSummary);
  const [screenshots, setScreenshots] = useState<ScreenshotMeta[]>(
    feature3SampleScreenshots
  );
  const [screenshotSummary, setScreenshotSummary] =
    useState<ScreenshotSummary>(feature3SampleSummary);
  const [analysisStatus, setAnalysisStatus] = useState<AnalysisStatus | undefined>(
    undefined
  );
  const [insightReports, setInsightReports] = useState<InsightReport[]>([]);
  const [dailyBriefResponse, setDailyBriefResponse] =
    useState<DailyBriefResponse | undefined>(undefined);
  const [health, setHealth] = useState<CollectorHealth | undefined>(undefined);
  const [collectorStatus, setCollectorStatus] = useState<CollectorStatus>("sample");
  const [collectorError, setCollectorError] = useState<string | null>(null);
  const [activityStatus, setActivityStatus] = useState<UiSourceMode>("sample");
  const [activityLoading, setActivityLoading] = useState(false);
  const [activityError, setActivityError] = useState<string | null>(null);
  const [inputStatus, setInputStatus] = useState<InputDataStatus>("sample");
  const [inputLoading, setInputLoading] = useState(false);
  const [inputError, setInputError] = useState<string | null>(null);
  const [screenshotStatus, setScreenshotStatus] =
    useState<UiSourceMode>("sample");
  const [screenshotLoading, setScreenshotLoading] = useState(false);
  const [analysisLoading, setAnalysisLoading] = useState(false);
  const [screenshotError, setScreenshotError] = useState<string | null>(null);
  const [analysisError, setAnalysisError] = useState<string | null>(null);
  const [dailyBriefError, setDailyBriefError] = useState<string | null>(null);
  const [visualSummaryError, setVisualSummaryError] = useState<string | null>(null);
  const [analyzingScreenshotId, setAnalyzingScreenshotId] = useState<number | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>("today");
  const [sourceMode, setSourceMode] = useState<UiSourceMode>("sample");
  const [privacyMode, setPrivacyMode] = useState<PrivacyMode>("redacted");
  const [densityMode, setDensityMode] = useState<DensityMode>("comfortable");
  const [granularity, setGranularity] = useState<TimelineGranularity>("event");
  const [layers, setLayers] = useState<LayerVisibility>(defaultLayerVisibility);
  const [queryDate, setQueryDate] = useState(() => currentCollectorDate());
  const latestRefreshContext = useRef<RefreshContext>({
    privacyMode,
    layers,
    viewMode,
    date: queryDate
  });
  const collectorRequestGeneration = useRef(0);
  latestRefreshContext.current = { privacyMode, layers, viewMode, date: queryDate };

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

  useEffect(() => {
    if (sourceMode !== "live") {
      return;
    }
    const intervalId = window.setInterval(() => {
      void refreshAnalysisFeedback(
        collectorRequestGeneration.current,
        latestRefreshContext.current.date
      );
    }, 15_000);
    return () => window.clearInterval(intervalId);
  }, [sourceMode]);

  function loadSample() {
    collectorRequestGeneration.current += 1;
    setEvents(feature1SampleEvents);
    setActivityBuckets(activitySampleBuckets);
    setVisualSummaries(visualSummarySample);
    setSegments(feature2SampleSegments);
    setInputSummary(feature2SampleSummary);
    setScreenshots(feature3SampleScreenshots);
    setScreenshotSummary(feature3SampleSummary);
    setAnalysisStatus(undefined);
    setInsightReports([]);
    setDailyBriefResponse(undefined);
    setHealth(undefined);
    setSourceMode("sample");
    setActivityStatus("sample");
    setInputStatus("sample");
    setScreenshotStatus("sample");
    setActivityLoading(false);
    setInputLoading(false);
    setScreenshotLoading(false);
    setAnalysisLoading(false);
    setCollectorStatus("sample");
    setCollectorError(null);
    setActivityError(null);
    setInputError(null);
    setScreenshotError(null);
    setAnalysisError(null);
    setDailyBriefError(null);
    setVisualSummaryError(null);
    setAnalyzingScreenshotId(null);
  }

  async function refreshCollector(options?: {
    privacyMode?: PrivacyMode;
    layers?: LayerVisibility;
    viewMode?: ViewMode;
    date?: string;
  }) {
    const requestGeneration = collectorRequestGeneration.current + 1;
    collectorRequestGeneration.current = requestGeneration;
    const currentContext = latestRefreshContext.current;
    const effectivePrivacyMode = options?.privacyMode ?? currentContext.privacyMode;
    const effectiveLayers = options?.layers ?? currentContext.layers;
    const effectiveViewMode = options?.viewMode ?? currentContext.viewMode;
    const effectiveDate = options?.date ?? currentContext.date;
    setCollectorStatus("loading");
    setActivityLoading(true);
    setInputLoading(true);
    setScreenshotLoading(true);
    setAnalysisLoading(true);
    setCollectorError(null);
    setActivityError(null);
    setInputError(null);
    setScreenshotError(null);
    setAnalysisError(null);
    setDailyBriefError(null);
    setVisualSummaryError(null);
    const isCurrentRequest = () =>
      requestGeneration === collectorRequestGeneration.current;
    const isDateCurrent = () =>
      latestRefreshContext.current.date === effectiveDate;
    const shouldLoadRawInput =
      effectivePrivacyMode === "raw" && effectiveViewMode === "input";
    const shouldLoadScreenshotRows =
      effectivePrivacyMode === "raw" &&
      (effectiveViewMode === "daily" || effectiveViewMode === "today") &&
      effectiveLayers.screenshots;

    try {
      void fetchTimeEvents()
        .then((eventRows) => {
          if (!isCurrentRequest()) {
            return;
          }
          setEvents(eventRows);
          setSourceMode("live");
          setCollectorStatus("connected");
        })
        .catch((error) => {
          if (!isCurrentRequest()) {
            return;
          }
          setCollectorStatus("offline");
          setCollectorError(errorMessage(error));
        });

      void fetchCollectorHealth()
        .then((collectorHealth) => {
          if (isCurrentRequest()) {
            setHealth(collectorHealth);
          }
        })
        .catch(() => {
          if (isCurrentRequest()) {
            setHealth(undefined);
          }
        });

      void refreshAnalysisFeedback(requestGeneration, effectiveDate);

      void fetchActivityBuckets(effectiveDate, 180)
        .then((result) => {
          if (!isCurrentRequest() || !isDateCurrent()) {
            return;
          }
          setActivityBuckets(result.buckets);
          setActivityStatus("live");
        })
        .catch((error) => {
          if (isCurrentRequest()) {
            setActivityError(errorMessage(error));
          }
        })
        .finally(() => {
          if (isCurrentRequest()) {
            setActivityLoading(false);
          }
        });

      void Promise.allSettled([
        fetchInputSummary(effectiveDate),
        shouldLoadRawInput ? fetchTextSegments(effectiveDate) : Promise.resolve([])
      ])
        .then(([summaryResult, segmentsResult]) => {
          if (!isCurrentRequest()) {
            return;
          }
          const latestContext = latestRefreshContext.current;
          const rawInputStillAllowed =
            shouldLoadRawInput &&
            latestContext.privacyMode === "raw" &&
            latestContext.viewMode === "input" &&
            latestContext.date === effectiveDate;
          if (
            summaryResult.status === "fulfilled" &&
            segmentsResult.status === "fulfilled"
          ) {
            if (latestContext.date === effectiveDate) {
              setInputSummary(summaryResult.value);
              setSegments(rawInputStillAllowed ? segmentsResult.value : []);
              setInputStatus("live");
            }
          } else {
            const reason =
              summaryResult.status === "rejected"
                ? summaryResult.reason
                : segmentsResult.status === "rejected"
                  ? segmentsResult.reason
                  : "unknown input error";
            setInputError(errorMessage(reason));
          }
        })
        .finally(() => {
          if (isCurrentRequest()) {
            setInputLoading(false);
          }
        });

      void Promise.allSettled([
        fetchScreenshotSummary(effectiveDate),
        shouldLoadScreenshotRows
          ? fetchScreenshots(effectiveDate)
          : Promise.resolve([])
      ])
        .then(([screenshotSummaryResult, screenshotsResult]) => {
          if (!isCurrentRequest()) {
            return;
          }
          const latestContext = latestRefreshContext.current;
          const screenshotRowsStillAllowed =
            shouldLoadScreenshotRows &&
            latestContext.privacyMode === "raw" &&
            (latestContext.viewMode === "daily" ||
              latestContext.viewMode === "today") &&
            latestContext.layers.screenshots &&
            latestContext.date === effectiveDate;
          if (
            screenshotSummaryResult.status === "fulfilled" &&
            screenshotsResult.status === "fulfilled"
          ) {
            if (latestContext.date === effectiveDate) {
              setScreenshotSummary(screenshotSummaryResult.value);
              setScreenshots(
                screenshotRowsStillAllowed ? screenshotsResult.value : []
              );
              setScreenshotStatus("live");
            }
          } else {
            const reason =
              screenshotSummaryResult.status === "rejected"
                ? screenshotSummaryResult.reason
                : screenshotsResult.status === "rejected"
                  ? screenshotsResult.reason
                  : "unknown screenshot error";
            setScreenshotError(errorMessage(reason));
          }
        })
        .finally(() => {
          if (isCurrentRequest()) {
            setScreenshotLoading(false);
          }
        });

      void fetchVisualSummaries(effectiveDate)
        .then((summaries) => {
          if (isCurrentRequest() && isDateCurrent()) {
            setVisualSummaries(summaries);
          }
        })
        .catch((error) => {
          if (isCurrentRequest()) {
            setVisualSummaryError(errorMessage(error));
          }
        });
    } catch (error) {
      if (isCurrentRequest()) {
        setCollectorStatus("offline");
        setCollectorError(errorMessage(error));
        setActivityError(errorMessage(error));
        setInputError(errorMessage(error));
        setScreenshotError(errorMessage(error));
        setAnalysisError(errorMessage(error));
        setDailyBriefError(errorMessage(error));
        setVisualSummaryError(errorMessage(error));
        setActivityLoading(false);
        setInputLoading(false);
        setScreenshotLoading(false);
        setAnalysisLoading(false);
      }
    }
  }

  async function refreshAnalysisFeedback(
    requestGeneration = collectorRequestGeneration.current,
    date = latestRefreshContext.current.date
  ) {
    setAnalysisLoading(true);
    setAnalysisError(null);
    setDailyBriefError(null);
    const [statusResult, reportsResult, dailyBriefResult] = await Promise.allSettled([
      fetchAnalysisStatus(),
      fetchInsightReports({ date, kind: "5h", limit: 20 }),
      fetchDailyBrief(date)
    ]);
    if (requestGeneration !== collectorRequestGeneration.current) {
      return;
    }
    if (latestRefreshContext.current.date !== date) {
      return;
    }

    const reviewErrors: string[] = [];
    let dailyError: string | null = null;
    if (statusResult.status === "fulfilled") {
      setAnalysisStatus(statusResult.value);
    } else {
      reviewErrors.push(errorMessage(statusResult.reason));
    }

    if (reportsResult.status === "fulfilled") {
      setInsightReports(reportsResult.value);
    } else {
      reviewErrors.push(errorMessage(reportsResult.reason));
    }

    if (dailyBriefResult.status === "fulfilled") {
      setDailyBriefResponse(dailyBriefResult.value);
    } else {
      dailyError = errorMessage(dailyBriefResult.reason);
    }

    setAnalysisError(reviewErrors.length > 0 ? reviewErrors.join(" / ") : null);
    setDailyBriefError(dailyError);
    setAnalysisLoading(false);
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
              onChange={(event) => {
                const nextDate = event.currentTarget.value;
                setLatestRefreshContext({ date: nextDate });
                setQueryDate(nextDate);
              }}
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
          className={`tab ${viewMode === "today" ? "active" : ""}`}
          onClick={() => changeViewMode("today")}
        >
          <Activity aria-hidden="true" size={16} />
          <span>Today</span>
        </button>
        <button
          type="button"
          className={`tab ${viewMode === "activity" ? "active" : ""}`}
          onClick={() => changeViewMode("activity")}
        >
          <Activity aria-hidden="true" size={16} />
          <span>Activity Review</span>
        </button>
        <button
          type="button"
          className={`tab ${viewMode === "dashboard" ? "active" : ""}`}
          onClick={() => changeViewMode("dashboard")}
        >
          <Gauge aria-hidden="true" size={16} />
          <span>Dashboard</span>
        </button>
        <button
          type="button"
          className={`tab ${viewMode === "timeline" ? "active" : ""}`}
          onClick={() => changeViewMode("timeline")}
        >
          <BarChart3 aria-hidden="true" size={16} />
          <span>Timeline</span>
        </button>
        <button
          type="button"
          className={`tab ${viewMode === "daily" ? "active" : ""}`}
          onClick={() => changeViewMode("daily")}
        >
          <Camera aria-hidden="true" size={16} />
          <span>Daily Tracking</span>
        </button>
        <button
          type="button"
          className={`tab ${viewMode === "input" ? "active" : ""}`}
          onClick={() => changeViewMode("input")}
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
      {activityError && (
        <p className="sampleNotice" role="status">
          Activity layer unavailable. Keeping {activityStatus} activity data visible:{" "}
          {activityError}
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
      {analysisError && (
        <p className="sampleNotice" role="status">
          AI insight layer unavailable. Keeping current insight state visible:{" "}
          {analysisError}
        </p>
      )}
      {dailyBriefError && (
        <p className="sampleNotice" role="status">
          Daily Brief unavailable. Keeping current daily brief state visible:{" "}
          {dailyBriefError}
        </p>
      )}
      {visualSummaryError && (
        <p className="sampleNotice" role="status">
          Visual summary layer unavailable. Keeping current summary state visible:{" "}
          {visualSummaryError}
        </p>
      )}

      {viewMode === "today" ? (
        <>
          <InsightFeedback
            analysisStatus={analysisStatus}
            reports={insightReports}
            privacyMode={privacyMode}
            sourceMode={sourceMode}
            loading={analysisLoading}
            error={analysisError}
          />
          <DailyBriefPanel
            response={dailyBriefResponse}
            sourceMode={sourceMode}
            privacyMode={privacyMode}
            loading={analysisLoading}
            error={dailyBriefError}
          />
          <TodayFlowBoard
            events={events}
            screenshotSummary={screenshotSummary}
            screenshots={screenshots}
            inputSummary={inputSummary}
            health={health}
            privacyMode={privacyMode}
            screenshotsVisible={layers.screenshots}
            visualSummaries={visualSummaries}
            analyzingScreenshotId={analyzingScreenshotId}
            onAnalyzeScreenshot={(screenshotId) => {
              void handleAnalyzeScreenshot(screenshotId);
            }}
            sourceLabel={sourceMode === "live" ? "Live collector" : "Sample workspace"}
          />
        </>
      ) : viewMode === "activity" ? (
        <ActivityReview
          date={queryDate}
          buckets={activityBuckets}
          sourceMode={activityStatus}
          loading={activityLoading}
          error={activityError}
          privacyMode={privacyMode}
          visualSummaries={visualSummaries}
          onLoadSample={loadSample}
          onLoadLive={() => void refreshCollector()}
        />
  ) : viewMode === "daily" ? (
        <DailyTracking
          date={queryDate}
          screenshots={screenshots}
          summary={screenshotSummary}
          sourceMode={screenshotStatus}
          loading={screenshotLoading}
          error={screenshotError}
          screenshotsVisible={layers.screenshots}
          privacyMode={privacyMode}
          visualSummaries={visualSummaries}
          dailyBriefResponse={dailyBriefResponse}
          dailyBriefError={dailyBriefError}
          analyzingScreenshotId={analyzingScreenshotId}
          analysisError={visualSummaryError}
          onAnalyzeScreenshot={(screenshotId) => {
            void handleAnalyzeScreenshot(screenshotId);
          }}
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
          privacyMode={privacyMode}
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
      setLatestRefreshContext({ layers: next });
      if (layer === "screenshots" && !next.screenshots) {
        setScreenshots([]);
      }
      if (
        layer === "screenshots" &&
        next.screenshots &&
        sourceMode === "live" &&
        privacyMode === "raw" &&
        (latestRefreshContext.current.viewMode === "daily" ||
          latestRefreshContext.current.viewMode === "today")
      ) {
        void refreshCollector({
          layers: next,
          privacyMode,
          viewMode: latestRefreshContext.current.viewMode,
          date: queryDate
        });
      }
      return next;
    });
  }

  function changeViewMode(nextMode: ViewMode) {
    setLatestRefreshContext({ viewMode: nextMode });
    setViewMode(nextMode);
    if (sourceMode === "live" && privacyMode === "raw" && viewNeedsRawRows(nextMode, layers)) {
      void refreshCollector({
        privacyMode,
        layers,
        viewMode: nextMode,
        date: queryDate
      });
    }
  }

  function changePrivacyMode(nextMode: PrivacyMode) {
    setLatestRefreshContext({ privacyMode: nextMode });
    setPrivacyMode(nextMode);
    if (nextMode === "redacted") {
      if (sourceMode === "live") {
        setSegments([]);
        setScreenshots([]);
      }
      setInputLoading(false);
      setScreenshotLoading(false);
      return;
    }
    if (sourceMode === "live") {
      void refreshCollector({
        privacyMode: nextMode,
        viewMode: latestRefreshContext.current.viewMode,
        date: queryDate
      });
    }
  }

  async function handleAnalyzeScreenshot(screenshotId: number) {
    if (privacyMode !== "raw") {
      return;
    }
    setAnalyzingScreenshotId(screenshotId);
    setVisualSummaryError(null);
    try {
      const summary = await analyzeScreenshot(screenshotId);
      const latestContext = latestRefreshContext.current;
      if (latestContext.privacyMode !== "raw" || latestContext.date !== queryDate) {
        return;
      }
      setVisualSummaries((current) => [
        ...current.filter((item) => item.screenshotId !== screenshotId),
        summary
      ]);
      const refreshed = await fetchVisualSummaries(queryDate);
      if (
        latestRefreshContext.current.privacyMode === "raw" &&
        latestRefreshContext.current.date === queryDate
      ) {
        setVisualSummaries(refreshed);
      }
    } catch (error) {
      setVisualSummaryError(errorMessage(error));
    } finally {
      setAnalyzingScreenshotId(null);
    }
  }

  function setLatestRefreshContext(next: Partial<RefreshContext>) {
    latestRefreshContext.current = {
      ...latestRefreshContext.current,
      ...next
    };
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

function viewNeedsRawRows(viewMode: ViewMode, layers: LayerVisibility): boolean {
  return (
    viewMode === "input" ||
    ((viewMode === "daily" || viewMode === "today") && layers.screenshots)
  );
}

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
