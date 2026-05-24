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
  blockerHits: number;
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
