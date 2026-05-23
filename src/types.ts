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
