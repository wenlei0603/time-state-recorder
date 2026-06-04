export type TimeEvent = {
  id: string;
  app: string;
  title: string;
  kind?: "active_window" | "lifecycle";
  status?: string;
  sessionId?: string;
  startedAt: string;
  endedAt?: string;
  durationSeconds?: number;
};

export type ActivityCategory =
  | "project_work"
  | "research"
  | "writing"
  | "coding"
  | "communication"
  | "meeting"
  | "admin"
  | "learning"
  | "planning"
  | "loafing"
  | "personal"
  | "idle"
  | "unknown";

export type AttentionState =
  | "deep_focus"
  | "steady"
  | "light_switching"
  | "fragmented"
  | "away"
  | "unknown";

export type BucketEvidence = {
  eventId: string;
  app: string;
  title: string;
  normalizedTitle: string;
  kind: "active_window" | "lifecycle";
  startedAt: string;
  endedAt: string;
  durationSeconds: number;
};

export type ActivityBucket = {
  id: string;
  startAt: string;
  endAt: string;
  bucketSeconds: number;
  dominantApp: string;
  dominantTitle: string;
  normalizedTitle: string;
  dominantDurationSeconds: number;
  switchCount: number;
  projectId?: string;
  projectName?: string;
  activityCategory: ActivityCategory;
  attentionState: AttentionState;
  confidence: number;
  evidence: BucketEvidence[];
  visualSummaryId?: number;
};

export type ActivityBucketsResponse = {
  date: string;
  bucketSeconds: number;
  buckets: ActivityBucket[];
};

export type PrivacyMode = "redacted" | "raw";

export type FlowConfidence = "high" | "partial" | "uncertain";

export type ScreenshotSkippedReasonCount = {
  reason: string;
  count: number;
};

export type FlowEvidence = {
  id: string;
  app: string;
  title: string;
  kind?: "active_window" | "lifecycle";
  status?: string;
  startedAt: string;
  endedAt?: string;
  durationSeconds: number;
  confidence: FlowConfidence;
  screenshotVisible: boolean;
};

export type FlowBucket = {
  id: string;
  app: string;
  title: string;
  kind?: "active_window" | "lifecycle";
  status?: string;
  startedAt: string;
  endedAt?: string;
  durationSeconds: number;
  confidence: FlowConfidence;
  evidence: FlowEvidence[];
};

export type TodayFlowModel = {
  privacyMode: PrivacyMode;
  activeSeconds: number;
  uncertainSeconds: number;
  screenshotCount: number;
  screenshotSkippedCount: number;
  inputChars: number;
  skippedReasons: ScreenshotSkippedReasonCount[];
  buckets: FlowBucket[];
  evidence: FlowEvidence[];
};

export type DurationSummary = {
  count: number;
  total: number;
  mean: number;
  median: number;
  min: number;
  max: number;
  q1: number;
  q3: number;
  standardDeviation: number;
};

export type ApplicationSummary = {
  app: string;
  eventCount: number;
  totalSeconds: number;
  averageSeconds: number;
  share: number;
};

export type ScreenshotMeta = {
  id: number;
  capturedAt: string;
  filePath: string;
  width: number;
  height: number;
  processName?: string;
  windowTitle?: string;
  captureStatus: string;
};

export type ScreenshotSummary = {
  date: string;
  totalScreenshots: number;
  hoursCovered: number;
  topApps: AppScreenshotCount[];
  skippedReasons?: ScreenshotSkippedReasonCount[];
};

export type VisualSummary = {
  id: number;
  screenshotId: number;
  capturedAt: string;
  modelProvider: string;
  modelName: string;
  promptVersion: string;
  summaryText: string;
  activityCategory: ActivityCategory;
  projectHints: string[];
  visibleApps: string[];
  visibleTextHints: string[];
  riskFlags: string[];
  confidence: number;
  createdAt: string;
  error?: string;
};

export type VisualObservation = {
  id: number;
  highResScreenshotId: number;
  capturedAt: string;
  filePath: string;
  modelProvider: string;
  modelName: string;
  promptVersion: string;
  summaryText: string;
  activityCategory: ActivityCategory;
  projectHints: string[];
  visibleApps: string[];
  visibleTextHints: string[];
  riskFlags: string[];
  confidence: number;
  createdAt: string;
  error?: string;
};

export type VisualTrajectoryPoint = {
  minuteMark: number;
  screenshotId: number;
  observation: string;
  activityCategory: ActivityCategory;
};

export type VisualWindowSummary = {
  id: number;
  windowStart: string;
  windowEnd: string;
  sampledScreenshotIds: number[];
  previousSummaryId?: number;
  modelProvider: string;
  modelName: string;
  promptVersion: string;
  summaryText: string;
  continuity: string;
  primaryActivity: ActivityCategory;
  projectHints: string[];
  taskIntent: string;
  trajectory: VisualTrajectoryPoint[];
  switchingLevel: string;
  switchingEvidence: string;
  loafingLevel: string;
  loafingEvidence: string;
  visibleApps: string[];
  visibleTextHints: string[];
  riskFlags: string[];
  confidence: number;
  rawSummaryJson: unknown;
  createdAt: string;
  error?: string;
};

export type ActivityCategoryCount = {
  activityCategory: ActivityCategory;
  count: number;
};

