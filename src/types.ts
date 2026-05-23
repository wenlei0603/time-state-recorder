export type TimeEvent = {
  id: string;
  app: string;
  title: string;
  startedAt: string;
  endedAt?: string;
  durationSeconds?: number;
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
};

export type AppScreenshotCount = {
  processName: string;
  count: number;
};