export type InsightReport = {
  id: number;
  periodStart: string;
  periodEnd: string;
  generatedAt: string;
  reportKind: string;
  modelProvider: string;
  modelName: string;
  summaryText: string;
  categoryMix: ActivityCategoryCount[];
  projectHints: string[];
  evidenceCount: number;
  error?: string;
};

export type DailyAppActivity = {
  processName: string;
  activeSeconds: number;
  share: number;
};

export type DailyActivityStats = {
  date: string;
  periodStart: string;
  periodEnd: string;
  activeSeconds: number;
  activeHours: number;
  windowEventCount: number;
  switchCount: number;
  distinctAppCount: number;
  topApps: DailyAppActivity[];
  categoryMix: ActivityCategoryCount[];
  inputChars: number;
  inputEvents: number;
  screenshotCount: number;
  highResScreenshotCount: number;
  visualWindowCount: number;
  fiveHourReportCount: number;
  firstActivityAt?: string;
  lastActivityAt?: string;
};

export type HourlyActivityMetric = {
  hour: number;
  startAt: string;
  endAt: string;
  activeSeconds: number;
  activeRatio: number;
  windowEventCount: number;
  switchCount: number;
  distinctAppCount: number;
  dominantApp?: string;
  dominantCategory: ActivityCategory;
  inputChars: number;
  screenshotCount: number;
  highResScreenshotCount: number;
  visualWindowCount: number;
  fiveHourReportIds: number[];
};

export type DailyComparison = {
  baselineDays: number;
  comparedDates: string[];
  activeSecondsDelta: number;
  switchesPerHourDelta: number;
  inputCharsDelta: number;
  screenshotCoverageDelta: number;
  dominantCategoryShift?: string;
  startTimeShiftMinutes?: number;
  endTimeShiftMinutes?: number;
  explanation: string;
};

export type DailyBrief = {
  id: number;
  date: string;
  periodStart: string;
  periodEnd: string;
  generatedAt: string;
  scheduledForLocal: string;
  modelProvider: string;
  modelName: string;
  promptVersion: string;
  status: string;
  descriptiveStats: DailyActivityStats;
  hourlyMetrics: HourlyActivityMetric[];
  comparison: DailyComparison;
  fiveHourReportIds: number[];
  dailySummaryText: string;
  actionTrajectory: string;
  rawSummaryJson: unknown;
  error?: string;
};

export type DailyBriefResponse = {
  date: string;
  status: "missing" | "pending" | "running" | "complete" | "error" | string;
  nextRunAt?: string;
  brief?: DailyBrief;
  fiveHourReports: InsightReport[];
  descriptiveStats: DailyActivityStats;
  hourlyMetrics: HourlyActivityMetric[];
  comparison: DailyComparison;
};

export type AnalysisWorkerStatus = {
  status: "idle" | "running" | "error" | string;
  lastStartedAt?: string;
  lastFinishedAt?: string;
  nextRunAt?: string;
  lastError?: string;
};

export type AnalysisStatus = {
  visual: AnalysisWorkerStatus;
  report: AnalysisWorkerStatus;
  daily?: AnalysisWorkerStatus;
  latestObservation?: VisualObservation;
  latestWindowSummary?: VisualWindowSummary;
  latestReport?: InsightReport;
  latestDailyBrief?: DailyBrief;
};

export type AppScreenshotCount = {
  processName: string;
  count: number;
};

export type InputEvent = {
  id: number;
  eventTs: string;
  eventType: "keydown" | "keyup";
  vkCode: number;
  scanCode: number;
  character?: string;
  segmentId: string;
  foregroundHwnd: number;
  foregroundPid: number;
  processName?: string;
  windowTitle?: string;
};

export type TextSegment = {
  id: string;
  startedAt: string;
  endedAt?: string;
  textContent: string;
  keyCount: number;
  backspaceCount: number;
  deleteCount: number;
  foregroundHwnd: number;
  foregroundPid: number;
  processName?: string;
  windowTitle?: string;
};

export type InputSummary = {
  date: string;
  totalEvents: number;
  keydownCount: number;
  keyupCount: number;
  segmentCount: number;
  totalChars: number;
  lastActivity?: string;
  topApps: AppInputCount[];
};

export type AppInputCount = {
  processName: string;
  charCount: number;
};

export type SubsystemHealth = {
  status: "running" | "error" | "not_started";
  lastEventAt?: string;
  errorCount: number;
  lastError?: string;
  mode?: string;
  lastCaptureStatus?: string;
  lastSkipReason?: string;
};

export type DbStats = {
  windowEvents: number;
  lifecycleEvents: number;
  inputEvents: number;
  textSegments: number;
  screenshots: number;
  highResScreenshots: number;
  blockerHits: number;
  imageRetention: ImageRetentionStats;
};

export type ImageRetentionStats = {
  retentionDays: number;
  activeFiles: number;
  expiredFiles: number;
  activeBytes: number;
  expiredBytes: number;
  pendingGoogleDriveUpload: boolean;
  googleDriveMessage?: string;
};

export type CollectorHealth = {
  status: "ok" | "degraded" | "error";
  startedAt: string;
  uptimeSeconds: number;
  version: string;
  windowCollector: SubsystemHealth;
  inputCollector: SubsystemHealth;
  screenshotCollector: SubsystemHealth;
  dbStats: DbStats;
};
